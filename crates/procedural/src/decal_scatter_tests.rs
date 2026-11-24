//! The decal brush's layout: where a stamp lays its pictures, how they're
//! turned and sized, its spacing and its budget.

use glam::{Vec2, Vec3};

use super::*;
use crate::rotation_from_orientation;

fn item(texture: &str) -> DecalItem {
    DecalItem {
        texture: texture.to_owned(),
        weight: 1.0,
        min_size: 2.0,
        max_size: 5.0,
        aspect: 1.0,
        min_opacity: 0.4,
        max_opacity: 0.8,
        rotation: 360.0,
    }
}

fn brush(items: Vec<DecalItem>) -> DecalBrush {
    DecalBrush {
        items,
        radius: 20.0,
        falloff: 0.0,
        // 5 per 100 square studs: about 63 in a 20-stud circle.
        density: 0.05,
        spacing: 0.0,
        seed: 9,
    }
}

/// Room for any number.
const NO_LIMIT: DecalBudget = DecalBudget {
    count: usize::MAX,
    distance: 150.0,
};

/// Flat ground at Y = 3.
fn flat(spot: Vec2) -> Option<(Vec3, Vec3)> {
    Some((Vec3::new(spot.x, 3.0, spot.y), Vec3::Y))
}

#[test]
fn a_stamp_lays_its_pictures_on_the_surface_inside_the_brush_the_same_for_a_seed() {
    let brush = brush(vec![
        item("stain.png"),
        DecalItem {
            weight: 3.0,
            aspect: 0.25,
            rotation: 0.0,
            ..item("tracks.png")
        },
        DecalItem {
            weight: 0.0,
            ..item("off.png")
        },
    ]);
    let center = Vec3::new(10.0, 3.0, -4.0);
    let stamped = stamp(&brush, center, &mut Vec::new(), NO_LIMIT, flat);
    let placed = &stamped.placed;
    assert!((45..=64).contains(&placed.len()), "{}", placed.len());
    assert_eq!(stamped.over_budget, 0);
    let tracks = placed.iter().filter(|d| d.texture == "tracks.png").count();
    assert!(placed.iter().all(|d| d.texture != "off.png"));
    let share = tracks as f32 / placed.len() as f32;
    assert!((share - 0.75).abs() < 0.15, "tracks are {share} of the mix");
    for decal in placed {
        let flat_away = Vec2::new(decal.position.x - center.x, decal.position.z - center.z);
        assert!(flat_away.length() <= 20.0);
        assert_eq!(decal.position.y, 3.0);
        // Its front faces up, out of the ground.
        let rotation = rotation_from_orientation(Vec3::new(
            decal.orientation.x,
            decal.orientation.y,
            decal.orientation.z,
        ));
        assert!((rotation * Vec3::NEG_Z).distance(Vec3::Y) < 1e-3);
        let longer = decal.size.x.max(decal.size.y);
        assert!((2.0..=5.0).contains(&longer), "{longer}");
        assert!((0.2..=0.6 + 1e-6).contains(&decal.transparency));
        assert_eq!(
            decal.size.z,
            (longer * DEPTH_SHARE).clamp(MIN_DEPTH, MAX_DEPTH)
        );
        if decal.texture == "tracks.png" {
            // Four times as long as wide, and every copy laid the same way.
            assert!((decal.size.x * 4.0 - decal.size.y).abs() < 1e-4);
            assert_eq!(decal.orientation, lying_on(Vec3::Y, 0.0));
        } else {
            assert_eq!(decal.size.x, decal.size.y);
        }
    }
    // Sizes, opacities and turns differ from one to the next.
    let stains: Vec<_> = placed.iter().filter(|d| d.texture == "stain.png").collect();
    assert!(stains.windows(2).any(|pair| pair[0].size != pair[1].size));
    assert!(
        stains
            .windows(2)
            .any(|pair| pair[0].orientation != pair[1].orientation)
    );
    assert!(
        stains
            .windows(2)
            .any(|pair| pair[0].transparency != pair[1].transparency)
    );
    // A new one lies as a ProjectedDecal does by default.
    assert_eq!(lying_on(Vec3::Y, 0.0), Vector3::new(90.0, 0.0, 0.0));

    // The same seed and middle: the same decals. Another seed: others.
    let again = stamp(&brush, center, &mut Vec::new(), NO_LIMIT, flat);
    assert_eq!(again, stamped);
    let other = DecalBrush {
        seed: 10,
        ..brush.clone()
    };
    assert_ne!(
        stamp(&other, center, &mut Vec::new(), NO_LIMIT, flat),
        stamped
    );
    // Nothing to lie on, no pictures or no density: nothing placed.
    assert!(
        stamp(&brush, center, &mut Vec::new(), NO_LIMIT, |_| None)
            .placed
            .is_empty()
    );
    let none = DecalBrush {
        density: 0.0,
        ..brush.clone()
    };
    assert!(
        stamp(&none, center, &mut Vec::new(), NO_LIMIT, flat)
            .placed
            .is_empty()
    );
    let empty = DecalBrush {
        items: Vec::new(),
        ..brush
    };
    assert!(
        stamp(&empty, center, &mut Vec::new(), NO_LIMIT, flat)
            .placed
            .is_empty()
    );
}

#[test]
fn a_decal_on_a_slope_faces_the_slope_and_a_soft_rim_thins_the_edge() {
    // Ground climbing 30 degrees along X.
    let normal = Vec3::new(-0.5, 0.866_025_4, 0.0);
    let slope = |spot: Vec2| Some((Vec3::new(spot.x, spot.x * 0.577_350_3, spot.y), normal));
    let hard = brush(vec![item("stain.png")]);
    let stamped = stamp(&hard, Vec3::ZERO, &mut Vec::new(), NO_LIMIT, slope);
    assert!(!stamped.placed.is_empty());
    for decal in &stamped.placed {
        let rotation = rotation_from_orientation(Vec3::new(
            decal.orientation.x,
            decal.orientation.y,
            decal.orientation.z,
        ));
        assert!((rotation * Vec3::NEG_Z).distance(normal) < 2e-3);
        assert!((decal.position.y - decal.position.x * 0.577_350_3).abs() < 1e-4);
    }
    // With all of the radius fading, the outer half of the radius holds
    // about half of them, not its three quarters of the area.
    let soft = DecalBrush {
        falloff: 1.0,
        density: 0.1,
        ..hard
    };
    let stamped = stamp(&soft, Vec3::ZERO, &mut Vec::new(), NO_LIMIT, flat);
    let outer = stamped
        .placed
        .iter()
        .filter(|d| Vec2::new(d.position.x, d.position.z).length() > 10.0)
        .count();
    assert!(stamped.placed.len() > 20, "{}", stamped.placed.len());
    assert!(
        outer * 100 < stamped.placed.len() * 64,
        "{outer} of {}",
        stamped.placed.len()
    );
}

#[test]
fn decals_keep_their_spacing_from_each_other_and_from_those_already_there() {
    let brush = DecalBrush {
        spacing: 4.0,
        density: 0.3,
        ..brush(vec![item("stain.png")])
    };
    let mut existing = vec![Vec3::new(0.0, 3.0, 0.0)];
    let first = stamp(
        &brush,
        Vec3::new(0.0, 3.0, 0.0),
        &mut existing,
        NO_LIMIT,
        flat,
    );
    assert!(first.placed.len() > 15, "{}", first.placed.len());
    assert_eq!(existing.len(), first.placed.len() + 1);
    for (index, a) in existing.iter().enumerate() {
        for b in &existing[index + 1..] {
            assert!(a.distance(*b) >= 4.0, "{a} and {b}");
        }
    }
    // Stamping again nearby fills gaps, and never closer than the spacing.
    let second = stamp(
        &brush,
        Vec3::new(3.0, 3.0, 1.0),
        &mut existing,
        NO_LIMIT,
        flat,
    );
    assert!(second.placed.len() < first.placed.len());
    for (index, a) in existing.iter().enumerate() {
        for b in &existing[index + 1..] {
            assert!(a.distance(*b) >= 4.0, "{a} and {b}");
        }
    }
}

#[test]
fn a_stamp_stops_at_its_budget_of_decals_within_the_budgets_distance() {
    let brush = brush(vec![item("stain.png")]);
    // Medium quality's: the nearest 128 within 150 studs are drawn.
    let medium = DecalBudget {
        count: 128,
        distance: 150.0,
    };
    // 120 already lie round the stamp, and 500 more a long way off.
    let mut existing: Vec<Vec3> = (0..120)
        .map(|n| Vec3::new(40.0 + n as f32 * 0.5, 3.0, 0.0))
        .chain((0..500).map(|n| Vec3::new(1000.0 + n as f32, 3.0, 0.0)))
        .collect();
    let stamped = stamp(
        &brush,
        Vec3::new(0.0, 3.0, 0.0),
        &mut existing,
        medium,
        flat,
    );
    // Room for 8 more here, however many the brush wanted; the far ones
    // don't count.
    assert_eq!(stamped.placed.len(), 8);
    assert!(stamped.over_budget > 30, "{}", stamped.over_budget);
    let near = existing
        .iter()
        .filter(|at| at.distance(Vec3::new(0.0, 3.0, 0.0)) <= 150.0)
        .count();
    assert!(near <= 128 + 8, "{near}");
    // Full: nothing more.
    let full = stamp(
        &brush,
        Vec3::new(1.0, 3.0, 0.0),
        &mut existing,
        medium,
        flat,
    );
    assert!(full.placed.is_empty());
    assert!(full.over_budget > 0);
    // Away from them all there's room again.
    let away = stamp(
        &brush,
        Vec3::new(0.0, 3.0, 400.0),
        &mut existing,
        medium,
        flat,
    );
    assert!(away.placed.len() > 40);
    assert_eq!(away.over_budget, 0);
    // No stamp adds more than its own limit, whatever the density.
    let dense = DecalBrush {
        density: 5.0,
        ..brush
    };
    let many = stamp(
        &dense,
        Vec3::new(0.0, 3.0, 2000.0),
        &mut Vec::new(),
        NO_LIMIT,
        flat,
    );
    assert_eq!(many.placed.len(), MAX_DECALS_PER_STAMP);
}
