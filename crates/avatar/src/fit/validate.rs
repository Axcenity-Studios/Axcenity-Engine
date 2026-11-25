//! Read-only Canonical Source Validator (ADR-0046).
//!
//! Replaces the destructive 11-step intake (`intake::prepare`) with a pre-fit
//! inspection gate. A canonical garment source must be a single-layer,
//! manifold mesh in the body's 45-degree A-pose with clean boundary loops and
//! opening edge lengths satisfying the chord-sag bound $L < \sqrt{8 r m}$.
//!
//! `validate_garment` never mutates geometry: it reports topological and
//! chord-sag metrics so broken upstream assets fail fast with actionable
//! diagnostics.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::GarmentMesh;

/// Minimum limb curvature radius (metres) used for opening chord-sag budgeting
/// (wrist / ankle radius ≈ 2.5 cm).
pub const MIN_LIMB_RADIUS_M: f32 = 0.025;

/// Read-only validation report for a canonical garment source before fitting.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Edges shared by more than two triangles (welded topology).
    pub non_manifold_edges: usize,
    /// Zero-area or collapsed triangles.
    pub degenerate_triangles: usize,
    /// Number of connected open boundary edge loops (e.g., neck + 2 cuffs + hem = 4).
    pub boundary_loops: usize,
    /// Total open boundary edges.
    pub boundary_edges: usize,
    /// Longest edge along an open boundary loop (metres).
    pub max_opening_edge_m: f32,
    /// Chord-sag edge length limit $\sqrt{8 r m}$ for the requested margin (metres).
    pub chord_sag_limit_m: f32,
    /// Number of open boundary edges exceeding `chord_sag_limit_m`.
    pub chord_sag_violations: usize,
    /// Actionable diagnostics for artists or upstream prep scripts.
    pub warnings: Vec<String>,
}

impl ValidationReport {
    /// True when the mesh has zero non-manifold edges, zero degenerate
    /// triangles, and zero chord-sag violations at openings.
    pub fn is_clean(&self) -> bool {
        self.non_manifold_edges == 0
            && self.degenerate_triangles == 0
            && self.chord_sag_violations == 0
    }
}

/// Validates a canonical garment mesh against manifoldness, boundary loop
/// sanity, and opening chord-sag budget $L < \sqrt{8 r m}$ for `margin`.
pub fn validate_garment(garment: &GarmentMesh, margin: f32) -> ValidationReport {
    let positions = garment.positions();
    let triangles = garment.triangles();
    let chord_sag_limit_m = (8.0 * MIN_LIMB_RADIUS_M * margin.max(1e-4)).sqrt();

    // Weld vertices within 1e-5 m so UV seams do not masquerade as open edges.
    let mut place_map: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    let mut welded_pos = Vec::new();
    let of_corner: Vec<u32> = positions
        .iter()
        .map(|&p| {
            let key = (p * 1e5).round().as_i64vec3().to_array();
            *place_map.entry(key).or_insert_with(|| {
                let id = welded_pos.len() as u32;
                welded_pos.push(p);
                id
            })
        })
        .collect();

    let mut edge_uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    let mut degenerate_triangles = 0usize;
    for t in &triangles {
        let [a, b, c] = t.map(|v| of_corner[v as usize]);
        if a == b || b == c || a == c {
            degenerate_triangles += 1;
            continue;
        }
        let area2 = (welded_pos[b as usize] - welded_pos[a as usize])
            .cross(welded_pos[c as usize] - welded_pos[a as usize])
            .length_squared();
        if area2 <= 1e-16 {
            degenerate_triangles += 1;
            continue;
        }
        for (u, v) in [(a, b), (b, c), (c, a)] {
            *edge_uses.entry((u.min(v), u.max(v))).or_default() += 1;
        }
    }

    let mut non_manifold_edges = 0usize;
    let mut open_edges: Vec<(u32, u32)> = Vec::new();
    let mut max_opening_edge_m = 0.0f32;
    let mut chord_sag_violations = 0usize;

    for (&(u, v), &count) in &edge_uses {
        if count > 2 {
            non_manifold_edges += 1;
        } else if count == 1 {
            open_edges.push((u, v));
            let len = welded_pos[u as usize].distance(welded_pos[v as usize]);
            max_opening_edge_m = max_opening_edge_m.max(len);
            if len > chord_sag_limit_m {
                chord_sag_violations += 1;
            }
        }
    }

    // Count connected open boundary loops via union-find.
    let mut parent: BTreeMap<u32, u32> = BTreeMap::new();
    fn find(parent: &mut BTreeMap<u32, u32>, mut x: u32) -> u32 {
        loop {
            let p = *parent.entry(x).or_insert(x);
            if p == x {
                return x;
            }
            x = p;
        }
    }
    for &(u, v) in &open_edges {
        let ru = find(&mut parent, u);
        let rv = find(&mut parent, v);
        if ru != rv {
            parent.insert(ru, rv);
        }
    }
    let mut roots = std::collections::BTreeSet::new();
    for &(u, _) in &open_edges {
        roots.insert(find(&mut parent, u));
    }
    let boundary_loops = roots.len();

    let mut warnings = Vec::new();
    if non_manifold_edges > 0 {
        warnings.push(format!(
            "{non_manifold_edges} non-manifold edges detected (run upstream manifold prep)"
        ));
    }
    if degenerate_triangles > 0 {
        warnings.push(format!(
            "{degenerate_triangles} degenerate/zero-area triangles detected"
        ));
    }
    if boundary_loops == 0 {
        warnings.push(
            "0 open boundary loops: mesh appears to be a closed shell (extract outer single-layer shell upstream)"
                .to_owned(),
        );
    }
    if chord_sag_violations > 0 {
        warnings.push(format!(
            "{chord_sag_violations} opening boundary edges exceed chord-sag limit {:.1} mm (max {:.1} mm)",
            chord_sag_limit_m * 1000.0,
            max_opening_edge_m * 1000.0
        ));
    }

    ValidationReport {
        non_manifold_edges,
        degenerate_triangles,
        boundary_loops,
        boundary_edges: open_edges.len(),
        max_opening_edge_m,
        chord_sag_limit_m,
        chord_sag_violations,
        warnings,
    }
}
