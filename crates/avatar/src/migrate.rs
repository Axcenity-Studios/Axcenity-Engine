//! Format 1 to format 2. Format 1 is Character Studio CS2's `Look`
//! (`editor::character_workspace`): a name, `build` (-1 to 1), `muscle`
//! (0 to 1) and a part name per slot (`hair`, `top`, `outerwear`,
//! `bottom`, `shoes`; `null` for none), always on the stylized humanoid.
//! It read missing values as the default character's, brought values in
//! range and ignored part names the parts file doesn't have; migration
//! does the same, so a migrated file is the character format 1 showed.

use serde::Deserialize;

use crate::{Description, FormatError, WornItem};

/// The base model every format 1 character was on: the stylized humanoid,
/// which the engine shipped until 2026-11-25. A project made before then
/// has the package in its own library, so its characters still resolve.
pub const V1_BASE: &str = "axcenity/stylized";

/// The base model the engine ships and a new character is on: the human
/// base (MakeHuman's body, ADR-0047).
pub const DEFAULT_BASE: &str = "axcenity/human";

/// Format 1's part names (objects of `HumanoidParts.glb`) and the items
/// they became.
pub const V1_ITEMS: [(&str, &str); 12] = [
    ("Hair_Swept", "axcenity/hair-swept"),
    ("Hair_Bob", "axcenity/hair-bob"),
    ("Hair_Long", "axcenity/hair-long"),
    ("Hair_Crop", "axcenity/hair-crop"),
    ("Hair_Buzz", "axcenity/hair-buzz"),
    ("Hair_Ponytail", "axcenity/hair-ponytail"),
    ("Shirt_Tee", "axcenity/tee"),
    ("Jacket_Cropped", "axcenity/cropped-jacket"),
    ("Pants_Cargo", "axcenity/cargo-pants"),
    ("Shoes_Sneakers", "axcenity/sneakers"),
    ("Underwear_Briefs", "axcenity/briefs"),
    ("Underwear_Top", "axcenity/sports-top"),
];

/// A format 1 file as format 1 read it: missing values are the default
/// character's.
#[derive(Deserialize)]
#[serde(default)]
struct LookV1 {
    name: String,
    build: f32,
    muscle: f32,
    hair: Option<String>,
    top: Option<String>,
    outerwear: Option<String>,
    bottom: Option<String>,
    shoes: Option<String>,
}

impl Default for LookV1 {
    fn default() -> Self {
        let some = |part: &str| Some(part.to_owned());
        Self {
            name: "New Character".to_owned(),
            build: 0.0,
            muscle: 0.0,
            hair: some("Hair_Swept"),
            top: some("Shirt_Tee"),
            outerwear: some("Jacket_Cropped"),
            bottom: some("Pants_Cargo"),
            shoes: some("Shoes_Sneakers"),
        }
    }
}

pub(crate) fn from_v1(value: serde_json::Value) -> Result<Description, FormatError> {
    let look: LookV1 =
        serde_json::from_value(value).map_err(|err| FormatError::Damaged(err.to_string()))?;
    let name = if look.name.trim().is_empty() {
        "New Character".to_owned()
    } else {
        look.name
    };
    let mut description = Description::new(name, V1_BASE);
    // Format 1's ranges: build -1 to 1, muscle 0 to 1 (a value that isn't a
    // number can't reach here: JSON has none).
    description
        .parameters
        .insert("build".to_owned(), look.build.clamp(-1.0, 1.0));
    description
        .parameters
        .insert("muscle".to_owned(), look.muscle.clamp(0.0, 1.0));
    let worn = [look.hair, look.top, look.outerwear, look.bottom, look.shoes];
    for part in worn.iter().flatten() {
        if let Some((_, id)) = V1_ITEMS.iter().find(|(name, _)| name == part) {
            description.items.push(WornItem::new(*id));
        }
    }
    Ok(description)
}
