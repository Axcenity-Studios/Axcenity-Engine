# Character Studio overhaul: architecture and contracts

Stage 2 (CO1) of [the plan](../../../plans/axcenity-engine-character-studio.md). This file is the **contract**: the layers, the pipeline systems, the file formats with their exact fields, what each value means, and the Rust and Lua signatures later stages build on. Only the lead changes it (plan section 5); a specialist who needs a change asks the lead first ([agent-assignments.md](agent-assignments.md)). Decisions behind it: [ADR-0032](../../../wiki/Architecture/Architecture-Decisions/ADR-0032-Semi-Realistic-Default-Character-Family.md) to [ADR-0037](../../../wiki/Architecture/Architecture-Decisions/ADR-0037-One-Groom-Asset-With-Card-Strand-And-Mesh-Tiers.md).

**Built in CO1:** the `avatar` crate (`crates/avatar`) with the description, base model manifest, item manifest, catalogue and material schemas, their checks, format 1 migration and parameter resolution; the stylized humanoid's manifests (`assets/characters/humanoid/Humanoid.base.json`, `items/*.item.json`, made by `manifests.py`) and the library's `assets/characters/catalogue.json` (made by `catalogue.py`). **Built in CO3 (the runtime applier):** `avatar::resolve`; derived models in `assets` (the identity bake, [ADR-0038](../../../wiki/Architecture/Architecture-Decisions/ADR-0038-Identity-Is-Baked-Into-Derived-Models-Named-By-MeshId.md); since refinement R2a only an optional bake, characters wear their identity live, [ADR-0041](../../../wiki/Architecture/Architecture-Decisions/ADR-0041-Shared-Base-Mesh-With-Live-Identity-Morphs-Baking-Is-Optional.md)); bone changes after the pose and followers taking live shape weights in `anim`; the `HumanoidDescription` class; the applier and its Lua API in `gameplay` (`description.rs`, `description_system.rs`); material values read by `scene`; the built-in library written into projects by `project`. Sections 7, 10, 14, 15, 16 and 17 say what was built. **Built in CO4 (the realistic base body):** the `axcenity/realistic` base model (`assets/characters/realistic/`, section 6), and in `anim` the twist and helper bone driver and the clip rule for base models that share bone names (section 12). It has one body shape (`Frame_Feminine`) until CO5 adds the body shapes and CO6 the face, and no items or hair, so it is not yet a family characters are made from. **Built in CO9's fitting core (refinement wave 3):** `avatar::fit` (binding, the transfer of the body's weights and identity and corrective shapes, push-out from the body and lower layers at rest, per shape and at the extremes, coverage, slot classification, the import flow with a thumbnail and a fitted glTF) and region hiding by the body's map (sections 9 and 16); not yet: shipped binding garments (CO10), loose-garment proxies, glTF material variants on import. Everything still marked *(CO10)* and so on is the contract for that milestone, not built. Character Studio's workspace still writes format 1 and poses its own preview until CO12; a test holds the two to the same result.

## 1. Three layers

| Layer | What it is | Where | Never |
|---|---|---|---|
| **Character Data** | The formats: a character's description, base model and item manifests, the catalogue, material values | `crates/avatar` (depends on `assets` only; may depend on `core`) | Rendering, UI, Instances, editor state |
| **Character Runtime** | Loads, applies and changes characters in a running game: resolves a description into bone changes, shape weights, attributes, material values and a parts list; wears identity live on the shared base mesh (ADR-0041; baking is optional, `gameplay::description::bake`); fits items *(CO9)* | `avatar` (pure resolution; fitting *CO9*) + `assets` (registered identities, `Model::live`; derived models for a bake) + `anim` (posing, live identity) + `gameplay` (the applier, `description.rs`, and the `HumanoidDescription` Lua API) + `scene` (material values) | Editor code; the player must not depend on `editor`, `egui` or `blocks` |
| **Character Studio** | The authoring workspace: panels drawn from the manifests, the wardrobe browser, presets, the stage | `crates/editor` (`character_workspace*`, `start/characters.rs`) | Writing editor state into Character Data |

Editor state (camera, stage lighting, backdrop, panel layout, undo history, selection) stays in the editor's settings or memory, never in a `*.character.json`.

## 2. Pipeline systems (addition A2)

Separate systems, each with its own module, owner and tests, integrated in one experience. Owners are the agents of [agent-assignments.md](agent-assignments.md).

| System | Offline (Blender, Python) | Runtime (Rust) | Owner | Tests |
|---|---|---|---|---|
| Creation and editing | | `editor/src/character_workspace*` | Agent 5 | Walkthroughs, drawing snapshots |
| Mesh generation and deformation (body, shapes, live identity; the optional bake) | `assets/characters/<family>/body.py`, `bodyshapes.py` | `avatar::resolve`; `Model::live` in `assets` and `anim::live_identity` *(R2a, ADR-0041)*; derived models in `assets` for an explicit bake *(CO3)* | Agent 1 (lead for live identity and the bake) | Manifest vs model; geometry QA |
| Clothing fitting | `fit.py`; the binding build *(CO9)* | `avatar::fit` *(CO9)* | Agent 4 | Poke-through thresholds per item × body |
| Hair authoring | `hair.py`; the groom converter *(CO11)* | groom runtime in `anim`/`scene` *(CO11)* | Agent 3 | Renders in motion; clipping at rest |
| Material authoring | `materials.py`, the bakes | `scene` material translation; `render` shading models *(CO7)* | Agent 6 | GPU timings; comparison renders |
| Rigging and skinning | `rig.py` (twist bones, weights) | `anim` (twist bones, correctives, body sliders) | Agent 1 | Joint volume in the QA poses |
| Packaging and export | `manifests.py`, `catalogue.py`, the import flow *(CO9)* | `avatar::Library`; character packages *(CO13)* | Lead (formats), Agent 4 (import) | Library tests (`crates/avatar/tests/shipped_library.rs`) |

## 3. Data flow

```text
 Content library (disk)                  Description (data)                Runtime (player, server, editor)
 catalogue.json                          *.character.json, format 2        HumanoidDescription ── Humanoid:ApplyDescription (CO3)
 <family>/<Name>.base.json  ──────┐       base: "axcenity/realistic"              │
   glTF body + parts, skeleton    │       parameters: {frame, fat, …}             ▼
   parameters, regions, slots     ├────►  items: [{id, variant, materials}]  avatar: parameter_values → drivers
 <family>/items/*.item.json       │       materials: {Skin: {melanin: …}}    ├─ shape weights ─► live weights on the shared base mesh (anim, ADR-0041; a bake is optional) ─► scene ─► render
   glTF + binding / groom tiers   │                                          ├─ bone offsets and scales ─► anim (after the pose, with BodyShape)
   materials, variants, tags      │                                          ├─ attributes (BodyHeight, …) ─► the body MeshPart ─► anim's body sliders
 *.groom (strands, optional) ─────┘                                          ├─ material values ─► scene material translation ─► render shading models
                                                                             └─ parts list ─► MeshParts that follow the body (followers)
```

Networking replicates the description (small data), never meshes; each machine resolves it itself *(CO13)*.

## 4. Versioning, IDs and paths

**Versions.** Every file has `format_version`. A reader refuses a newer version with a message ("It was saved by a newer Axcenity (character format 3; this version reads up to 2)") and migrates an older one on load (`avatar::FormatError::Newer`). A file with no `format_version` is version 1.

- Adding an **optional** field is not a version change. A description keeps fields it doesn't know at its top level and on each worn item (`Description::extra`, `WornItem::extra`), so they survive a round trip through an older editor. Manifests and the catalogue **ignore** unknown fields: the engine copies manifests as files, but it writes a project's catalogue again from its typed value, which drops what it doesn't know.
- **A new value in an existing enum is a breaking change**: an older engine rejects the whole file (`category`, `kind`, `fit.method`, `constraints[].kind`, `shading`, `tier`). Add a field instead, or bump the version with a migration. The enums added in R1 (`level`, `materials[].role`, a preset's `kind`) read a value they don't know as `other` (`#[serde(other)]`), so they can grow.
- Changing what a field means, renaming or removing one, or making one required **is**: bump the format's version constant in `avatar`, add the migration, add a test that an old file migrates, and update this section.
- Damaged files are errors, never panics (`FormatError::Damaged`). Checks return every `Problem` they find (`at`: a path into the file such as `parameters[3].shapes[0]`; `message`).

| Format | Constant | Version |
|---|---|---|
| Character description `*.character.json` | `DESCRIPTION_VERSION` | 2 (1 was CS2's `Look`) |
| Base model manifest `*.base.json` | `BASE_VERSION` | 1 |
| Item manifest `*.item.json` | `ITEM_VERSION` | 1 |
| Catalogue `catalogue.json` | `CATALOGUE_VERSION` | 1 |
| Groom file `*.groom` (binary) | `GROOM_VERSION` *(CO11)* | 1 |

**IDs** name base models and items: lower-case segments of `a`–`z`, `0`–`9`, `-`, `_`, `.` joined by `/`, at least two (namespace, then name): `axcenity/tee`, `studio-x/coats/parka`. First-party content uses `axcenity/`; an import picks its own namespace (the project's or the author's). `avatar::is_valid_id`.

**Paths** inside a manifest are relative to the manifest's folder, `/`-separated, never absolute, no drive letter or backslash (`avatar::is_valid_path`). `..` is allowed but must stay inside the library (`Library::path_of`, checked by `Library::validate`). Model objects are named by their `MeshId` pick path (`AxcenityHumanoid/Shirt_Tee`), as `assets::Pick` reads them.

## 5. The character description (`*.character.json`, format 2)

One character: its base model, parameter values, items and material values. **Values only**: no meshes, no editor state.

```json
{
  "format_version": 2,
  "name": "Ada",
  "base": "axcenity/stylized",
  "parameters": { "frame": 0.6, "build": -0.3, "height": 1.04, "nose_width": 0.2 },
  "items": [
    { "id": "axcenity/tee", "variant": "navy", "materials": { "Shirt": { "tint": "#e8e8e8" } } },
    { "id": "axcenity/hair-bob" }
  ],
  "materials": { "Skin": { "melanin": 0.4 }, "Iris": { "iris_tint": "#5a7a9a" } }
}
```

| Field | Type | Meaning |
|---|---|---|
| `format_version` | 2 | |
| `name` | string | Shown in the studio and to scripts; not empty |
| `base` | ID | The base model package |
| `parameters` | map ID → number | Values by the base manifest's parameter IDs; one left out takes its default; resolution clamps to the range (a check reports values outside it) |
| `items` | list of `{id, variant?, materials?, parameters?, transform?}` | Worn items, each in its manifest's slot (and its `also_fills` slots); one per slot unless the slot is `multiple`. `variant` names one of the item's variants; `materials` are values over the variant's, by the item's material slot names; `parameters` are values for the item's own parameters (R1), over the variant's; `transform` `{offset, turn, scale}` moves a socket item (metres, degrees about X, Y, Z, a factor; each part optional) inside its `adjust` (R1). Unknown fields of a worn item are kept (`WornItem::extra`) |
| `materials` | map slot → values | Values for the base model's materials (skin, eyes, brows, lashes, mouth, teeth), see section 10 |
| `skin` | `{makeup: [SkinLayer], markings: [SkinLayer]}` | R5 ([ADR-0042](../../../wiki/Architecture/Architecture-Decisions/ADR-0042-Makeup-And-Markings-Are-Layers-Over-Library-Masks-Drawn-Live-By-The-Skin-Model.md)), optional: makeup and markings turned on. A `SkinLayer` is `{layer, enabled?, colour?, opacity?, roughness?, amount?, seed?}`: `layer` names one of the base's `skin_layers` (makeup in `makeup`, markings in `markings`, each once); `enabled` false keeps its settings and draws nothing (absent: true); `opacity` 0 to 1 (absent: 1); `colour` `#rrggbb` and `roughness` 0 to 1 over the base's; a marking's `amount` (the share of its spots shown, 0 to 1) and `seed` (a whole number: another seed, other spots). Unknown fields of the block and of a layer are kept (decals, later). An older engine keeps the whole block in `extra` |
| anything else | any | Kept as it was (`Description::extra`): room for later additive fields |

**Addition A3, field by field:** versioned with migration (`format_version`, section 4); external asset references (`base` and item IDs, resolved through the catalogue); morph configuration referenced (the base manifest owns what each parameter drives; the description holds values only); skeleton and rig metadata (the base manifest's `skeleton`); material references (material slot names) and values; clothing and hair references (`items`); runtime customization parameters (`parameters` and `materials`, which a game changes at runtime and re-applies); no editor state.

**Format 1** (`editor::character_workspace::Look`: `name`, `build` −1–1, `muscle` 0–1, `hair`, `top`, `outerwear`, `bottom`, `shoes`) migrates on load (`avatar/src/migrate.rs`): base `axcenity/stylized` (`V1_BASE`); `parameters` `build` and `muscle`, clamped as format 1 did; each slot's part name becomes its item ID (`V1_ITEMS`: `Shirt_Tee` → `axcenity/tee`, `Hair_Bob` → `axcenity/hair-bob`, …) in format 1's slot order; missing values are the default character's; an empty name is "New Character"; unknown part names are dropped, as format 1 ignored them. The editor's test `a_saved_look_migrates_to_the_same_parts_and_shapes` proves a migrated file wears the same parts and resolves to the same body shapes the workspace poses today. Until CO12 the workspace still writes format 1.

## 6. The base model manifest (`*.base.json`)

Ships beside a base model's glTF file. The stylized one is `assets/characters/humanoid/Humanoid.base.json` (generated by `manifests.py` from `Humanoid.customization.json` and `HumanoidParts.glb`; edit the script, not the file). The realistic one is `assets/characters/realistic/Realistic.base.json` (`axcenity/realistic`, written by that package's `manifests.py` when `build.py` runs): model `RealisticParts.glb`, body `AxcenityRealistic/Body`, parts `Eyes` and `Teeth`, 16 added bones (section 12) and 14 regions by `map_index`, stored as `RealisticRegions.png` and as the body's `TEXCOORD_1` (`assets/characters/realistic/README.md`), plus three regions with mask shapes, one under each underwear item (`under_boxer_briefs`, `under_briefs`, `under_sports_bra`: an interim exception to section 9, below).

| Field | Type | Meaning |
|---|---|---|
| `format_version` | 1 | |
| `id`, `label` | ID, string | `axcenity/stylized`, "Stylized" |
| `family` | string | `stylized`, `realistic`: groups bases; items name the exact bases they fit |
| `model` | `{file, body, parts[], regions_map?}` | The glTF file; the body's object path; parts always worn with it (eyes, teeth, brows, lashes), which follow its pose and shapes; CO9: `regions_map`, the body's region map (an image over its first texture coordinates whose red value is a region's `map_index`, 0 where none), relative to the manifest: what hides the regions worn items cover and what fitting reads coverage from |
| `skeleton` | `{convention, height, pose, added_bones[]}` | `humanoid` (Unity/VRM names), metres standing, `a-pose-45`; twist and helper bones added as `{name, parent}` (section 12) |
| `groups` | `[{id, label, category, about?, icon?}]` | Parameter groups; `category` is a studio category: `body`, `face`, `skin`, `hair`, `clothing`, `accessories`. `about` is a tooltip sentence, `icon` a name from the editor's icon set (an unknown one draws none) (R1) |
| `parameters` | `[Parameter]` | The sliders, in display order (section 7) |
| `corners` | `[{shape, factors[]}]` | Shapes driven by several parameters at once (section 7) |
| `constraints` | `[{kind: "max_sum", parameters, limit}` or `{kind: "follow", source, target, factor}]` | Rules between parameters (section 7) |
| `regions` | `[{id, label, mask_shape?, map_index?}]` | Body regions items hide: by a mask shape (stylized, ADR-0030) or an index in the body's region map ([ADR-0035](../../../wiki/Architecture/Architecture-Decisions/ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md)); at least one |
| `slots` | `[{id, label, category, multiple?, empty_label?, about?, icon?}]` | Where items are worn; data, so a new slot is not a code change |
| `sockets` | `[{name, bone}]` | Attachments that follow a bone (`anim`'s sockets); the stylized ones are `project::HUMANOID_SOCKETS` |
| `materials` | `[{name, shading, label?, values?, role?}]` | **Every** material of the body and its parts, with its shading model (section 10). R1: `label` is its name for people; `values` the values of its shading model it offers (all when absent; each must be one of the model's); `role` `primary`, `secondary` or `accent` |
| `face` | `{profile, eye_bones[], jaw_bone?}` | The FaceControls profile the head uses and its eye and jaw bones (section 13) |
| `defaults` | `{items[]}` | What a new character on this base wears |
| `features` | `[Feature]` | What a person can point at on the character (R1, below) |
| `handles` | `[Handle]` | Handles on the character that move parameters (R1 schema, R2 content, below) |
| `skin_layers` | `[{id, label, kind, material, about?, colour?, roughness?, opacity?, amount?}]` | R5 (ADR-0042): the makeup and markings its skin can draw, each a channel of the masks in the model file (`docs/rendering/skin.md`). `kind` is `makeup` or `marking` (a newer kind reads as `other`: listed, not drawn); `material` a slot with `skin` shading; `colour`, `roughness`, `opacity` and `amount` where a new layer starts. The renderer draws the IDs `foundation`, `blush`, `eyeshadow`, `eyeliner`, `lips`, `brow_fill`, `freckles`, `moles` and `scar`. The realistic base offers all nine (`manifests.py`, `SKIN_LAYERS`) |

**Parameter:**

| Field | Type | Meaning |
|---|---|---|
| `id`, `label` | string | The ID descriptions use (`snake_case`); the slider's name |
| `group` | group ID | |
| `range` | `[min, max]` | min < max |
| `default` | number | Inside the range; 0 when absent |
| `advanced` | bool | Shown only in the advanced view (D3's basic/advanced). Kept beside `level`, written in step with it |
| `about` | string | R1: one sentence, the friendly description and the tooltip |
| `feature` | feature ID | R1: the feature it belongs to (selection, randomizing a feature). Absent: its group's ID, when a feature has that ID (`BaseManifest::feature_of`). A parameter can sit in group `proportions` and belong to feature `shoulders`: the group orders the page, the feature drives selection |
| `level` | `quick`, `detailed`, `advanced` | R1: brief §3.3's three levels. Absent: `advanced` when `advanced` is true, else `detailed` (`Parameter::level`). Generators write both, so an older editor hides the same sliders; a check reports a parameter whose two disagree |
| `random` | `[min, max]` | R1: the part of the range Randomize draws from; the middle half of the range when absent (`avatar::random_span`) |
| `mirror` | parameter ID | The other side's parameter, for symmetric editing; each names the other |
| `shapes` | `[{shape, from?, to?}]` | Blend shape weights |
| `bones` | `[{bone, from?, to?, offset?, scale?}]` | At mapped weight w: the bone's head moves by `offset × w` (model space, metres, glTF axes: Y up, +Z front) and the bone scales by `1 + (scale − 1) × w`; applied after the animated pose *(CO3)* |
| `attributes` | `[{attribute, from?, to?}]` | A number attribute on the character's skinned body: `anim`'s body sliders (`BodyHeight`, `LegLength`, `ArmLength`, `ShoulderWidth`, `WaistWidth`, `HipWidth`, `HeadSize`, `HandSize`, `FootSize`; PR #400), which become one output of applying a description (D2) |
| `material_values` | `[{material, value, from?, to?}]` | A number value of a material slot's shading model |
| `choices` | `[{value, label, items?}]` | Named values to pick from instead of a slider (each inside the range): Character Studio shows a chip each and keeps the parameter at one of them (the nearest). Picking one wears its `items` in their slots and takes off the items the other choices name. The realistic body's `frame` is Male (0, boxer briefs) or Female (1, briefs and a sports bra). Optional, no version bump |

**Feature** (R1; plan section 11.3): `{id, label, category, about?, icon?, anchors: [{bone, offset}], radius, regions?}`. An anchor is a bone and an offset from its head in model space at rest (metres, glTF axes): it follows bone changes. One anchor for a middle feature, two for a pair (left first). A click picks the feature whose anchor is nearest within `radius` (metres), and the camera frames its anchors; `regions` are body regions that also pick it. The realistic body has 23 (`body` for the whole body; `head`, `neck`, `shoulders`, `chest`, `arms`, `hands`, `abdomen`, `waist`, `back`, `hips`, `thighs`, `calves`, `feet`; `eyes`, `brows`, `nose`, `cheeks`, `jaw`, `chin`, `lips`, `forehead`, `ears`), measured on the model by `assets/characters/realistic/anchors.py`: the weighted centre of the vertices the feature's shapes move (or of its region in the region map), on the skin for the face's features and at the centre of the part for the body's, following the bone that carries most of them, on the masculine frame. Their placement on the feminine frame is not checked yet.

**Handle** (R1 schema; R2 writes them and Character Studio uses them): `{id, feature, label?, mirror?, anchor: {bone, offset}, drives: [{parameter, axis, travel, screen?, sensitivity?, frames?}], views?}`. A handle only names parameters: no second geometry state (brief §3.4). Its anchor is a point of the skin at rest (a bone and an offset from its head, as a feature's): Character Studio finds the skin's corner there (`scene::nearest_corner`, a corner of the skin-shaded primitives) and draws the handle on that corner where the frame's pose puts it (morphed, skinned, the identity worn live), so it stays on the skin at every value. A drive's `axis` (model space, glTF axes) and `travel` (metres) are how that corner moves from the parameter's lowest value to its highest: the studio draws the drive's arrow, and takes a drag along it, in the direction that move shows on the screen; how far a drag goes is the studio's one rule for numeric controls, a parameter's whole range in 110 points of drag whatever the zoom (RA1, Paul's rule of 2026-11-18, `character_workspace/numeric.rs`), so the skin under the handle doesn't follow the pointer one to one, and `sensitivity` (which divides the travel) no longer changes the gain. `frames` `[{frame, axis, travel}]` (wave 3) lists the drive on a value of the `frame` parameter whose own shapes move the corner otherwise (the realistic waist's `<Frame>_Waist_*`: 37 mm on the feminine frame against 48 mm on the masculine); `axis` and `travel` are the default frame's and hold on any frame not listed within half a frame (`Drive::on_frame`). `handles.py` lists a frame when its move differs by more than 1 mm. `screen` `{view: [dx, dy]}` lists the framings the drive works in (`front`, `three_quarter`, `side` as the Side button frames it, the character's right profile, and `back`), each with the direction the corner moves on the screen there (x right, y up); a framing where less than 35% of the travel shows, or where a drive that shows more of its travel has the same line (within 25 degrees), has none. `views` are the framings where any drive works. `mirror` names the other side's handle, each naming the other, on the same feature. The realistic body's 19 handles (nose tip, bridge, nostrils; each eye's upper lid and outer corner; the jaw's corners; the mouth's corners and both lips; each shoulder, side of the waist and hip) are written by `assets/characters/realistic/handles.py`, which measures every corner and travel on the built model, working the proportion sliders' bone changes out as `anim`'s body sliders make them; the player's `handle_tests.rs` holds each travel to the applied skin's within 5% (they agree to 0.1 mm), each anchor within 2 mm of a skin corner at rest, and, at both ends of every drive's parameter on both frames, the handle the data predicts (the anchor where the starting description draws it, plus `axis` × `travel` × the share of the range the value moved) within 2 mm of the applied skin's triangles (each frame with its own drive; at most 1.7 mm). The anchor is carried by its skin corner, not by its bone alone: the feminine frame's shapes move the face up to 22 mm and the shoulders 36 mm off where their bones carry them, and a hip corner shared with the thigh's twist bone sits 5 mm off its bone's carry on either frame. An eye's outer-corner handle sits at the outer canthus (the lid margin's most lateral corner at the eye's height, then the outward-facing skin corner beside it).

**Checks** (`BaseManifest::validate`, `check_model`): feature and handle IDs unique; a parameter's `feature`, a handle's `feature` and `drives[].parameter` exist; `mirror` names back, isn't the handle itself and is on the same feature; `regions` exist; an anchor's offset and a drive's axis (and each of its `frames`' axes) are finite, the axis not zero; `travel` (each frame's too), `radius` and `sensitivity` above 0; `random` inside the range; `level` and `advanced` agree; a material slot's `values` are values of its shading model; anchor bones are bones of the body (`check_model`).

## 7. What parameters mean (resolution)

`avatar/src/resolve.rs` is the reference; the runtime and the studio's preview use it.

1. **Values.** Each parameter's value is the description's or its default, clamped to its range (`parameter_values`).
2. **`max_sum` constraints** then apply: when the named parameters (all with ranges from 0 up) add to more than `limit`, each is scaled by `limit / sum`. Example: fat and muscle at 1 with a limit of 1.5 resolve to 0.75 each.
3. **Spans.** Every driver maps the value through its `from` span onto its `to` span (`avatar::span`): `t = clamp((v − from[0]) / (from[1] − from[0]), 0, 1)`, result `to[0] + (to[1] − to[0]) × t`. `from` defaults to `[0, 1]`, `to` to `from`. A downward span is allowed: `build`'s `Build_Slim` driver is `from [0, −1] to [0, 1]`, so −0.4 gives 0.4 and any positive value gives 0. A bipolar identity shape is `from [−1, 1]` (−0.6 gives −0.6).
4. **Shapes.** A shape's weight is the sum of its drivers' results plus its **corners'**: a corner's weight is the product of its factors' spans (MakeHuman's corner targets, D5: `Feminine_Heavy` is frame × fat, so fat and muscle layers mix without cancelling). Every shape the manifest names is in the result, at 0 when nothing drives it.
5. **Regions.** Each region with a mask shape is in the result at 0, and at 1 when a worn item hides it (`body_shape_weights`). Map regions are hidden by the renderer from the region map *(CO9)*.
6. **`follow` constraints** are editing rules, not resolution: changing `source` by d moves `target` by `factor × d` unless the person pinned `target` *(CO12)*. A saved description holds the result.
7. **Item parameters** (R1) resolve the same way on the item's own part: each value is the worn item's, else its variant's, else the default, clamped; shape drivers give `ResolvedPart::shapes` (the item's shapes, never the body's) and material drivers go under the variant's and the worn item's material values. The applier wears `ResolvedPart::shapes` live on the item's part (R2a): the part's identity is the body's plus one more layer of the item's shapes, so they are weights like the body's and a change bakes nothing. A socket item carries its worn `transform` in `ResolvedPart::transform` (applied from R8).
8. **Which items are worn.** One item per slot unless the slot is `multiple`, counting `also_fills`; two items where either `excludes` the other can't both be worn. In both cases the later one in the description is worn and the earlier one is left out with a reason. An item whose `requires` aren't all worn is left out too (R1).
9. **Bones, attributes and material values** use the same spans. Bone offsets and scales from several parameters add (offsets) and multiply (scales). Attributes are written to the body MeshPart; `anim::BodyShape` applies them after the pose as it does today. A parameter's material values are the defaults the description's own `materials` go over.
10. **What a bone change does to a mesh.** A bone's `offset` moves its head in model space, absolutely: a bone no driver names stays where it is, whatever its parent does. A `scale` scales the bone and everything on it. Every pose gets the changes **after the clips** (`LiveIdentity::settle` for an identity worn live, `Model::settle` for a baked one; both called by `anim`), so every clip plays on every character. What that does to a surface depends on the mesh, per parameter: **a mesh that has one of the parameter's shapes was already moved by it and is only re-seated** (its bind moves with the joint: nothing more moves at rest, poses turn about the new joint; the body under `frame`), and **a mesh that has none of them is carried by the bone** (the eyeballs and teeth have no `Frame_Feminine`: they move, and the eyes scale, with their bones). That is why a parameter keeps its shapes and its bones together: `avatar::resolve` gives the identity one *layer* per parameter, corner and region mask, and each model decides per layer. Live, re-seating and carrying are one matrix per joint multiplied into the posed joints (`LiveIdentity::dress`); baked, they are in the vertices and the bind.

**Pose-driven corrective shapes (CO5).** `correctives`: `[{shape, bone, axis, angles: [start, full]}]`. Such a shape's weight is `clamp((θ − start) / (full − start), 0, 1)`, where θ is the twist, in degrees, about `axis` of the bone's turn from rest (its local rotation times the inverse of its rest local rotation, so in its parent's space). `axis` is a unit vector in the space of the bone's parent at rest; several entries may name one bone. A shape named here is driven by no parameter and stays live whether the identity is worn live or baked. Today the realistic body's six (`Corrective_Shoulder_Level_*`, `_Raised_*` and `_Lowered_*`, on the upper arms; lowering is a turn about the raise axis reversed, so every rule has `start < full`) are written to `Realistic.build.json` and are constants in `anim` (`correctives.rs`), which a test holds equal to the build data; the manifest field takes their place when a second body needs other rules.

**Identity versus live shapes (ADR-0041, refinement R2a; supersedes D4 and ADR-0038).** Every shape a parameter, corner or region drives is **identity**: constant until the description changes. A character on screen is its base model's **shared mesh** with the identity **worn live**: the applier registers the resolved `Identity` in `Assets` (`Assets::register_identity`, keyed by `Identity::key`, a hash of its names and numbers, the same on every machine) and writes the key on the body and every part as the attribute `Identity` (16 hex digits, `anim::IDENTITY`; a part with item parameters gets the body's identity plus a layer of its own). `anim` works out once per model and identity how that model wears it (`Model::live` → `LiveIdentity`: the identity shapes' weights by index, the bone changes, and a correction per joint), adds the bone changes after the clips, and puts the weights and joint corrections on the pose last, after the face's and the correctives' (`LiveIdentity::dress`). Followers take the body's weights by name as for expressions, then their own identity. The result is the bake's to float precision (`assets` `an_identity_worn_live_poses_the_mesh_where_its_bake_does`; in pictures, the player's `an_explicit_bake_draws_the_same_pixels_as_the_live_character`). A key this run hasn't registered (a saved world before its description is applied again) wears nothing. **Baking is optional** (`gameplay::description::bake`): it gives each MeshPart the **derived model** of ADR-0038 (the base with the identity shapes added into its vertices and dropped from its blend shapes, carried surfaces moved, the bind on the changed skeleton; `…/HumanoidParts.glb#AxcenityHumanoid/Body#i3f9c…`, `Assets::derive`) and removes the `Identity` attribute; nothing bakes by default, and applying a description again makes the character live. The renderer morphs and skins each character **once per frame** (CO8, `render` `posing.rs`): a compute pass walks only the shapes whose weight isn't 0 and writes a posed buffer that the shadow, depth, motion and main passes read and ray tracing builds its BLASes from, so identity shapes cost once, not per pass, a shape at weight 0 costs nothing, and ray-traced shadows, AO, reflections and GI show identity and expression shapes (measured in [refinement/wave3/co8-compute-posing.md](refinement/wave3/co8-compute-posing.md); before, in [refinement/r2/r2a-live-identity.md](refinement/r2/r2a-live-identity.md)). **Live** expression and corrective shapes morph every frame as before. A manifest may add `correctives` later as an optional field (no version bump): `[{shape, bone, axis, angles: [start, full]}]`.

## 8. The item manifest (`*.item.json`)

One garment, accessory or groom. The stylized ones are `assets/characters/humanoid/items/`.

| Field | Type | Meaning |
|---|---|---|
| `format_version` | 1 | |
| `id`, `label` | ID, string | |
| `kind` | `garment`, `accessory`, `groom` | |
| `slot` | slot ID | A slot of every base it fits |
| `layer` | 0–3 | Garments only, the order outward for push-out: 0 underwear, 1 base (shirts, trousers, shoes), 2 mid (sweaters, vests), 3 outer (jackets, coats) |
| `tier` | `built_in` (default), `premium` | The two content tiers (section 0, decision 2): generated by the engine's scripts, or artist-made or imported |
| `bases` | `[ID]` | The base models it fits |
| `model` | `{file, object?}` | Garments and accessories: the mesh. Grooms have none here |
| `hides` | `[region ID]` | The body regions it hides |
| `materials` | `[{name, shading}]` | **Every** material of its model(s), with its shading model |
| `variants` | `[{id, label, materials}]` | Colour and material variants: values per material slot (glTF's `KHR_materials_variants` maps onto these on import *(CO9)*) |
| `fit` | `{method: "shapes", follows[]}`, `{method: "binding", data?, margin}`, `{method: "socket", socket}` | Section 9 |
| `groom` | `{cards?, strands?, mesh?}` | Grooms only: section 11 |
| `tags` | `[string]` | For the wardrobe browser's search and filters |
| `thumbnail` | path | For the browser *(CO9 makes them)* |
| `source` | `{author, licence, url?}` | Provenance; `licence` is an SPDX ID (`CC0-1.0`). Imports must fill it |
| `about` | string | R1: a sentence for the browser |
| `parameters` | `[ItemParameter]` | R1: the item's own sliders (a hairstyle's length): `{id, label, range, default?, about?, level?, random?, shapes?, material_values?, choices?}`, shaped like a base parameter without `group`, `mirror`, `feature`, bones or attributes. `shapes` name shapes of the item's model, `material_values` its own material slots; a choice wears nothing |
| `materials[].label`, `.values`, `.role` | | R1, as on a base manifest's material slot: what the slot is called, which values it offers (recolouring), its colour role |
| `variants[].thumbnail`, `.parameters` | path; map ID → number | R1: a picture of the variant; values for the item's parameters |
| `also_fills` | `[slot ID]` | R1: other slots it occupies (a dress: `bottom`) |
| `requires` | `[item ID]` | R1: items that must be worn with it |
| `excludes` | `[item ID]` | R1: items it can't be worn with (either naming the other is enough) |
| `limitations` | `[{note, parameter?, range?}]` | R1: known limits, a note for the browser; with `parameter` (the item's or the base's) and `range` the editor can warn outside the range |
| `quality` | `draft`, `production` | CO9 (plan 11.3, [asset-quality.md](asset-quality.md)): unset is a draft (`ItemManifest::is_draft`); an import is always `draft`. Hiding drafts in the studio and keeping them out of defaults, presets and Randomize is not built yet |
| `adjust` | `{offset?: [[min, max] × 3], turn?: [[min, max] × 3], scale?: [min, max]}` | R1, socket items only: how far a person may move (metres per axis), turn (degrees about each axis) and scale it. Each limit holds the item as made (0, 0, 1); a part left out can't change |

**Supported operations are derived, not listed** (brief §14.1): an item can be recoloured when a material slot offers values, has variants when it lists them, has sliders when it has `parameters`, can be moved when it has `adjust`. An `operations` list can be added later if something can't be derived.

**Rules** (`ItemManifest::validate`, `check_against`): a garment needs a model and a layer; an accessory needs a model and no layer; only accessories hang on a socket; a groom has no `model`, has `groom`, and has **cards or a mesh** (strands are never the only tier); the slot, every hidden region and the socket exist on every base it fits; variant values suit the shading models. R1: item parameter IDs are unique, ranges go upwards and hold their defaults and `random`; material drivers drive number values of the item's own slots; a material slot's `values` are its shading model's; variant parameters exist and are in range; `also_fills` doesn't name the item's own slot and names slots every base has; `requires` and `excludes` are IDs, not the item itself, and the library has them (`Library::validate`); a limitation with a range names a parameter of the item or the base; only socket items have `adjust`. `ItemManifest::check_model`: item parameter shapes are shapes of the item's model. A description (`Description::validate`): worn parameters exist and are in range; an item without what it `requires` worn, or worn with one it `excludes`, is a problem; `transform` is inside `adjust` (no `adjust`: only the identity).

## 9. Fitting (ADR-0035)

| Method | How the item follows the body | Who uses it |
|---|---|---|
| `shapes` | The item carries its own copies of the named body shapes; the runtime gives each the body's weight by name (section 14) | The stylized humanoid's garments and hair (today's build) |
| `binding` | Each item vertex is bound to the body's surface (a triangle, the point's weights of its corners, and the vertex's offset in the frame of the triangle's smoothed normal and first edge), **made at build or import** (CO9, `avatar::fit`, ADR-0041 decision 4): through the binding the item takes the body's skin weights and **every identity and corrective shape of the body, as morph targets of the same names**, pushed clear of the body and of every lower layer by `margin` metres (default 0.002) at rest, at each shape's full weight and at the parameter extremes. At run time it is a skinned part like a `shapes` item: it takes the body's weight of every shape it has (section 14), so nothing is fitted or rebuilt when the description changes. `data` is the binding file (format 1: `{format_version, body, vertices: [{triangle, bary, local}]}`), kept for refitting | Every new garment (CO10) and every import (the test tee) |
| `socket` | Rigid on a socket; scaled by the socket bone's scale | Caps, glasses, watches |

**The realistic body's underwear** (`assets/characters/realistic/underwear.py`, 2026-11-16) is cut from the body's own surface, so it is fitted by `shapes`: its vertices are fixed mixes of body vertices and carry every body shape exactly. CO9 measured it against the transfer (`avatar`'s `tests/fitting.rs`): where its cloth lies on the skin the transferred shapes agree (90 to 96% of offsets within 1 mm), and where it bridges a hollow (the gusset, the bra between the breasts on the feminine frame) its own per-frame shapes are right and a transfer by binding would not be (up to 64 mm off), so it keeps `shapes`. Its regions keep their mask shapes, which sink the skin under it 5 mm (easing out 15 mm past the hems): it covers no map region whole, so map hiding can't stand in for them.

**Body hiding by regions** (CO9, built): the base's `model.regions_map` gives each body triangle a region (the map's value at the middle of the triangle's texture coordinates); `resolve` lists what the worn items hide (`Resolved::hidden_regions`), and the applier draws the body from a model with the triangles of every hidden **map** region left out (`axcenity_assets::RegionHiding`, `Assets::hide_regions`: the body's `MeshId` with a tag, `…#AxcenityRealistic/Body#h<16 hex>`, made from the base model on first use and shared; only the triangle lists change, at every level of detail, so live identity and posing apply unchanged; a bake on top is `…#h<key>#i<key>`). Taking the item off gives the plain `MeshId` back. An import hides the regions it covers whole (`fit::hidden_regions`, 99.5% of a region's triangles under it): the realistic `torso` region runs up the neck, so a crew neck hides `waist` but not `torso`. The stylized base keeps its mask shapes (`regions[].mask_shape`) until it is retired. Inner layers covered by an outer one are pushed under it, never through.

## 10. Character materials (ADR-0036)

Each material slot has a **shading model**. Values **modulate** the material's own maps (a tint multiplies the base colour map; a scale multiplies roughness); they never replace them, and a material with no values looks exactly as authored (every value's neutral setting changes nothing; audit §3.8). **In the World** a value is an attribute of the MeshPart whose model has the material, named `Mat_<material>_<value>` (`core::material_value_attribute`; a number or a `Color3`), written by the applier and read by `scene` (`material::modulate`), so it is saved, copied and replicated with the part. Until the renderer has the new models *(CO7)*, `scene` draws eye, cloth and hair as Standard; skin has its model since R5 (below). Since 2026-11-17 (CO7's first slice, `material::modulate`) it draws `melanin`, `undertone` and `redness` as one colour ratio in linear light on the base colour (`scene::skin_tone`: the tone table of `references/skin/README.md` §1.3, interpolated in optical density; all three at 0 give exactly 1), then `tint`, `iris_tint` and `sclera_tint` as multiplies, `roughness_scale` on the roughness and `detail_strength` on the normal map's strength (held to 0 to 2). The ratio is per material, not per texel: lips, nails, palms and soles tone with the skin until the Skin model has masks.

**How the renderer learns a slot's shading model (decided 2026-11-17, R5):** the model file says it. A glTF material's `extras` carry `"axcenity_shading": "skin"` (read by `assets` into `ModelMaterial::shading`, drawn by `scene` as `render::Shading::Skin`), and a skin may name a second, *defined* normal map beside its own smooth one (`"axcenity_defined_normal": {"index": <texture>}`). The realistic build writes both on `Skin` (`skin.py`'s `mark`). Chosen over an attribute the applier writes because the defined map must come from the file anyway, the model then draws alike wherever it is drawn (with or without a description), and it follows the `axcenity_sway` precedent; the manifest's `materials[].shading` stays the contract for which values a slot takes, and an `assets` test holds the realistic file and manifest in agreement on which materials are skin. Other models (`eye`, `cloth`, `hair`) will use the same key; an unknown one draws as Standard. On a Skin-shaded material `scene` also draws `scatter_scale` (× how far light spreads under it) and `definition × (1 − softness)` (how much of the defined normal map shows); the realistic manifest drives `definition` from `muscle` and `softness` from `fat` (`manifests.py`'s `SKIN_DRIVERS`), so abdominal lines show only on lean, muscular bodies. What the Skin model draws: `docs/rendering/skin.md`. Other slots' values are still drawn by name (Hair's `melanin` and `redness` take the skin's curve meanwhile). Still stored only: `pupil_size`, `limbal_ring`, `iris_depth`, the cloth and hair values beyond `tint` and `roughness_scale`. The set is `avatar::ShadingModel::values`:

| Model | Value | Kind, range, neutral | Meaning |
|---|---|---|---|
| `standard` | `tint` | colour, white | × base colour map |
| | `roughness_scale` | 0–2, 1 | × roughness |
| `skin` | `tint` | colour, white | × albedo after toning |
| | `melanin` | −1–1, 0 | Lighter to darker than the map, along a melanin absorption curve |
| | `redness` | −1–1, 0 | Haemoglobin: pallor to flush |
| | `undertone` | −1–1, 0 | Cooler to warmer |
| | `roughness_scale` | 0.5–1.5, 1 | Oily to dry |
| | `detail_strength` | 0–2, 1 | Pore and micro-detail normal |
| | `scatter_scale` | 0–2, 1 | × subsurface radius |
| | `definition` | 0–1, 0 | How much of the defined (lean, muscular) normal map shows over the smooth one; the realistic body drives it from `muscle` |
| | `softness` | 0–1, 0 | The share of that definition fat covers; the realistic body drives it from `fat` |
| `eye` | `iris_tint`, `sclera_tint` | colour, white | × iris map, × sclera map |
| | `pupil_size` | 0.5–2, 1 | × authored pupil diameter |
| | `limbal_ring` | 0–2, 1 | × ring darkness |
| | `iris_depth` | 0–2, 1 | × iris parallax depth under the cornea |
| `cloth` | `tint`, `sheen_tint` | colour, white | × base colour map, × authored sheen colour |
| | `roughness_scale`, `sheen_roughness_scale` | 0.5–1.5, 1 | |
| | `anisotropy_scale` | 0–2, 1 | × authored anisotropy (satin, silk) |
| `hair` | `tint` | colour, white | × colour after melanin and dye |
| | `melanin`, `redness` | −1–1, 0 | Lighter to darker; ash to red (pheomelanin) |
| | `dye`, `dye_amount` | colour white; 0–1, 0 | A dye mixed in |
| | `tip_lightening` | 0–1, 0 | Root-to-tip gradient |
| | `highlight` | 0–2, 1 | Secondary (coloured) highlight strength |
| | `roughness_scale` | 0.5–1.5, 1 | |

**Makeup and markings (R5, [ADR-0042](../../../wiki/Architecture/Architecture-Decisions/ADR-0042-Makeup-And-Markings-Are-Layers-Over-Library-Masks-Drawn-Live-By-The-Skin-Model.md)):** a description's `skin` block (section 5) resolves to **draw values** on each layer's material, `layer_<id>_opacity`, `_colour`, `_roughness`, and for markings `_amount` and `_seed` (`avatar::skin_layer_value`; the base's settings fill what the layer leaves out; a layer off or at opacity 0 writes none). They are not values of the Skin model (a description can't set them in `materials`); the applier writes them as material value attributes like the rest, so a change re-applies live, with no bake and no texture made per character. The masks are content: three RGBA images in the model file (`extras.axcenity_skin_masks`), drawn by the Skin model (`docs/rendering/skin.md`).

Colours are sRGB `"#rrggbb"`. A value is checked against its slot's model (`check_values`): its name, kind and range. Customization baked for games (MetaHuman's approach) is a CO17/CO18 optimization; the description keeps the values.

## 11. Grooms (ADR-0037)

One groom item, three tiers: **cards** (a glTF mesh of alpha-clipped cards, every GPU, spring bones *(CO11)*), **strands** (an optional `*.groom` file, drawn by a compute pipeline on capable GPUs *(CO14)*), **mesh** (a solid fallback for crowds and far LODs). A groom must have cards or a mesh; the runtime picks the best tier the machine's settings allow and falls back to cards, then the mesh. Facial hair is a groom in a face slot, bound to the face.

**The groom file** (`*.groom`, little-endian binary, made by the editor-only converter from USD `BasisCurves` or Blender hair curves, ADR-0027) *(CO11)*:

| Part | Layout |
|---|---|
| Header | magic `AXGROOM\0` (8 bytes); `u32` version (1); `u32` flags (bit 0: has guides); `u32` strand count S; `u32` point count P; `u32` guide count G; `f32[6]` bounds (min, max; metres, glTF axes, the base's rest pose) |
| Strands | `u32[S + 1]` first point of each strand (the last is P) |
| Points | `f32[3 × P]` positions; `f32[P]` widths (metres) |
| Roots | per strand: `u32` body triangle, `f32[2]` barycentric coordinates (bound to the scalp of the item's first base) |
| Attributes | per strand: `u16` clump, `u16` reserved, `f32` melanin variation (added to the material's `melanin`) |
| Guides | `u32[G]` strand indices simulated as guides |

Strands interpolate from guides with curl and clumping at runtime *(CO14)*; cards are generated from the same strands offline *(CO11)*, so one groom drives every tier.

## 12. The skeleton

Today's contract stays: Unity/VRM humanoid names, 45° A-pose, 1.75 m = 5 studs, +Z front, at most 4 influences per vertex ([default-humanoid.md](../../research/default-humanoid.md) D3–D6). Every clip, retarget profile, state machine and face profile depends on it.

**Added bones** are only ever added, never renamed, and listed in `skeleton.added_bones` as `{name, parent}`. No clip keys them. `anim` turns them from the bones they read (`crates/anim/src/twist.rs`; its module docs hold the rules in full), at one call site (`skinned` in `crates/anim/src/animators.rs`): after everything that poses a bone (the clips, the identity's bone changes, body sliders, IK, the feet, the ragdoll) and before the skin is posed. So every clip plays unchanged, and a character's parts get the added bones through the pose by bone name (section 14).

| Kind | Bones (each with `Left` and `Right`) | Parent | Rule |
|---|---|---|---|
| Twist | `UpperArmTwist`, `UpperLegTwist` | The limb's upper bone (`UpperArm`, `UpperLeg`) | Takes half of its parent's own twist about its length back out (share −0.5), so the skin by the shoulder or hip turns half as far as the limb |
| Twist | `LowerArmTwist`, `LowerLegTwist` | The limb's lower bone (`LowerArm`, `LowerLeg`) | Takes half of the hand's or foot's twist about the limb's length (share 0.5) |
| Helper | `ShoulderHelper`, `ElbowHelper`, `HipHelper`, `KneeHelper` | The parent of the bone it reads (`Shoulder`, `UpperArm`, `Hips`, `UpperLeg`) | Stays on the joint and turns half as far as the bone it reads (`UpperArm`, `LowerArm`, `UpperLeg`, `LowerLeg`), which holds the joint's width when it bends |

A skeleton is driven **by these names**: one with none of them (the stylized humanoid) is untouched, and no manifest field describes the rule yet. A base model that has bones of these names must be weighted for these rules, as `axcenity/realistic` is (its QA scripts turn the bones the same way; change both together).

**Base models share bone names, not bone lengths.** A take whose bones are named like the model's binds by name, except when it would reshape the body: it keys bone offsets (translations), its skeleton rests at other offsets than the model's (a humanoid role's bone more than 5% different) and both rest in the same pose (arms within 10°). Then it plays through humanoid roles, retargeted (`reshapes` in `crates/anim/src/clip.rs`) without matching rest poses, so the model keeps its own bone lengths and rest directions; such a take moves role bones only. That is how the starter animations, made on the stylized skeleton, play on the realistic body.

## 13. Facial animation (addition A4)

The face is its own subsystem on the existing runtime (F1–F7, [ADR-0028](../../../wiki/Architecture/Architecture-Decisions/ADR-0028-Facial-Animation-Morph-Targets-On-GPU-And-FaceControls.md)), kept separate from identity:

| Layer | Contract |
|---|---|
| Identity (who the face is) | Base manifest parameters in the `face` category (eyes, brows, nose, mouth, jaw, cheeks, ears, head); identity shapes are worn live on the shared mesh (ADR-0041, which supersedes D4), or baked on request (`gameplay::description::bake`) |
| Controls (what it does) | `FaceControls`' 48 standard controls, mapped to the head's shapes and bones by the profile `face.profile` names (`axcenity`, `arkit`, `vroid`, or a new head's `*.face.json`) |
| Blend shapes | Expression shapes on the body mesh (the head is part of it) and on the face's parts (brows, lashes), named the same so parts follow (section 14) |
| Eye movement and eyelids | `face.eye_bones` turn by gaze; lids by `Blink`/`Eye_Closed` shapes that must close fully (CO6's check) |
| Jaw and lip sync | `face.jaw_bone` and the viseme map (F5) |
| Expressions and animation-driven poses | F4's layers (expressions, overrides, blinks, gaze, idle) and clips' face tracks, on the controls |

A new head plugs in by a profile; nothing in the description stores expressions. Wrinkle maps by expression are CO17.

## 14. Parts follow the body's shapes

A part (a follower MeshPart inside the skinned body) takes the body's pose by bone name, the body's bone changes with it. It also takes, **for every live blend shape it has, the weight of the body's shape of the same name** each frame (`anim`'s `follow`); shapes the body lacks keep the part's own weight. That is how brows and lashes blink with the face. A body may carry an **empty** shape so that a part's shape of the same name is driven (the tongue's and the pupil's, CO6): the weight is the body's, the geometry the part's. The eye and jaw turns a face makes are in the pose parts take, so eyes and teeth that are parts look and open with it. **Identity** shapes reach a part live (ADR-0041): each part takes the body's weights by name like any live shape, then wears its own identity (the body's, plus its item's own parameters), so a `fit: shapes` garment's own `Frame_Feminine` and `Build_Heavy` follow the body's weights with no copy of the mesh; an explicit bake derives every part with its identity instead. Binding-fitted items carry the body's identity and corrective shapes too, transferred at build or import (CO9, section 9), and follow the same way. The renderer applies each part's weights and joints once per frame (CO8's posing pass, per skin and mesh), and every pass and ray tracing draws that one posed copy.

## 15. The content library (ADR-0034)

A folder with `catalogue.json` at its root. The repository's is `assets/characters/`. **A project's is its own `assets/characters/`** (`avatar::LIBRARY_FOLDER`), which is what the applier resolves against (`gameplay::description::library`, read once and again when the catalogue changes). The default character's package is embedded in the engine: `project::add_character_library` writes its manifests, model and a catalogue into a project, files already there kept (`project::built_in_character_library` is the same library in memory); `project::add_default_humanoid` calls it, so a new project with a character, and Insert > Character, have the library. A project made before CO3 gets it from Insert > Character. **Since R10 (OPEN-22, decision 7) a project gets both families**: `project::add_default_game_character(project, shipped)` writes the built-in package and copies the engine's shipped packages beside it (`add_both_character_libraries`; the shipped folder is found by `project::shipped_character_library`, the working folder or a folder above the program), then writes the `Character` prefab as the **realistic** default character, `project::described_character`: the default humanoid's Model with a `HumanoidDescription` named `HumanoidDescription` in its Humanoid (`Base` `axcenity/realistic`, `Data` the family's default character, `project::game_character_description`), which the applier puts on wherever the world is loaded. New projects (`create_project_from`) and the editor's Insert > Character use it; Insert > Character also applies the description at once, in the same undo step. Without the shipped folder the prefab stays the stylized character. Files already there are kept, so an older project keeps its prefab and its characters and only gains the realistic family. *(CO9)* The editor installs the repository's library as the global character library and a project copies what it uses, so games ship only what they wear.

```text
assets/characters/
  catalogue.json                  the index (catalogue.py writes it; a test fails when it's stale)
  catalogue.py
  humanoid/                       the stylized base model package
    Humanoid.base.json
    HumanoidParts.glb …
    items/tee.item.json …         its items (they share HumanoidParts.glb)
  realistic/                      (CO4) the realistic package: <Name>.base.json, glTF, items/…
```

**`catalogue.json`:** `format_version` 1; `bases`: `[{id, manifest, label, family}]`; `items`: `[{id, manifest, label, kind, slot, tier, bases, tags?, thumbnail?}]`; `presets` (R1, left out when empty): `[{id, file, label, kind, base, tags?, thumbnail?}]`, all sorted by ID, `manifest`, `file` and `thumbnail` relative to the library. `Library::read` reads every preset it lists. An older engine that writes a project's catalogue again drops `presets` (section 4); the files stay where they are. It is an index made from the manifests (`avatar::Catalogue::of` makes the same one), so the browser can list a large wardrobe without opening every manifest.

## 15a. Presets (R1)

A preset file (`*.preset.json`, `avatar::PRESET_SUFFIX`) is a **format 2 description with a `preset` block**, so no new parser reads it and any engine checks it against the library:

```json
{
  "format_version": 2, "name": "Athletic", "base": "axcenity/realistic",
  "parameters": { "muscle": 0.4, "fat": -0.1 },
  "preset": { "id": "axcenity/realistic-body-athletic-male", "kind": "body",
              "about": "Trained but not bulky.", "tags": ["build", "male"], "order": 30 }
}
```

`name` is the preset's label. The block rides in `Description::extra` (`Description::preset` reads it as `PresetInfo {id, kind, about, tags, thumbnail?, order?}`). `kind` says what applying it sets; everything else is left alone:

| `kind` | Sets | Left out of the file means |
|---|---|---|
| `body` | Every parameter of a `body` group | Its default. A parameter with `choices` (the frame) changes only when the file names it |
| `face` | Every parameter of a `face` group | The default: a face preset is a complete face (brief §5.1) |
| `hair` | The items in `hair` slots (with variant, materials, parameters), and `hair` group parameters | Bald |
| `outfit` | The items in `clothing` and `accessories` slots, except the slots a parameter's choices fill (`chosen_slots`) | Nothing worn there |
| `skin` | The base's `materials` values and `skin` group parameters | Neutral |
| `character` | Everything but the name (base, parameters, items, materials) | As a description |

**One function**, `avatar::apply_preset(&mut Description, &preset, &Library) -> Result<(), Problem>`, is the rule for the studio, scripts and Randomize. It never copies the preset's `extra` (the block can't leak into a saved character), keeps the character's own `extra` and name, and puts the choices' items right afterwards (`wear_choices`). It refuses, changing nothing, a description without a block, a kind it doesn't know, or (but for `character`) a preset of another base. `matches_preset` says whether applying it would change nothing (the studio's "Custom" is no preset of a kind matching). `check_preset` holds a preset sound: its block (an ID, a sentence, a path), the description against the library, and nothing set outside what its kind owns; `Library::validate` runs it on every preset.

**Shipped presets** are written by `assets/characters/realistic/presets.py` into `realistic/presets/`: seven body presets per frame (brief §4.1's Slim, Average, Athletic, Muscular, Heavyset, Lean, Broad, from `references/body/README.md` section 6; tagged `male` or `female`, never naming `frame`), the nine face presets of brief §5.1 (`references/face/README.md` section 3) and the six starting characters the editor had in code (kind `character`). A person's own presets are the same files in the workspace's `presets/` folder; favourites are editor state. `project::add_character_packages` and `packaging/stage.py` copy preset files with the rest of a package. The test `every_shipped_preset_is_sound_listed_and_resolves` holds them.

## 15b. Randomize (R1)

`avatar::randomize(&Description, &Scope, locks: &BTreeSet<String>, seed: u64, &Library) -> Description`, in `avatar` because games randomize NPCs too (section 0, decision 6).

- **Scope**: `All`; `Category(category)` (parameters of its groups and its slots); `Feature(id)` (parameters whose feature it is); `Parameters(ids)`. Only what it names changes; the name and everything else is kept.
- **Locks** are parameter IDs and slot IDs that never change.
- **Values** are drawn evenly inside `random` (`random_span`: the middle half of the range when absent), rounded to a thousandth. A parameter with choices takes one of its choices, so `All` and `Category(Body)` can switch `frame` (male and female); the Studio should lock `frame` by default (RA1). The base's constraints come next: every `follow` first (a target that wasn't drawn or locked moves with its drawn source, held in range), then every `max_sum` (what changed, drawn or moved by a follow, is scaled down; locked and out-of-scope values stay), so the limit holds whatever order the manifest lists them in (test `randomize_keeps_max_sum_and_moves_follow_targets`).
- **Items**: each slot in scope (not locked, not a choice's slot) gets one of the items that fit the base there, or nothing, drawn evenly; never one that conflicts with what is worn (`excludes`), needs what isn't (`requires`) or also fills a slot already taken. A `multiple` slot draws up to two. A drawn item gets a random variant and random values for its own parameters. Material values are left alone (skin is R5's).
- **Deterministic**: a small in-crate SplitMix64 generator, so a seed gives the same character on every machine. 1,000 seeds on every shipped base and on the test library validate and resolve with nothing left out, and locks hold (tests).

## 15c. Character Studio's editing contract (R1)

The workspace's panels never change the character themselves. Each draw function reads the workspace and returns `Intent`s (`editor/src/character_workspace/intent.rs`: set a parameter, wear, pick a category or feature or level, focus a view, undo, redo, save, reset a scope, apply a preset; since R1's second part also save as preset, toggle a favourite, randomize a scope, lock a value, hold the current character, show or keep or discard a held one, side by side, and recover or discard the autosaved one; since wave 3 the asymmetry tools, `Sides { tool, of }`, which mirror a side onto the other or reset a side for a feature's left and right pairs at a level or for one pair, one undo step each (`sides.rs`), and `PreviewMotion`, the preview's pose and motion: Still, Idle, Walk, Run or a gesture, `motion.rs`). Only the shell (`shell.rs`) applies them. `edit.rs` holds the history: snapshots of the description, with transactions (begin, commit, cancel) so a drag is one undo step, and reset by scope (everything, a category, a group). `selection.rs` holds what is being shown: the category, the feature and the level (Quick, Detailed, Advanced, read from each parameter's `level`, `feature` and `about`, §6). Selection, levels, locks, favourites (`favourites.json`), held versions and the second character side by side (`compare.rs`), the autosave (`recovery/character.json`, `autosave.rs`), the preview's expression and pose and motion (the Walk take at its own speed, a clip written to the workspace's `assets/animations/Humanoid/Preview/`, never to a character), the camera and the stage are editor state:
 none of them is ever written into a description (A3). Comparison (CO19) goes through the same `Intent`s. So does **direct manipulation** (R2, `direct.rs`): a click on the character sends `PickFeature` (the navigation's own intent; the shell moves to the feature's category) and `FocusFeature` (the camera frames a feature that is small on the screen); a handle's drag sends the slider's `SetParameter`, or with Symmetry `SetMirrored`, as one gesture (`Drag` until the release's `Commit`), so a handle and a slider produce the same description and a drag is one undo step; `SetHandles` is the switch (editor state). Which feature is under the pointer comes from a ray against the skin as drawn (`scene::ray_parts` and `ray_model`: the frame's pose, morphed and skinned on the CPU once per pose) and the manifest's features: the smallest feature whose tint covers the point hit, else the body region hit (the build's region map, `TEXCOORD_1`, read by `assets` into `Primitive::regions`) picks a feature that names it, and a smaller feature whose anchors are near (within 1.2 of its radius, at rest) wins over it; a handle under the pointer names its own feature. The tint and the handles are drawn by the editor over the preview, which has no depth test there, so the studio rasterizes the posed character into a small CPU depth buffer each frame (`direct/depth.rs`) and draws only what it sees: a handle behind other skin is neither drawn nor grabbed. Drawing the tint as draw data depth-tested in `render` (ux patterns section 3) remains the long-term route.

## 16. Rust API (`axcenity_avatar`)

```rust
// Files
Description::from_json(&str) -> Result<Description, FormatError>   // migrates format 1
Description::to_json(&self) -> String                               // deterministic, trailing newline
Description::new(name, base) -> Description
Description::validate(&self, &Library) -> Vec<Problem>
BaseManifest::{from_json, to_json, validate, check_model(&self, body: &ModelContents, parts: &[ModelContents])}
ItemManifest::{from_json, to_json, validate, check_against(&self, &BaseManifest), check_model(&self, item, body), models()}
Catalogue::{from_json, to_json, of(&Library)}
Library::{new, add_base, add_item, base, item, manifest_path, path_of, read(root: &Path) -> (Library, Option<Catalogue>, Vec<Problem>), validate}
ModelContents::of(&axcenity_assets::Model)                          // shapes, bones, materials by name
// R1 contracts
Parameter::level() -> Level;  ItemParameter::level() -> Level     // Level::{Quick, Detailed, Advanced, Other}
BaseManifest::{group, feature, handle, category_of(&Parameter), feature_of(&Parameter)}
ItemManifest::{parameter, slots(), conflicts_with(&ItemManifest)}
Feature { id, label, category, about, icon, anchors: Vec<Anchor { bone, offset }>, radius, regions }
Handle { id, feature, label, mirror, anchor, drives: Vec<Drive { parameter, axis, travel, screen, sensitivity, frames: Vec<FrameDrive { frame, axis, travel }> }>, views }
MaterialSlot { name, shading, label, values, role: Option<MaterialRole> };  MaterialSlot::{new, offered}
WornItem { id, variant, materials, parameters, transform: Option<Transform { offset, turn, scale }>, extra }
Library::{add_preset, preset, preset_path, presets};  CataloguePreset
Description::preset() -> Option<PresetInfo>;  PresetKind;  PRESET_SUFFIX
apply_preset(&mut Description, &Description, &Library) -> Result<(), Problem>
matches_preset(&Description, &Description, &Library) -> bool;  check_preset(&Description, &Library) -> Vec<Problem>
randomize(&Description, &Scope, &BTreeSet<String>, seed: u64, &Library) -> Description;  random_span(range, random)
// Resolution
parameter_values(&BaseManifest, &Description) -> BTreeMap<String, f32>
body_shape_weights(&BaseManifest, &Description, &Library) -> BTreeMap<String, f32>
span(value, from, to) -> f32
// Materials
ShadingModel::{values, value}; MaterialValue::{Number, Colour}; Colour::parse
```

**The runtime (CO3):**

```rust
// avatar
resolve(&Library, &Description) -> Result<Resolved, Problem>       // Err: no such base model
Resolved { base, body: ResolvedPart, parts: Vec<ResolvedPart>,     // base parts, then items as worn
           shapes, bones: BTreeMap<String, BoneChange>, identity: Identity,
           attributes, materials, problems }                      // items left out, and why
ResolvedPart { name, item, slot, model /* MeshId relative to the library */, fit: PartFit, materials,
               shapes /* the item's own parameters' shape weights (R1) */, transform /* a socket item's (R1) */ }
PartFit::{Skinned, Socket(name)}
LIBRARY_FOLDER                                                     // "assets/characters"
// assets
Identity { layers: Vec<IdentityLayer { shapes: Vec<(String, f32)>, bones: Vec<BoneChange> }> }
Identity::key() -> u64;  Model::bake(&Identity) -> Model;  Model::settle(&mut pose);  Model::settles()
Model::live(&Identity) -> LiveIdentity;  LiveIdentity::{settle, settles, dress(&mut Posed), weights, is_empty};  Posed::rebind(&[Mat4])   // R2a
Assets::register_identity(&Arc<Identity>) -> Option<u64>;  Assets::identity(key);  Assets::derived_count()   // R2a
Assets::derive(mesh_id, &Arc<Identity>) -> String;  Assets::identity_of(mesh_id)   // the optional bake
derived_mesh_id(base, key);  split_derived(mesh_id) -> (base, Option<key>)
// gameplay::description
apply_description(&mut World, &Assets, character, &Description) -> Result<Applied, String>   // live identity
bake(&mut World, &Assets, character) -> Result<usize, String>;  is_baked(&World, character)   // R2a: the optional bake
Applied { body, parts, description, problems }
library(&Assets) -> Arc<Library>;  body_of;  applied_instance;  description_of;  default_description;  write_description
DescriptionSync::{note, forget, step}                              // applies kept descriptions after load and replication; forget: apply again (R10)
DESCRIPTION_PART                                                   // the attribute marking a part the applier owns
// anim
IDENTITY;  identity_key(&World, part) -> Option<u64>;  set_identity_key(&mut World, part, Option<u64>)   // R2a
// project
add_character_library(project);  built_in_character_library();  CHARACTER_LIBRARY_PATH
// core
ClassName::HumanoidDescription { Base, Data };  material_value_attribute(material, value);  MATERIAL_VALUE_PREFIX
```

**What applying writes into the World:** the character's `Body` MeshPart gets the base body's `MeshId` (kept where the old model's skeleton stood, `anim::ragdoll::refit`), the attribute `Identity` (the key of the identity it wears live; R2a) and the body slider attributes; each part is a MeshPart inside the `Body` named after its object, with its base `MeshId`, its `Identity` and the attribute `DescriptionPart` (its item's ID, or `base:` and its model for the base's own parts and the `Body`); a `Body` with no Animator gets one, and an Animator with nothing to play poses a body that has parts at rest, so a still character wears its parts; parts already worn are kept, parts no longer worn are destroyed, and MeshParts without the attribute are the creator's own and left alone (one named like a library part is taken over, so the default humanoid's hand-placed parts become applied ones); a socket item gets an Attachment named after its socket, so `anim`'s sockets hang it; material values become `Mat_…` attributes; the description is kept as a `HumanoidDescription` inside the character's Humanoid (or named `AppliedDescription` inside a character without one). `DescriptionSync` applies kept descriptions that this run hasn't applied, or whose `Data` changed, once a frame from the game systems' step, so a loaded or replicated character registers its identity on each device (a character that was baked is baked again). The Game Editor steps one too in edit mode (R10, OPEN-21; `crates/editor/src/app/edit_characters.rs`), with no game system: it applies the kept descriptions and poses the described characters at rest (a copy of each in a World of its own, its Animators emptied, posed by `anim`'s Animators), so a saved described character shows its identity, items and materials in the viewport. It keeps what each character showed (body, model, identity, parts) and calls `DescriptionSync::forget` when a recorded step (an undo to a whole-world snapshot, Revert all) put back the unshown character with the same `Data`; a placed prefab copy is compared with its source shown the same way, in memory, so what edit mode derives is never a prefab override. `bake` swaps each owned MeshPart's `MeshId` for its derived one and removes its `Identity`; applying again undoes it.

**Fitting (CO9, `avatar::fit`; ADR-0041 decision 4 moved fitting from run time to build and import time):**

```rust
BodyMesh::from_model(&Model) -> Result<BodyMesh, FitError>        // a skinned model in its file space, its shapes
GarmentMesh::from_model(&Model);  Surface::new(positions, normals, triangles).closest(point, within) -> Option<Hit>
bind(&Surface, &[Vec3], &[Vec3]) -> Binding                         // garment points and their normals (empty: closest points)
place(&Binding, positions, normals, triangles) -> Vec<Vec3>
carried_shapes(&BaseManifest, &BodyMesh) -> Vec<String>          // identity (parameters, corners) + Corrective_*, no Mask_*
fit_garment(&BodyMesh, &GarmentMesh, &FitOptions { margin, shapes, under: Vec<Layer>, extremes, rounds, poses, pose_states }) -> Result<Fitted, FitError>
    // weights from the skin under each corner (both sides of a crease), the bound point's where none; normals turned with the push-out
Fitted { garment, joints, weights, shapes: Vec<CarriedShape>, binding, report: FitReport { layers: Vec<LayerOffset>, inside_most, .. } }
triangle_regions(&BodyMesh, &RegionMap) -> Vec<u8>;  coverage(&BodyMesh, &regions, &Surface) -> Coverage
hidden_regions(&BaseManifest, &Coverage) -> Vec<String>;  classify(&BaseManifest, &Coverage) -> Option<SlotGuess>
write_fitted_glb(&FittedModel) -> Vec<u8>;  thumbnail(&GarmentMesh, size) -> rgba
extremes_of(&BaseManifest, &Library) -> Vec<Vec<(shape, weight)>>  // the sliders check's pairs of build ends, and body presets, both frames
extremes_near(&BodyMesh, &GarmentMesh, &extremes) -> extremes        // only the shapes that reach the garment; duplicates dropped
fit_poses(&Rig, Option<FinishPose>) -> Vec<FitPose>                  // held poses, arms hanging, walk swings; with the body's corrective weights
pose_states_of(&BaseManifest, &Library) -> states                    // frames, the build's muscle/fat corners, body presets
// posed fit: in each pose a corrective leans on, the garment's copies of the
// leaning correctives take what linear skinning leaves between the cloth and
// where its binding puts it on the posed skin (FitReport.posed)
align(&mut GarmentMesh, &BodyMesh) -> Alignment { scale, offset, units, spread_before, spread_after }   // units, rough place, fine fit
import_garment(&ImportRequest) -> Result<Imported, FitError>     // align, fit, classify, thumbnail, write, list in the catalogue
// assets (region hiding)
RegionHiding { map, hidden };  Assets::hide_regions(mesh_id, &Arc<RegionHiding>) -> String;  Assets::hiding_of(mesh_id)
hidden_mesh_id(base, key);  split_hidden(mesh_id);  strip_tags(mesh_id);  Model::without_triangles(&dyn Fn(uv) -> bool)
```

The command line: `cargo run -p axcenity-avatar --release --example import_garment -- <garment> <library> <base> <id> <label> <folder> [--under <id>]...`.

## 17. Lua API (built in CO3, Roblox-familiar names)

| Member | Signature | Does |
|---|---|---|
| `HumanoidDescription` | Instance | Holds a description: `Base` (string), `Data` (the JSON text, replicated) |
| `HumanoidDescription:GetParameter(id)` / `:SetParameter(id, value)` | number | Read and change a value |
| `HumanoidDescription:GetItems()` / `:SetItem(slot, id, variant?)` / `:RemoveItem(slot)` | | The wardrobe |
| `HumanoidDescription:SetMaterialValue(slot, name, value)` | number or `Color3` | Material values |
| `Humanoid:ApplyDescription(description)` | | Resolves and applies it to the character (parts, shapes, bones, attributes, materials) |
| `Humanoid:GetAppliedDescription()` | `HumanoidDescription` | A copy of what is applied |
| `Players:CreateHumanoidModelFromDescription(description)` | `Model` | A character from a description (NPCs, previews) |

## 18. Where each stage plugs in

| Stage | Uses | Adds (optional fields need no version bump) |
|---|---|---|
| CO2 QA harness | Descriptions as the input of every capture | |
| CO3 applier (built) | Sections 5–7, 10, 14, 17 | `Resolved`, live identity (registered identities, the `Identity` attribute; R2a, ADR-0041) and derived models for an optional bake, `HumanoidDescription`, material value attributes |
| CO4–CO6 realistic body and head | Sections 6, 7, 12, 13 | `assets/characters/realistic/`, its base manifest, twist and helper bones (CO4, built); body shapes and `correctives` (CO5); the face (CO6) |
| CO7 shading | Section 10 | Renderer shading models; no format change |
| CO9 clothing | Sections 8, 9, 15 | Binding files, region maps, the import flow, thumbnails |
| CO11 hair | Section 11 | The groom file and converter |
| CO12 studio | Sections 6, 7 (`follow`), 15 | Panels from the manifest; writes format 2 |
| CO13 presets, network | Sections 5, 15a | Preset files (format 2 descriptions with a `preset` block, built in R1), replication of `Data` |
