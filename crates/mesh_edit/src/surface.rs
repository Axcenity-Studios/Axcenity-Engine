//! Object-level physical surface properties and decal/sound presets for
//! Model Studio (`ModelDoc`), inspired by Source 2's ModelDoc.
//!
//! Picking a [`SurfacePreset`] (e.g. `Wood_Plank`, `Metal_Hollow`, `Concrete`,
//! `Flesh`) automatically provides density (used to compute mass from the
//! mesh's bounding-box volume), friction, bounciness (restitution), and default
//! impact decal and sound sets, while allowing per-property manual overrides.

use serde::{Deserialize, Serialize};

/// Built-in physical surface presets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfacePreset {
    #[default]
    Default,
    WoodPlank,
    MetalHollow,
    MetalSolid,
    Concrete,
    Flesh,
    Plastic,
    Glass,
    Rubber,
    Stone,
    Fabric,
}

impl SurfacePreset {
    pub const ALL: [SurfacePreset; 11] = [
        SurfacePreset::Default,
        SurfacePreset::WoodPlank,
        SurfacePreset::MetalHollow,
        SurfacePreset::MetalSolid,
        SurfacePreset::Concrete,
        SurfacePreset::Flesh,
        SurfacePreset::Plastic,
        SurfacePreset::Glass,
        SurfacePreset::Rubber,
        SurfacePreset::Stone,
        SurfacePreset::Fabric,
    ];

    /// Display name matching Source 2 / ModelDoc conventions.
    pub fn label(self) -> &'static str {
        match self {
            SurfacePreset::Default => "Default",
            SurfacePreset::WoodPlank => "Wood_Plank",
            SurfacePreset::MetalHollow => "Metal_Hollow",
            SurfacePreset::MetalSolid => "Metal_Solid",
            SurfacePreset::Concrete => "Concrete",
            SurfacePreset::Flesh => "Flesh",
            SurfacePreset::Plastic => "Plastic",
            SurfacePreset::Glass => "Glass",
            SurfacePreset::Rubber => "Rubber",
            SurfacePreset::Stone => "Stone",
            SurfacePreset::Fabric => "Fabric",
        }
    }

    /// Density in kg/m³ (used with bounding-box volume to compute mass).
    pub fn density(self) -> f32 {
        match self {
            SurfacePreset::Default => 1000.0,
            SurfacePreset::WoodPlank => 650.0,
            SurfacePreset::MetalHollow => 1800.0,
            SurfacePreset::MetalSolid => 7850.0,
            SurfacePreset::Concrete => 2400.0,
            SurfacePreset::Flesh => 1050.0,
            SurfacePreset::Plastic => 950.0,
            SurfacePreset::Glass => 2500.0,
            SurfacePreset::Rubber => 1200.0,
            SurfacePreset::Stone => 2600.0,
            SurfacePreset::Fabric => 350.0,
        }
    }

    /// Default Coulomb friction coefficient (0.0 to 1.0+).
    pub fn friction(self) -> f32 {
        match self {
            SurfacePreset::Default => 0.5,
            SurfacePreset::WoodPlank => 0.55,
            SurfacePreset::MetalHollow => 0.35,
            SurfacePreset::MetalSolid => 0.4,
            SurfacePreset::Concrete => 0.75,
            SurfacePreset::Flesh => 0.65,
            SurfacePreset::Plastic => 0.4,
            SurfacePreset::Glass => 0.2,
            SurfacePreset::Rubber => 0.9,
            SurfacePreset::Stone => 0.7,
            SurfacePreset::Fabric => 0.8,
        }
    }

    /// Default restitution / bounciness (0.0 to 1.0).
    pub fn bounciness(self) -> f32 {
        match self {
            SurfacePreset::Default => 0.1,
            SurfacePreset::WoodPlank => 0.15,
            SurfacePreset::MetalHollow => 0.25,
            SurfacePreset::MetalSolid => 0.1,
            SurfacePreset::Concrete => 0.05,
            SurfacePreset::Flesh => 0.1,
            SurfacePreset::Plastic => 0.2,
            SurfacePreset::Glass => 0.15,
            SurfacePreset::Rubber => 0.75,
            SurfacePreset::Stone => 0.08,
            SurfacePreset::Fabric => 0.05,
        }
    }

    /// Default impact decal set identifier.
    pub fn decal_set(self) -> &'static str {
        match self {
            SurfacePreset::Default => "decals/impact_generic",
            SurfacePreset::WoodPlank => "decals/impact_wood",
            SurfacePreset::MetalHollow | SurfacePreset::MetalSolid => "decals/impact_metal",
            SurfacePreset::Concrete => "decals/impact_concrete",
            SurfacePreset::Flesh => "decals/impact_flesh",
            SurfacePreset::Plastic => "decals/impact_plastic",
            SurfacePreset::Glass => "decals/impact_glass",
            SurfacePreset::Rubber => "decals/impact_rubber",
            SurfacePreset::Stone => "decals/impact_stone",
            SurfacePreset::Fabric => "decals/impact_fabric",
        }
    }

    /// Default impact sound set identifier.
    pub fn sound_set(self) -> &'static str {
        match self {
            SurfacePreset::Default => "sounds/impact_generic",
            SurfacePreset::WoodPlank => "sounds/impact_wood_plank",
            SurfacePreset::MetalHollow => "sounds/impact_metal_hollow",
            SurfacePreset::MetalSolid => "sounds/impact_metal_solid",
            SurfacePreset::Concrete => "sounds/impact_concrete",
            SurfacePreset::Flesh => "sounds/impact_flesh",
            SurfacePreset::Plastic => "sounds/impact_plastic",
            SurfacePreset::Glass => "sounds/impact_glass",
            SurfacePreset::Rubber => "sounds/impact_rubber",
            SurfacePreset::Stone => "sounds/impact_stone",
            SurfacePreset::Fabric => "sounds/impact_fabric",
        }
    }
}

/// Physical surface properties, auto-calculated mass, and impact decal/sound
/// links for a [`crate::DocObject`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceProperties {
    #[serde(default)]
    pub preset: SurfacePreset,
    /// Optional manual mass override in kg. When `None`, mass is computed as
    /// `bounding_box_volume * preset.density()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mass_override: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friction_override: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounciness_override: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decal_set_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound_set_override: Option<String>,
}

impl SurfaceProperties {
    pub fn is_default(&self) -> bool {
        self.preset == SurfacePreset::Default
            && self.mass_override.is_none()
            && self.friction_override.is_none()
            && self.bounciness_override.is_none()
            && self.decal_set_override.is_none()
            && self.sound_set_override.is_none()
    }

    /// Mass in kg computed from bounding-box volume (in cubic meters / studs³)
    /// times the preset's density.
    pub fn computed_mass(&self, bbox_volume: f32) -> f32 {
        (bbox_volume.max(1e-4) * self.preset.density()).max(0.01)
    }

    /// Effective mass in kg (manual override if set, otherwise computed).
    pub fn effective_mass(&self, bbox_volume: f32) -> f32 {
        self.mass_override
            .map(|m| m.max(0.01))
            .unwrap_or_else(|| self.computed_mass(bbox_volume))
    }

    pub fn effective_friction(&self) -> f32 {
        self.friction_override
            .unwrap_or_else(|| self.preset.friction())
            .clamp(0.0, 2.0)
    }

    pub fn effective_bounciness(&self) -> f32 {
        self.bounciness_override
            .unwrap_or_else(|| self.preset.bounciness())
            .clamp(0.0, 1.0)
    }

    pub fn effective_decal_set(&self) -> &str {
        self.decal_set_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.preset.decal_set())
    }

    pub fn effective_sound_set(&self) -> &str {
        self.sound_set_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.preset.sound_set())
    }
}
