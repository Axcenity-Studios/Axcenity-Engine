//! Character descriptions for scripts (Character Studio overhaul CO3): a
//! HumanoidDescription's parameters, wardrobe and material values, checked
//! against the repository's character library (`assets/characters/`).
//! Applying one to a character is tested where there is a character: the
//! player's `default_humanoid_tests/description_lua.rs`.

use std::path::{Path, PathBuf};

use axcenity_assets::Assets;
use axcenity_avatar::{
    Colour, DEFAULT_BASE, Description, Library, MaterialValue, Variant, WornItem,
};
use axcenity_core::{ClassName, World};
use axcenity_script::{OutputKind, ScriptRuntime};

use crate::description::default_description;
use crate::description_system::{
    parameter, remove_item, set_item, set_material_value, set_parameter,
};
use crate::script_runtime;

const FRAME: f64 = 1.0 / 60.0;

/// The repository, which is a project folder with a character library.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The repository's character library.
fn shipped() -> Library {
    let (library, _, problems) = Library::read(&repository().join(axcenity_avatar::LIBRARY_FOLDER));
    assert!(problems.is_empty(), "{problems:?}");
    library
}

const BRIEFS: &str = "axcenity/human-boxer-briefs";
const HAIR: &str = "makehuman/hair-short02";
const SHIRT: &str = "makehuman/crude-male-shirt";
const TROUSERS: &str = "makehuman/cargo-pants";
const SHOES: &str = "makehuman/shoes01";
const HAT: &str = "makehuman/fedora";
const BOB: &str = "makehuman/hair-bob02";
const LONG: &str = "makehuman/hair-long01";
const SLEEVES: &str = "makehuman/armsleeve-white-m";

/// A dressed character on the human base: what its body type wears (the
/// boxer briefs), then short hair, a shirt, cargo trousers, shoes and a
/// hat.
fn default_character(library: &Library) -> Description {
    let mut description = default_description(library, DEFAULT_BASE);
    assert_eq!(worn(&description), [BRIEFS]);
    description
        .items
        .extend([HAIR, SHIRT, TROUSERS, SHOES, HAT].map(WornItem::new));
    description
}

/// What `description` wears, in order.
fn worn(description: &Description) -> Vec<String> {
    description
        .items
        .iter()
        .map(|item| match &item.variant {
            Some(variant) => format!("{}#{variant}", item.id),
            None => item.id.clone(),
        })
        .collect()
}

/// Runs `source` as a Script and as a LocalScript for a frame, with the
/// repository as the project (so its character library is the project's),
/// and returns the output: errors marked `error: `, warnings `warning: `.
fn run_both_sides(source: &str, assets: Assets) -> Vec<String> {
    let mut world = World::new();
    let mut runtime = script_runtime();
    runtime.set_assets(assets);
    let script = world.create(ClassName::Script).unwrap();
    world
        .set_parent(script, Some(world.server_script_service()))
        .unwrap();
    runtime.start_script(&mut world, script, "scripts/check.server.lua", source);
    let local = world.create(ClassName::LocalScript).unwrap();
    world
        .set_parent(local, Some(world.starter_player_scripts()))
        .unwrap();
    runtime.start_script(&mut world, local, "scripts/check.client.lua", source);
    runtime.step(&mut world, FRAME);
    lines(&mut runtime)
}

fn lines(runtime: &mut ScriptRuntime) -> Vec<String> {
    runtime
        .take_output()
        .into_iter()
        .map(|line| match line.kind {
            OutputKind::Print => line.text.replace('\t', " "),
            OutputKind::Error => format!("error: {}", line.text),
            OutputKind::Warning => format!("warning: {}", line.text),
        })
        .collect()
}

/// Each side printed the same lines (but for the script an error names):
/// the first side's.
fn one_side(lines: Vec<String>) -> Vec<String> {
    let half = lines.len() / 2;
    let client: Vec<String> = lines[half..]
        .iter()
        .map(|line| line.replace("check.client.lua", "check.server.lua"))
        .collect();
    assert_eq!(lines[..half], client, "both sides say the same");
    lines[..half].to_vec()
}

#[test]
fn a_parameter_reads_its_default_and_is_clamped_to_its_range_when_set() {
    let library = shipped();
    let mut character = default_character(&library);
    // Left out, a parameter is at the manifest's default.
    assert_eq!(parameter(&library, &character, "height"), Ok(1.0));
    assert_eq!(parameter(&library, &character, "muscle"), Ok(0.0));
    // (1.1: inside the human body's `height` range, 0.82 to 1.13.)
    set_parameter(&library, &mut character, "height", 1.1).unwrap();
    // As the file says it (1.1), not the nearest 32-bit number.
    assert_eq!(parameter(&library, &character, "height"), Ok(1.1));
    assert_eq!(character.parameters["height"], 1.1);
    // Outside its range, the nearest end.
    set_parameter(&library, &mut character, "muscle", 5.0).unwrap();
    assert_eq!(parameter(&library, &character, "muscle"), Ok(1.0));
    set_parameter(&library, &mut character, "height", -3.0).unwrap();
    assert_eq!(parameter(&library, &character, "height"), Ok(0.82));

    let unknown = parameter(&library, &character, "nose_wide").unwrap_err();
    assert!(unknown.contains("no parameter `nose_wide`"), "{unknown}");
    let unknown = set_parameter(&library, &mut character, "nose_wide", 1.0).unwrap_err();
    assert!(unknown.contains("no parameter `nose_wide`"), "{unknown}");
    for bad in [f64::NAN, f64::INFINITY] {
        let before = character.clone();
        let why = set_parameter(&library, &mut character, "height", bad).unwrap_err();
        assert!(why.contains("finite"), "{why}");
        assert_eq!(character, before, "nothing changed");
    }

    // A base model the library hasn't: what the description holds can
    // still be read, but there's no default or range to go by.
    let mut stranger = Description::new("Stranger", "other/base");
    stranger.parameters.insert("height".to_owned(), 1.5);
    assert_eq!(parameter(&library, &stranger, "height"), Ok(1.5));
    let why = parameter(&library, &stranger, "muscle").unwrap_err();
    assert!(why.contains("no base model `other/base`"), "{why}");
    let why = set_parameter(&library, &mut stranger, "height", 1.0).unwrap_err();
    assert!(why.contains("no base model `other/base`"), "{why}");
}

#[test]
fn an_item_goes_in_its_slot_in_place_of_what_the_slot_held() {
    let mut library = shipped();
    let mut character = default_character(&library);
    assert_eq!(
        worn(&character),
        [BRIEFS, HAIR, SHIRT, TROUSERS, SHOES, HAT]
    );
    // In the slot's place in the list; nothing else moves.
    set_item(&library, &mut character, "hair", BOB, None).unwrap();
    assert_eq!(worn(&character), [BRIEFS, BOB, SHIRT, TROUSERS, SHOES, HAT]);
    // An empty slot's item goes last.
    set_item(&library, &mut character, "gloves", SLEEVES, None).unwrap();
    assert_eq!(worn(&character).last().unwrap(), SLEEVES);
    assert_eq!(character.items.len(), 7);

    // What can't be worn is an error, and changes nothing.
    let before = character.clone();
    let mut refused = |library: &Library, slot: &str, id: &str, variant: Option<&str>| {
        let why = set_item(library, &mut character, slot, id, variant).unwrap_err();
        assert_eq!(character, before, "nothing changed: {why}");
        why
    };
    let why = refused(&library, "hair", "makehuman/hair-mohawk", None);
    assert!(why.contains("no item `makehuman/hair-mohawk`"), "{why}");
    let why = refused(&library, "top", LONG, None);
    assert!(
        why.contains("`hair` slot") && why.contains("`top`"),
        "{why}"
    );
    let why = refused(&library, "hair", LONG, Some("neon"));
    assert!(why.contains("no variant `neon`"), "{why}");
    // An item made for another base model doesn't fit this one.
    let mut other_base = library.clone();
    other_base.items.get_mut(LONG).unwrap().bases = vec!["other/base".to_owned()];
    let why = refused(&other_base, "hair", LONG, None);
    assert!(why.contains("doesn't fit"), "{why}");

    // A variant is kept with the item, and setting the item again changes
    // its variant where it is, keeping the item's own material values.
    library
        .items
        .get_mut(SHIRT)
        .unwrap()
        .variants
        .push(Variant {
            id: "navy".to_owned(),
            label: "Navy".to_owned(),
            materials: Default::default(),
            thumbnail: None,
            parameters: Default::default(),
        });
    let mut character = default_character(&library);
    character.items[2].materials.insert(
        "Shirt".to_owned(),
        [("roughness_scale".to_owned(), MaterialValue::Number(0.5))].into(),
    );
    set_item(&library, &mut character, "top", SHIRT, Some("navy")).unwrap();
    assert_eq!(worn(&character)[2], format!("{SHIRT}#navy"));
    assert_eq!(character.items.len(), 6);
    assert_eq!(character.items[2].materials.len(), 1);
    set_item(&library, &mut character, "top", SHIRT, None).unwrap();
    assert_eq!(worn(&character)[2], SHIRT);

    // A slot that holds several keeps what it has.
    let base = library.bases.get_mut(DEFAULT_BASE).unwrap();
    base.slots
        .iter_mut()
        .find(|slot| slot.id == "hair")
        .unwrap()
        .multiple = true;
    let mut character = default_character(&library);
    set_item(&library, &mut character, "hair", BOB, None).unwrap();
    set_item(&library, &mut character, "hair", BOB, None).unwrap();
    assert_eq!(
        worn(&character),
        [BRIEFS, HAIR, SHIRT, TROUSERS, SHOES, HAT, BOB]
    );
    // And taking the slot's items off takes them all.
    remove_item(&library, &mut character, "hair").unwrap();
    assert_eq!(character.items.len(), 5);
}

#[test]
fn removing_a_slots_item_leaves_the_others_and_an_unknown_slot_is_an_error() {
    let library = shipped();
    let mut character = default_character(&library);
    remove_item(&library, &mut character, "headwear").unwrap();
    remove_item(&library, &mut character, "hair").unwrap();
    assert_eq!(worn(&character), [BRIEFS, SHIRT, TROUSERS, SHOES]);
    // An empty slot is fine.
    remove_item(&library, &mut character, "hair").unwrap();
    remove_item(&library, &mut character, "outerwear").unwrap();
    assert_eq!(character.items.len(), 4);
    let why = remove_item(&library, &mut character, "hat").unwrap_err();
    assert!(why.contains("no slot `hat`"), "{why}");
    assert!(why.contains("hair"), "it names the slots there are: {why}");
}

/// The realistic body's underwear comes with Male or Female: a script
/// can't take it off or swap it, and setting the frame dresses for it.
#[test]
fn a_script_cant_take_off_the_underwear_a_body_type_wears() {
    let library = shipped();
    let mut character = default_description(&library, DEFAULT_BASE);
    assert_eq!(worn(&character), [BRIEFS]);
    let why = remove_item(&library, &mut character, "underwear_bottom").unwrap_err();
    assert!(why.contains("comes with `frame`"), "{why}");
    let why = set_item(
        &library,
        &mut character,
        "underwear_bottom",
        "makehuman/female-panties-01",
        None,
    )
    .unwrap_err();
    assert!(why.contains("can't be changed"), "{why}");
    assert_eq!(worn(&character), [BRIEFS]);

    set_parameter(&library, &mut character, "frame", 1.0).unwrap();
    let mut items = worn(&character);
    items.sort();
    assert_eq!(items, ["axcenity/human-bandeau", BRIEFS]);
}

#[test]
fn a_material_value_is_checked_against_its_slots_shading_model() {
    let library = shipped();
    let mut character = default_character(&library);
    let tint = MaterialValue::Colour(Colour([192, 128, 96]));
    set_material_value(
        &library,
        &mut character,
        "Skin",
        "melanin",
        MaterialValue::Number(0.4),
    )
    .unwrap();
    set_material_value(&library, &mut character, "Skin", "tint", tint).unwrap();
    set_material_value(
        &library,
        &mut character,
        "Brows",
        "redness",
        MaterialValue::Number(0.5),
    )
    .unwrap();
    assert_eq!(
        character.materials["Skin"]["melanin"],
        MaterialValue::Number(0.4)
    );
    assert_eq!(character.materials["Skin"]["tint"], tint);
    assert_eq!(
        character.materials["Brows"]["redness"],
        MaterialValue::Number(0.5)
    );
    // Set again, a value is replaced.
    set_material_value(
        &library,
        &mut character,
        "Skin",
        "melanin",
        MaterialValue::Number(-1.0),
    )
    .unwrap();
    assert_eq!(
        character.materials["Skin"]["melanin"],
        MaterialValue::Number(-1.0)
    );
    assert_eq!(character.materials["Skin"].len(), 2);

    let before = character.clone();
    let mut refused = |slot: &str, name: &str, value: MaterialValue| {
        let why = set_material_value(&library, &mut character, slot, name, value).unwrap_err();
        assert_eq!(character, before, "nothing changed: {why}");
        why
    };
    // A name the slot's shading model hasn't.
    let why = refused("Skin", "shine", MaterialValue::Number(1.0));
    assert!(why.contains("`shine`") && why.contains("melanin"), "{why}");
    // The wrong kind of value, both ways.
    let why = refused("Skin", "melanin", tint);
    assert!(why.contains("number"), "{why}");
    let why = refused("Skin", "tint", MaterialValue::Number(0.5));
    assert!(why.contains("Color3"), "{why}");
    // A number outside the value's range (it isn't clamped), or not one.
    let why = refused("Skin", "melanin", MaterialValue::Number(3.0));
    assert!(why.contains("-1") && why.contains("to 1"), "{why}");
    let why = refused("Skin", "melanin", MaterialValue::Number(f32::NAN));
    assert!(why.contains("finite"), "{why}");
    // Only the base model's materials: an item's are its own.
    let why = refused("Shirt", "tint", tint);
    assert!(
        why.contains("no material `Shirt`") && why.contains("Skin"),
        "{why}"
    );
}

#[test]
fn scripts_read_and_change_a_description_on_both_sides() {
    let lines = run_both_sides(
        r##"
local description = Instance.new("HumanoidDescription")
local function worn()
	local names = {}
	for _, item in description:GetItems() do
		local variant = if item.Variant then "#" .. item.Variant else ""
		table.insert(names, item.Slot .. "=" .. item.Id .. variant)
	end
	return table.concat(names, " ")
end
-- New, it is the default character of the default base model, and holds
-- nothing until it's changed.
print(description.Base == "", description.Data == "")
print(worn())
print(description:GetParameter("height"), description:GetParameter("muscle"))
description:SetParameter("height", 1.1)
description:SetParameter("muscle", 5)
print(description:GetParameter("height"), description:GetParameter("muscle"))
print(description.Base, description.Data ~= "")
description:SetItem("hair", "makehuman/hair-bob02")
description:SetItem("headwear", "makehuman/fedora")
description:RemoveItem("headwear")
description:SetItem("gloves", "makehuman/armsleeve-white-m")
print(worn())
description:SetMaterialValue("Skin", "melanin", 0.4)
description:SetMaterialValue("Brows", "tint", Color3.fromRGB(90, 122, 154))
-- Its Data is the character file's text, so another one given it is the
-- same character.
local copy = Instance.new("HumanoidDescription")
copy.Data = description.Data
print(#copy:GetItems(), copy:GetParameter("height"))
print(string.find(description.Data, '"melanin": 0.4', 1, true) ~= nil)
print(string.find(description.Data, '"tint": "#5a7a9a"', 1, true) ~= nil)
"##,
        Assets::new(repository()),
    );
    assert_eq!(
        one_side(lines),
        [
            "true true",
            "underwear_bottom=axcenity/human-boxer-briefs",
            "1 0",
            "1.1 1",
            "axcenity/human true",
            "underwear_bottom=axcenity/human-boxer-briefs hair=makehuman/hair-bob02 \
             gloves=makehuman/armsleeve-white-m",
            "3 1.1",
            "true",
            "true",
        ]
    );
}

#[test]
fn a_scripts_mistakes_with_a_description_are_errors_that_say_why() {
    let lines = run_both_sides(
        r#"
local description = Instance.new("HumanoidDescription")
local function try(method, ...)
	local ok, why = pcall(description[method], description, ...)
	print(method, ok, why)
end
try("GetParameter", "nose_wide")
try("GetParameter", 5)
try("SetParameter", "nose_wide", 1)
try("SetParameter", "height", 0 / 0)
try("SetParameter", "height", "tall")
try("SetItem", "hair", "makehuman/hair-mohawk")
try("SetItem", "top", "makehuman/hair-bob02")
try("SetItem", "hair", "makehuman/hair-bob02", "neon")
try("SetItem", "hair")
try("RemoveItem", "hat")
try("SetMaterialValue", "Skin", "shine", 1)
try("SetMaterialValue", "Skin", "melanin", Color3.new(1, 1, 1))
try("SetMaterialValue", "Skin", "tint", 0.5)
try("SetMaterialValue", "Skin", "melanin", 3)
try("SetMaterialValue", "Shirt", "tint", Color3.new(1, 1, 1))
try("SetMaterialValue", "Skin", "tint", "red")
-- Nothing was changed by any of them.
print(description.Data == "")
-- Data that isn't a character file.
description.Data = "{ not json"
try("GetItems")
try("SetParameter", "height", 1)
print(description.Data)
-- The methods are a HumanoidDescription's.
print(pcall(description.GetItems, workspace))
"#,
        Assets::new(repository()),
    );
    let lines = one_side(lines);
    let expected = [
        ("GetParameter false", "no parameter `nose_wide`"),
        ("GetParameter false", "must be a string, not number"),
        ("SetParameter false", "no parameter `nose_wide`"),
        ("SetParameter false", "finite number"),
        ("SetParameter false", "finite number, not string"),
        ("SetItem false", "no item `makehuman/hair-mohawk`"),
        ("SetItem false", "`hair` slot"),
        ("SetItem false", "no variant `neon`"),
        ("SetItem false", "must be a string, not nil"),
        ("RemoveItem false", "no slot `hat`"),
        ("SetMaterialValue false", "`shine`"),
        ("SetMaterialValue false", "takes a number"),
        ("SetMaterialValue false", "takes a Color3"),
        ("SetMaterialValue false", "from -1 to 1"),
        ("SetMaterialValue false", "no material `Shirt`"),
        ("SetMaterialValue false", "a number or a Color3, not string"),
        ("true", ""),
        ("GetItems false", "Data"),
        ("SetParameter false", "Data"),
        ("{ not json", ""),
        ("false", "HumanoidDescription"),
    ];
    assert_eq!(lines.len(), expected.len(), "{lines:#?}");
    for (line, (start, inside)) in lines.iter().zip(expected) {
        assert!(
            line.starts_with(start) && line.contains(inside),
            "`{line}` should start with `{start}` and say `{inside}`"
        );
    }
}

#[test]
fn a_project_without_a_character_library_is_told_so() {
    let lines = run_both_sides(
        r#"
local description = Instance.new("HumanoidDescription")
-- Nothing to wear, and nothing to check a parameter against.
print(#description:GetItems())
print(pcall(description.SetParameter, description, "height", 1))
print(pcall(description.SetItem, description, "hair", "axcenity/hair-bob"))
-- A Humanoid with no character has nothing applied, and nothing to apply to.
local humanoid = Instance.new("Humanoid")
print(humanoid:GetAppliedDescription())
print(pcall(humanoid.ApplyDescription, humanoid, description))
local model = Instance.new("Model")
humanoid.Parent = model
print(pcall(humanoid.ApplyDescription, humanoid, description))
print(pcall(humanoid.ApplyDescription, humanoid, model))
print(pcall(humanoid.ApplyDescription, humanoid))
-- And there is no character to copy.
print(pcall(game.Players.CreateHumanoidModelFromDescription, game.Players, description))
print(pcall(game.Players.CreateHumanoidModelFromDescription, game.Players, "Ada"))
"#,
        Assets::default(),
    );
    let lines = one_side(lines);
    let expected = [
        ("0", ""),
        (
            "false",
            "no character library (`assets/characters/catalogue.json`)",
        ),
        (
            "false",
            "no character library (`assets/characters/catalogue.json`)",
        ),
        ("nil", ""),
        ("false", "needs the Humanoid to be in a character's Model"),
        ("false", "no skinned body"),
        ("false", "needs a HumanoidDescription, not a Model"),
        ("false", "needs a HumanoidDescription, not nil"),
        ("false", "StarterCharacter"),
        ("false", "needs a HumanoidDescription, not string"),
    ];
    assert_eq!(lines.len(), expected.len(), "{lines:#?}");
    for (line, (start, inside)) in lines.iter().zip(expected) {
        assert!(
            line.starts_with(start) && line.contains(inside),
            "`{line}` should start with `{start}` and say `{inside}`"
        );
    }
}
