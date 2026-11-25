//! Character Studio overhaul CO3: character descriptions from scripts
//! (`gameplay`'s `description_system.rs`, `docs/scripting/api-character.md`).
//! A Script builds a HumanoidDescription and applies it to the player's
//! character, reads back what's applied, makes a character from one, and
//! changes the one a character keeps, which the next frame applies.

use axcenity_core::{Color3, material_value_attribute};
use axcenity_script::OutputKind;

use super::*;

/// A new Baseplate project whose library has the human base and what
/// these scripts wear, its player spawned.
fn game(name: &str) -> Game {
    let mut game = Game::new(name);
    let shipped = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters");
    let mut all = axcenity_avatar::Description::new("Everything", "axcenity/human");
    all.items = [
        "makehuman/hair-bob02",
        "makehuman/hair-long01",
        "makehuman/crude-male-shirt",
        "makehuman/cargo-pants",
        "makehuman/shoes01",
    ]
    .map(axcenity_avatar::WornItem::new)
    .to_vec();
    axcenity_project::add_character_packages_for(&game.folder, &shipped, &[&all]).unwrap();
    game.frames(5, MoveInput::default());
    game.scripts.take_output();
    game
}

/// What every script here starts with: the player's character.
const CHARACTER: &str = r#"
local character = game.Players:GetPlayers()[1].Character
local humanoid = character:FindFirstChildOfClass("Humanoid")
"#;

/// Runs `source` as a Script, then `frames` frames, and returns the output:
/// errors marked `error: `, warnings `warning: `.
fn run(game: &mut Game, source: &str, frames: usize) -> Vec<String> {
    let script = game.world.create(ClassName::Script).unwrap();
    let service = game.world.server_script_service();
    game.world.set_parent(script, Some(service)).unwrap();
    let source = format!("{CHARACTER}{source}");
    game.scripts.start_script(
        &mut game.world,
        script,
        "scripts/describe.server.lua",
        &source,
    );
    game.frames(frames, MoveInput::default());
    game.scripts
        .take_output()
        .into_iter()
        .map(|line| match line.kind {
            OutputKind::Print => line.text.replace('\t', " "),
            OutputKind::Error => format!("error: {}", line.text),
            OutputKind::Warning => format!("warning: {}", line.text),
        })
        .collect()
}

/// The MeshParts inside `body`, by name.
fn parts(game: &Game, body: InstanceId) -> Vec<String> {
    let mut names: Vec<String> = game
        .world
        .get(body)
        .unwrap()
        .children()
        .iter()
        .filter_map(|child| game.world.get(*child))
        .filter(|child| child.class() == ClassName::MeshPart)
        .map(|child| child.name().to_owned())
        .collect();
    names.sort();
    names
}

fn attribute(game: &Game, id: InstanceId, name: &str) -> Option<Value> {
    game.world.get(id).unwrap().attribute(name).cloned()
}

/// The HumanoidDescriptions inside `character`'s Humanoid.
fn kept(game: &Game, character: InstanceId) -> Vec<InstanceId> {
    let humanoid = game.world.find_first_child(character, "Humanoid").unwrap();
    game.world
        .get(humanoid)
        .unwrap()
        .children()
        .iter()
        .copied()
        .filter(|child| game.world.get(*child).unwrap().class() == ClassName::HumanoidDescription)
        .collect()
}

#[test]
fn a_script_builds_a_description_and_applies_it_to_the_character() {
    let mut game = game("description-lua-apply");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    let lines = run(
        &mut game,
        r#"
local description = Instance.new("HumanoidDescription")
description:SetParameter("height", 1.1)
description:SetParameter("fat", 0.5)
description:SetItem("hair", "makehuman/hair-bob02")
description:SetItem("top", "makehuman/crude-male-shirt")
description:SetItem("bottom", "makehuman/cargo-pants")
description:SetItem("shoes", "makehuman/shoes01")
description:RemoveItem("outerwear")
description:SetMaterialValue("Skin", "melanin", 0.4)
description:SetMaterialValue("Brows", "tint", Color3.fromRGB(90, 122, 154))
humanoid:ApplyDescription(description)
print("applied")
-- What's applied comes back as a copy of its own, not in the game.
local applied = humanoid:GetAppliedDescription()
print(applied.ClassName, applied.Parent, applied ~= description)
print(applied ~= humanoid:FindFirstChildOfClass("HumanoidDescription"))
print(applied ~= humanoid:GetAppliedDescription())
print(applied.Base, applied.Data == description.Data)
print(applied:GetParameter("height"), #applied:GetItems())
-- Changing the copy changes nothing on the character.
applied:RemoveItem("hair")
print(#humanoid:GetAppliedDescription():GetItems())
"#,
        3,
    );
    assert_eq!(
        lines,
        [
            "applied",
            "HumanoidDescription nil true",
            "true",
            "true",
            "axcenity/human true",
            "1.1 5",
            "5",
        ]
    );
    assert_eq!(
        parts(&game, body),
        // (The boxer briefs aren't drawn under the trousers.)
        [
            "Brows",
            "CargoPants",
            "CrudeMaleShirt",
            "Eyes",
            "HairBob02",
            "Lashes",
            "Shoes01",
            "Teeth",
            "Tongue"
        ]
    );
    // The body slider, and the build worn live on the library's body
    // (ADR-0041): its own mesh, the identity named by its key.
    let height = attribute(&game, body, "BodyHeight").and_then(|value| value.as_number());
    assert!((height.unwrap() - 1.1).abs() < 1e-6, "{height:?}");
    let mesh = game.world.get(body).unwrap().property("MeshId").cloned();
    let mesh = mesh.as_ref().and_then(Value::as_str).unwrap();
    assert!(mesh.contains("HumanParts.glb#AxcenityHuman/Body"), "{mesh}");
    assert!(axcenity_anim::identity_key(&game.world, body).is_some());
    // The material values, on the body.
    let melanin = attribute(&game, body, &material_value_attribute("Skin", "melanin"))
        .and_then(|value| value.as_number());
    assert!((melanin.unwrap() - 0.4).abs() < 1e-6, "{melanin:?}");
    assert_eq!(
        attribute(&game, body, &material_value_attribute("Brows", "tint")),
        Some(Value::Color3(Color3::new(
            90.0 / 255.0,
            122.0 / 255.0,
            154.0 / 255.0
        )))
    );
    // One description is kept, and the frames after didn't make another.
    assert_eq!(kept(&game, character).len(), 1);
    game.frames(20, MoveInput::default());
    game.no_errors();

    // An item that can't be worn is left out with a warning, not an error:
    // the rest is applied.
    let lines = run(
        &mut game,
        r#"
local description = humanoid:GetAppliedDescription()
description.Data = string.gsub(description.Data, "crude%-male%-shirt", "cape")
humanoid:ApplyDescription(description)
print("applied")
"#,
        2,
    );
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines[0].starts_with("warning: ") && lines[0].contains("no item `makehuman/cape`"),
        "{lines:?}"
    );
    assert_eq!(lines[1], "applied");
    assert_eq!(
        parts(&game, body),
        // (The boxer briefs aren't drawn under the trousers.)
        [
            "Brows",
            "CargoPants",
            "Eyes",
            "HairBob02",
            "Lashes",
            "Shoes01",
            "Teeth",
            "Tongue"
        ]
    );

    // One that can't be applied is an error with the reason, and the
    // character stays as it was.
    let lines = run(
        &mut game,
        r#"
local description = Instance.new("HumanoidDescription")
description.Data = '{ "format_version": 2, "name": "Ada", "base": "other/base" }'
print(pcall(humanoid.ApplyDescription, humanoid, description))
"#,
        2,
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("false") && lines[0].contains("no base model `other/base`"),
        "{lines:?}"
    );
    assert_eq!(parts(&game, body).len(), 8);
}

#[test]
fn players_makes_a_character_from_a_description() {
    let mut game = game("description-lua-create");
    let lines = run(
        &mut game,
        r#"
local description = Instance.new("HumanoidDescription")
description:SetItem("hair", "makehuman/hair-long01")
description:SetItem("top", "makehuman/crude-male-shirt")
description:RemoveItem("shoes")
local Players = game:GetService("Players")
local guard = Players:CreateHumanoidModelFromDescription(description)
print(guard.ClassName, guard.Parent, guard ~= game.StarterPlayer:FindFirstChild("StarterCharacter"))
print(guard.Humanoid:GetAppliedDescription().Data == description.Data)
guard.Name = "Guard"
guard.Parent = workspace
-- Without a StarterCharacter there is nothing to copy.
game.StarterPlayer:FindFirstChild("StarterCharacter"):Destroy()
print(pcall(Players.CreateHumanoidModelFromDescription, Players, description))
"#,
        10,
    );
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(lines[0], "Model nil true");
    assert_eq!(lines[1], "true");
    assert!(
        lines[2].starts_with("false") && lines[2].contains("StarterPlayer.StarterCharacter"),
        "{lines:?}"
    );
    let guard = game
        .world
        .find_first_child(game.world.workspace(), "Guard")
        .expect("the script put it in the Workspace");
    let body = game.world.find_first_child(guard, "Body").unwrap();
    assert_eq!(
        parts(&game, body),
        [
            "BoxerBriefs",
            "Brows",
            "CrudeMaleShirt",
            "Eyes",
            "HairLong01",
            "Lashes",
            "Teeth",
            "Tongue"
        ]
    );
    assert_eq!(kept(&game, guard).len(), 1);
    // It's a character like the player's: posed with its parts.
    let hair = game.world.find_first_child(body, "HairLong01").unwrap();
    assert!(game.animators.poses().contains_key(&body), "posed");
    assert!(game.animators.poses().contains_key(&hair), "its hair too");
    // The player's own character is as it was.
    let own = game.character();
    assert!(kept(&game, own).is_empty());
    game.no_errors();
}

#[test]
fn a_kept_description_changed_by_a_script_is_applied_the_next_frame() {
    let mut game = game("description-lua-kept");
    let character = game.character();
    let body = game.world.find_first_child(character, "Body").unwrap();
    // A description put in the Humanoid is the character's: nothing calls
    // ApplyDescription.
    let lines = run(
        &mut game,
        r#"
local description = Instance.new("HumanoidDescription")
description:SetItem("hair", "makehuman/hair-bob02")
description.Parent = humanoid
"#,
        2,
    );
    assert_eq!(lines, Vec::<String>::new());
    assert!(parts(&game, body).contains(&"HairBob02".to_owned()));
    assert_eq!(kept(&game, character).len(), 1);

    // Changed with its methods.
    let lines = run(
        &mut game,
        r#"
local kept = humanoid:FindFirstChildOfClass("HumanoidDescription")
kept:SetItem("hair", "makehuman/hair-long01")
kept:SetParameter("height", 0.9)
"#,
        2,
    );
    assert_eq!(lines, Vec::<String>::new());
    let names = parts(&game, body);
    assert!(names.contains(&"HairLong01".to_owned()), "{names:?}");
    assert!(!names.contains(&"HairBob02".to_owned()), "{names:?}");
    let height = attribute(&game, body, "BodyHeight").and_then(|value| value.as_number());
    assert!((height.unwrap() - 0.9).abs() < 1e-6, "{height:?}");

    // Changed as a plain property: another description's Data.
    let lines = run(
        &mut game,
        r#"
-- (Bare but for what its body type wears, which can't be taken off;
-- a parameter set, so its Data isn't empty.)
local bare = Instance.new("HumanoidDescription")
bare:SetParameter("height", 1)
humanoid:FindFirstChildOfClass("HumanoidDescription").Data = bare.Data
"#,
        2,
    );
    assert_eq!(lines, Vec::<String>::new());
    assert_eq!(
        parts(&game, body),
        ["BoxerBriefs", "Brows", "Eyes", "Lashes", "Teeth", "Tongue"]
    );

    // Data that can't be read is said once, as an error, and the character
    // stays as it was.
    let lines = run(
        &mut game,
        r#"
humanoid:FindFirstChildOfClass("HumanoidDescription").Data = "{ not json"
"#,
        5,
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("error: ") && lines[0].contains("wasn't applied"),
        "{lines:?}"
    );
    assert_eq!(
        parts(&game, body),
        ["BoxerBriefs", "Brows", "Eyes", "Lashes", "Teeth", "Tongue"]
    );
    game.no_errors();
}
