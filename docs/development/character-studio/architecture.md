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

Ships beside a base model's glTF file. Since 2026-11-24 there is a third, `assets/characters/human/Human.base.json` (`axcenity/human`, family `human`: MakeHuman's body on the Humanoid skeleton with its four twist bones, model `HumanParts.glb`, regions by `map_index` in `HumanRegions.png`; ADR-0047, the package's `README.md`), which is to replace the realistic one. The stylized one is `assets/characters/humanoid/Humanoid.base.json` (generated by `manifests.py` from `Humanoid.customization.json` and `HumanoidParts.glb`; edit the script, not the file). The realistic one is `assets/characters/realistic/Realistic.base.json` (`axcenity/realistic`, written by that package's `manifests.py` when `build.py` runs): model `RealisticParts.glb`, body `AxcenityRealistic/Body`, parts `Eyes` and `Teeth`, 16 added bones (section 12) and 14 regions by `map_index`, stored as `RealisticRegions.png` and as the body's `TEXCOORD_1` (`assets/characters/realistic/README.md`), plus three regions with mask shapes, one under each underwear item (`under_boxer_briefs`, `under_briefs`, `under_sports_bra`: an interim exception to section 9, below).

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

**Feature** (R1; plan section 11.3): `{id, label, category, about?, icon?, anchors: [{bone, offset}], radius, regions?}`. An anchor is a bone and an offset from its head in model space at rest (metres, glTF axes): it follows bone changes. One anchor for a middle feature, two for a pair (left first). A click picks the feature whose anchor is nearest within `radius` (metres), and the camera frames its anchors; `regions` are body regions that also pick it. The realistic body has 24 (`body` for the whole body; `head`, `neck`, `shoulders`, `chest`, `arms`, `hands`, `abdomen`, `waist`, `back`, `hips`, `thighs`, `calves`, `feet`; `face_shape`, since 2026-11-24 the first of the face's: the outline, anchored on the front of the face and naming the `head` region, so it is lit as the whole head and doesn't share the face's skin with the features below; `eyes`, `brows`, `nose`, `cheeks`, `jaw`, `chin`, `lips`, `forehead`, `ears`), measured on the model by `assets/characters/realistic/anchors.py`: the weighted centre of the vertices the feature's shapes move (or of its region in the region map), on the skin for the face's features and at the centre of the part for the body's, following the bone that carries most of them, on the masculine frame. Their placement on the feminine frame is not checked yet.

**Handle** (R1 schema; R2 writes them and Character Studio uses them): `{id, feature, label?, mirror?, anchor: {bone, offset}, drives: [{parameter, axis, travel, screen?, sensitivity?, frames?}], views?}`. A handle only names parameters: no second geometry state (brief §3.4). Its anchor is a point of the skin at rest (a bone and an offset from its head, as a feature's): Character Studio finds the skin's corner there (`scene::nearest_corner`, a corner of the skin-shaded primitives) and draws the handle on that corner where the frame's pose puts it (morphed, skinned, the identity worn live), so it stays on the skin at every value. A drive's `axis` (model space, glTF axes) and `travel` (metres) are how that corner moves from the parameter's lowest value to its highest: the studio draws the drive's arrow, and takes a drag along it, in the direction that move shows on the screen; how far a drag goes is the studio's one rule for numeric controls, a parameter's whole range in 110 points of drag whatever the zoom (RA1, Paul's rule of 2026-11-18, `character_workspace/numeric.rs`), so the skin under the handle doesn't follow the pointer one to one, and `sensitivity` (which divides the travel) no longer changes the gain. `frames` `[{frame, axis, travel}]` (wave 3) lists the drive on a value of the `frame` parameter whose own shapes move the corner otherwise (the realistic waist's `<Frame>_Waist_*`: 37 mm on the feminine frame against 48 mm on the masculine); `axis` and `travel` are the default frame's and hold on any frame not listed within half a frame (`Drive::on_frame`). `handles.py` lists a frame when its move differs by more than 1 mm. `screen` `{view: [dx, dy]}` lists the framings the drive works in (`front`, `three_quarter`, `side` as the Side button frames it, the character's right profile, and `back`), each with the direction the corner moves on the screen there (x right, y up); a framing where less than 35% of the travel shows, or where a drive that shows more of its travel has the same line (within 25 degrees), has none. `views` are the framings where any drive works. `mirror` names the other side's handle, each naming the other, on the same feature. The realistic body's 59 handles are written by `assets/characters/realistic/handles.py`: the first 20 (nose tip, bridge, nostrils; each eye's upper lid and outer corner; the jaw's corners; the mouth's corners and both lips; each shoulder, side of the waist and hip) and, since 2026-11-24, 39 more for the face: a ring of 14 round its outline on the `face_shape` feature (the forehead's top, each temple, cheekbone edge, cheek, jaw corner, side of the jawline and corner of the chin, and the chin), seven more on the nose (root, hump, each side of the bridge and of the tip, the columella; a nostril's handle also drives its height and, in profile, its flare), and handles for the skin under each eye, each inner corner, the cheeks, the jawline, the chin, the forehead and the temples. A handle's drives are its own feature's parameters, **except the outline's ring**, whose handles drive the outline's parameters and those of the features the outline passes (`forehead_height`, `temple_width`, `cheekbone_height`, `jaw_angle`, `jaw_definition`, `chin_width`, `chin_shape`, `chin_forward`): any parameter of a `face` group (`avatar`'s `realistic_handles` test holds both rules). A handle's corner is found by a rule: the corner a parameter moves furthest (`moved`, `middle`, `top`), and since 2026-11-24 `outline` (the face's outline seen from the front at the height of the corner a parameter moves furthest, off the ear), `edge` (the outer edge of what a parameter moves), `between` (the skin's corner part of the way from one rule's corner to another's) and `toward` (the middle-line corner most nearly in a direction from a bone); a handle's `yields` names drives that take a framing only when no other drive of the handle has their line there (the nostril's flare, beside its width). The script measures every corner and travel on the built model, working the proportion sliders' bone changes out as `anim`'s body sliders make them; the player's `handle_tests.rs` holds each travel to the applied skin's within 5% (they agree to 0.1 mm), each anchor within 2 mm of a skin corner at rest, and, at both ends of every drive's parameter on both frames, the handle the data predicts (the anchor where the starting description draws it, plus `axis` × `travel` × the share of the range the value moved) within 2 mm of the applied skin's triangles (each frame with its own drive; at most 1.7 mm). The anchor is carried by its skin corner, not by its bone alone: the feminine frame's shapes move the face up to 22 mm and the shoulders 36 mm off where their bones carry them, and a hip corner shared with the thigh's twist bone sits 5 mm off its bone's carry on either frame. An eye's outer-corner handle sits at the outer canthus (the lid margin's most lateral corner at the eye's height, then the outward-facing skin corner beside it).

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

**Pose-driven corrective shapes (CO5).** `correctives`: `[{shape, bone, axis, angles: [start, full]}]`. Such a shape's weight is `smoothstep(clamp((θ − start) / (full − start), 0, 1))` (eased at both ends, `t²(3 − 2t)`, since wave 4: a straight ramp started and stopped the shape at full speed, and cloth carrying a large copy of it jolted in that frame; `anim::correctives::ramp`, `avatar::fit::corrective_weights` and the QA scripts' `drive_correctives` agree), where θ is the twist, in degrees, about `axis` of the bone's turn from rest (its local rotation times the inverse of its rest local rotation, so in its parent's space). `axis` is a unit vector in the space of the bone's parent at rest; several entries may name one bone. A shape named here is driven by no parameter and stays live whether the identity is worn live or baked. Today the realistic body's ten (`Corrective_Shoulder_Level_*`, `_Raised_*` and `_Lowered_*`, on the upper arms; lowering is a turn about the raise axis reversed, so every rule has `start < full`; and, since wave 4 (CO9 part 2), `_Forward_*` and `_Back_*`, the hanging arm swung forward to 40 degrees and back to 50 about the axis across the body, on top of the lowered shape, which keep the skin from folding out where the arm meets the chest or the back in a walk) are written to `Realistic.build.json` and are constants in `anim` (`correctives.rs`), which a test holds equal to the build data; the manifest field takes their place when a second body needs other rules.

**Identity versus live shapes (ADR-0041, refinement R2a; supersedes D4 and ADR-0038).** Every shape a parameter, corner or region drives is **identity**: constant until the description changes. A character on screen is its base model's **shared mesh** with the identity **worn live**: the applier registers the resolved `Identity` in `Assets` (`Assets::register_identity`, keyed by `Identity::key`, a hash of its names and numbers, the same on every machine) and writes the key on the body and every part as the attribute `Identity` (16 hex digits, `anim::IDENTITY`; a part with item parameters gets the body's identity plus a layer of its own). `anim` works out once per model and identity how that model wears it (`Model::live` → `LiveIdentity`: the identity shapes' weights by index, the bone changes, and a correction per joint), adds the bone changes after the clips, and puts the weights and joint corrections on the pose last, after the face's and the correctives' (`LiveIdentity::dress`). Followers take the body's weights by name as for expressions, then their own identity. The result is the bake's to float precision (`assets` `an_identity_worn_live_poses_the_mesh_where_its_bake_does`; in pictures, the player's `an_explicit_bake_draws_the_same_pixels_as_the_live_character`). A key this run hasn't registered (a saved world before its description is applied again) wears nothing. **Baking is optional** (`gameplay::description::bake`): it gives each MeshPart the **derived model** of ADR-0038 (the base with the identity shapes added into its vertices and dropped from its blend shapes, carried surfaces moved, the bind on the changed skeleton; `…/HumanoidParts.glb#AxcenityHumanoid/Body#i3f9c…`, `Assets::derive`) and removes the `Identity` attribute; nothing bakes by default, and applying a description again makes the character live. The renderer morphs and skins each character **once per frame** (CO8, `render` `posing.rs`): a compute pass walks only the shapes whose weight isn't 0 and writes a posed buffer that the shadow, depth, motion and main passes read and ray tracing builds its BLASes from, so identity shapes cost once, not per pass, a shape at weight 0 costs nothing, and ray-traced shadows, AO, reflections and GI show identity and expression shapes (measured in [refinement/wave3/co8-compute-posing.md](refinement/wave3/co8-compute-posing.md); before, in [refinement/r2/r2a-live-identity.md](refinement/r2/r2a-live-identity.md)). **Live** expression and corrective shapes morph every frame as before. A manifest may add `correctives` later as an optional field (no version bump): `[{shape, bone, axis, angles: [start, full]}]`.

## 8. The item manifest (`*.item.json`)

One garment, accessory or groom. The stylized ones are `assets/characters/humanoid/items/`.

| Field | Type | Meaning |
|---|---|---|
| `format_version` | 1 | |
| `id`, `label` | ID, string | |
| `kind` | `garment`, `accessory`, `groom` | |
| `slot` | slot ID | A slot of every base it fits |
| `layer` | 0–3 | Garments only, the order outward for push-out: 0 next to the skin (underwear, and a top that lies on the body and is worn tucked into the bottoms: section 9, "Worn with the bottoms"), 1 base (shirts, trousers, shoes), 2 mid (sweaters, vests), 3 outer (jackets, coats) |
| `tier` | `built_in` (default), `premium` | The two content tiers (section 0, decision 2): generated by the engine's scripts, or artist-made or imported |
| `bases` | `[ID]` | The base models it fits |
| `model` | `{file, object?}` | Garments and accessories: the mesh. Grooms have none here |
| `hides` | `[region ID]` | The body regions it hides |
| `cover_map` | path | 2026-11-24 ([ADR-0045](../../../wiki/Architecture/Architecture-Decisions/ADR-0045-What-Is-Worn-Over-Skin-Or-Cloth-Hides-It-By-A-Cover-Map.md)): an image beside the manifest, over the **body's** first texture coordinates, made by the fit. Red (128 or more) is skin the body leaves out under the item; green is skin under which a garment worn beneath it leaves its cloth out (section 9, "Cover maps") |
| `materials` | `[{name, shading}]` | **Every** material of its model(s), with its shading model |
| `variants` | `[{id, label, materials}]` | Colour and material variants: values per material slot (glTF's `KHR_materials_variants` maps onto these on import *(CO9)*) |
| `fit` | `{method: "shapes", follows[]}`, `{method: "binding", data?, margin}`, `{method: "socket", socket}` | Section 9 |
| `groom` | `{cards?, strands?, mesh?}` | Grooms only: section 11 |
| `tags` | `[string]` | For the wardrobe browser's search and filters. `masculine`, `feminine` and `unisex` say who a garment is cut for (the Clothing page's "Cut for" choice; one that says nothing is for both) |
| `thumbnail` | path | For the browser *(CO9 makes them)* |
| `source` | `{author, licence, url?}` | Provenance; `licence` is an SPDX ID (`CC0-1.0`). Imports must fill it |
| `about` | string | R1: a sentence for the browser |
| `parameters` | `[ItemParameter]` | R1: the item's own sliders (a hairstyle's length): `{id, label, range, default?, about?, level?, random?, shapes?, material_values?, choices?}`, shaped like a base parameter without `group`, `mirror`, `feature`, bones or attributes. `shapes` name shapes of the item's model, `material_values` its own material slots; a choice wears nothing |
| `materials[].label`, `.values`, `.role` | | R1, as on a base manifest's material slot: what the slot is called, which values it offers (recolouring), its colour role |
| `variants[].thumbnail`, `.parameters` | path; map ID → number | R1: a picture of the variant; values for the item's parameters |
| `also_fills` | `[slot ID]` | R1: other slots it occupies (a dress or a jumpsuit: `bottom`; so nothing is worn there under it) |
| `requires` | `[item ID]` | R1: items that must be worn with it |
| `excludes` | `[item ID]` | R1: items it can't be worn with (either naming the other is enough) |
| `limitations` | `[{note, parameter?, range?}]` | R1: known limits, a note for the browser; with `parameter` (the item's or the base's) and `range` the editor can warn outside the range. A garment cut for one frame says so with a range on the base's `frame` (`[0.5, 1]` feminine, `[0, 0.5]` masculine) |
| `quality` | `draft`, `production` | CO9 (plan 11.3, [asset-quality.md](asset-quality.md)): only `draft` is a draft (`ItemManifest::is_draft`); an import is always `draft`. Randomize never draws an item that says `draft` (R6 and R7, wave 4); an item with `quality` unset (made before the standard) is still drawn and listed until it is reviewed (the lead's call at the wave-4 merge: `ItemManifest::is_draft` is true only for `draft`). Character Studio's Hair and Clothing browsers hide drafts unless "Show drafts" is on and mark them "Draft" (R6, R7, §15c); keeping them out of defaults is not built yet, and no shipped preset wears a draft (test `drafts_are_in_no_preset_and_never_drawn_at_random`) |
| `adjust` | `{offset?: [[min, max] × 3], turn?: [[min, max] × 3], scale?: [min, max]}` | R1, socket items only: how far a person may move (metres per axis), turn (degrees about each axis) and scale it. Each limit holds the item as made (0, 0, 1); a part left out can't change |

**Supported operations are derived, not listed** (brief §14.1): an item can be recoloured when a material slot offers values, has variants when it lists them, has sliders when it has `parameters`, can be moved when it has `adjust`. An `operations` list can be added later if something can't be derived.

**How Character Studio's browsers read it** (R6, R7; `editor/src/character_workspace/browser.rs`, `Declared`): a colour button per `role` (`primary`, `secondary`, `accent`), for the slots of that role that offer `tint`; a slot without a role is no control. On a groom the roles mean the hair's colour (`primary`), its **highlights** (`secondary`) and its **roots** (`accent`), and each such Hair slot that offers `melanin` gets the natural colours (`melanin` and `redness` together), so a style declares highlights or roots by having a slot with that role, and one that doesn't shows no such control. Variants are chips; `parameters` are slider rows at their `level`; `limitations` with a base parameter and a range drive the "Fits this body" filter. No new field: this uses `role`, `values`, `variants`, `parameters` and `limitations` as they are.

**Rules** (`ItemManifest::validate`, `check_against`): a garment needs a model and a layer; an accessory needs a model and no layer; only accessories hang on a socket; a groom has no `model`, has `groom`, and has **cards or a mesh** (strands are never the only tier); the slot, every hidden region and the socket exist on every base it fits; variant values suit the shading models. R1: item parameter IDs are unique, ranges go upwards and hold their defaults and `random`; material drivers drive number values of the item's own slots; a material slot's `values` are its shading model's; variant parameters exist and are in range; `also_fills` doesn't name the item's own slot and names slots every base has; `requires` and `excludes` are IDs, not the item itself, and the library has them (`Library::validate`); a limitation with a range names a parameter of the item or the base; only socket items have `adjust`. `ItemManifest::check_model`: item parameter shapes are shapes of the item's model. A description (`Description::validate`): worn parameters exist and are in range; an item without what it `requires` worn, or worn with one it `excludes`, is a problem; `transform` is inside `adjust` (no `adjust`: only the identity).

## 9. Fitting (ADR-0035)

| Method | How the item follows the body | Who uses it |
|---|---|---|
| `shapes` | The item carries its own copies of the named body shapes; the runtime gives each the body's weight by name (section 14) | The stylized humanoid's garments and hair (today's build) |
| `binding` | Each item vertex is bound to the body's surface (a triangle, the point's weights of its corners, and the vertex's offset in the frame of the triangle's smoothed normal and first edge), **made at build or import** (CO9, `avatar::fit`, ADR-0041 decision 4): through the binding the item takes the body's skin weights and **every identity and corrective shape of the body, as morph targets of the same names**, pushed clear of the body and of every lower layer by `margin` metres (default 0.002) at rest, at each shape's full weight and at the parameter extremes. At run time it is a skinned part like a `shapes` item: it takes the body's weight of every shape it has (section 14), so nothing is fitted or rebuilt when the description changes. `data` is the binding file (format 1: `{format_version, body, vertices: [{triangle, bary, local}]}`), kept for refitting | Every new garment (CO10) and every import (the test tee) |
| `socket` | Rigid on a socket; scaled by the socket bone's scale | Caps, glasses, watches |

**Intake: a garment that was not made on the body** (2026-11-24, [ADR-0044](../../../wiki/Architecture/Architecture-Decisions/ADR-0044-Garments-Not-Made-On-The-Body-Come-Through-An-Intake-Before-The-Fit.md); `avatar::fit::prepare`, `fit/intake/`). The binding fit expects one layer of cloth in the body's rest pose, at its size and within a few centimetres of it. A generated or downloaded garment is none of that: its own size and place, its own pose (arms nearly hanging), cut for another body, and a closed surface (a shell with an inner wall, or a solid with a shut pit at each opening). An import with `ImportRequest::intake` set runs `prepare` in place of `align`:

| Step | What it does |
|---|---|
| Winding | A shell wound inside out (its triangles enclose a negative volume) is turned |
| Reading (`slices.rs`, `structure.rs`) | 240 level planes cut it into loops; a loop's depth is how many others are round it. The **trunk** is the run of rings round the middle line with the most cloth (a ring is measured as a tape lies: its convex hull, so an open front or a slit reads), ended where it takes in the loops beside it (the underarm). Loops beside the trunk and farther out are **sleeves**; two side by side under it are **legs** (the crotch is where they part). A limb's middle line runs through its loops' middles |
| One wall (`shell.rs`) | Points on a loop of odd depth that is round and at least 0.5% of its ring (the inside of a tube, a pit at an opening) are the inner wall, with what joins them; a thin band of cloth (a strap, an open front) is split by which way it faces. The inner wall comes off; a hollow garment keeps one ring of it as the turned edge |
| Kind (`GarmentKind`: `top`, `trousers`, `skirt`, `one_piece`) | Guessed: legs make trousers, or a one-piece when it goes on above the trunk; without legs a top when it goes on above the trunk, else a skirt. Can be given |
| Pose | The body's upper arms and legs are turned so the wrists and ankles lie on the garment's cuffs and hems (only limbs at least 1.5 times longer than thick show a direction) |
| Size and place | **Top**: hangs from the shoulders (lowered until a tenth of its upward-facing cloth over upward-facing skin is at the skin); sized by its sleeves reaching the wrists when they are long, held between 2% and 70% chest ease, else by its chest and `ease` (`snug` 4%, `regular` 10%, `loose` 22%, `oversized` 38%). **Trousers**: the crotch 1.5 cm under the body's; sized by crotch to hem against crotch to 3 cm past the ankle when the legs are long, else by the seat with 6% ease. **Skirt**: its waistband at the body's waist, 3% longer round. **One-piece**: from the shoulders, the size at which its hems reach the ankles (or its crotch the body's). The body it is measured on is the one it was cut for (`cut_for`: body shape weights, such as the feminine frame), at rest |
| Back to rest | Each sleeve and leg is turned back about its joint, each corner by its share of the limb (1 on the limb, 0 on the trunk and the other limbs, settled between them); triangles the turn stretches 2.5 times (a cuff joined to the hip it touched) come out, and **the cuts are closed again** (`fit/mend.rs` `close_cuts`, 2026-11-24 evening): a sleeve modelled hanging is one wall with the garment's side, so taking the joins out leaves the sleeve open along its underside and the side open where the sleeve was; each open ring is closed from its ends inward between the corners of what came out, by triangles no longer than 20 cm whose corners are within half a share of each other on every limb, so the sleeve closes to itself and the side to itself, and they meet only under the arm. Then from the `cut_for` body to the default one: each corner moves with the skin nearest it, the moves smoothed over the cloth |
| Centre (`intake/centre.rs`, 2026-11-24 evening) | Each sleeve (and each leg of trousers or a one-piece) is brought round its limb. The turn back to rest moves a sleeve as one straight piece, so only its cuff comes to lie on the arm's line; a generated sleeve is bent at the elbow or hangs, and between the shoulder and the cuff it then lies beside the arm (measured on the puffer jacket: from the elbow down the whole sleeve was 2 to 14 cm above the arm's line, the forearm outside it). The limb's cloth is every piece of cloth, joined over the garment's own surface, that starts a fifth of the way along the limb, lies within 30 cm of its line and nearer it than the trunk's skin or another limb's line, and whose corners are 16 cm or nearer at their middle. It is cut into rings 2.5 cm wide along the limb's bones; each ring's middle (of its extent across the limb) is moved onto the line, the moves evened along the limb, faded in over 8 cm and smoothed over the cloth. A limb whose rings would move more than 15 cm is left alone. `IntakeReport::centred` is the farthest a corner moved |
| Conform (`intake/conform.rs`, ADR-0045) | Drawn in round the body by how the kind of garment is worn (`Conform`: `structured` keeps 3.5 cm and, each of two passes, 80% of what is past it (64% in all), `partial` 2.5 cm and 65% (42%), `moderate` 1.5 cm and 50% (25%), `body` 1 cm and 40% (16%), `strong` 4 mm and 15% (2%); `none` leaves it as made). Each corner has **shares of the body's parts** (`fit/parts.rs`: the trunk, each arm, each leg, by the bone each skin triangle follows most; all of one part where the cloth is less than half as far from it as from any other, elsewhere settled over the cloth from the corners round it), so a sleeve goes to the arm, never to the side it hangs beside. What is shortened is a patch's **mean** clearance (smoothed 24 rounds over the cloth), and the moves are smoothed again, so a collar, a cuff or a fold moves with the cloth it stands on. Never nearer the skin than the clearance kept. Cloth under the body's crotch on a garment without legs (a skirt's, a dress's, a coat's) keeps its shape |
| Budget | Thinned to `triangles` (`axcenity_assets::simplify_triangles`) |
| Tidy (`conform::tidy`) | Scraps (pieces joined to nothing with under 1% of the triangles, and since 2026-11-24 evening any piece but the largest with no corner within about 1.5 cm of it: a bit of a pit's floor floating past a cuff) and teeth (triangles held by one side: three rounds) come out, and each open edge's corners are evened along it (eight rounds of a step toward the two neighbours' middle and a longer one back, so a hem keeps its size; 2 cm at most) |
| Mend (`fit/mend.rs`, 2026-11-24 evening) | Every hole is filled. An open ring of the cloth (corners at one place are one; walked whichever way its edges wind) is an **opening** when it is over 0.6 m round, when a bone passes through it (each bone's line from its parent's head to its own, under 0.6 m, the hands, feet and head drawn on 25 cm) or when its middle is over 1 cm inside the body; anything else is a **hole** and is closed with triangles between its own corners (ears cut best-shaped first; no corner is added, so weights, shapes and the binding are untouched), none with a side over 12 cm. `IntakeReport::mended` and `openings` (the open rings and edges after each step: a step that adds rings tore the cloth) say what was done. The same runs in the fit after webs are cut, refusing any triangle that would be a web again (`FitReport::mended`) |
| Weld (`intake/weld.rs`, 2026-11-24 evening) | A generated mesh gives nearly every triangle corners of its own (one place and texture coordinate, a normal each): about three times the corners the cloth needs, each skinned and carrying every body shape, and its faces show as facets. A triangle wound against its corners' normals is turned over; each corner's normal is made again from the faces round its place within 80 degrees of its own; corners at one place with the same texture coordinates and normals within 12 degrees become one. `IntakeReport::welded` has the counts (over the 32 garments 12% to 42% fewer corners, 26% in all; the 30 in Character Studio's library went from 591 MB to 446 MB) |

`IntakeOptions` overrides any guess (`kind`, `ease`, `conform`, `scale` in metres per file unit, `nudge`); `IntakeReport` says which rule sized it and what it measured. For such a garment the import also sets four `FitOptions` that are off for everything else: `weights_by_part` (its skin weights are each body part's own at the point nearest the corner, mixed by the corner's shares of the parts, in place of the nearest skin's: a sleeve's from the arm all the way round, a skirt's from both legs; and a hand's, a finger's and the forearm's twist bone's weight goes to the forearm, a foot's and a toe's to the shin, `parts::held_joints`: a cuff sits on the forearm, and one that turned with the wrist swung round the arm), `cut_webs` (5: a triangle one of whose sides grows more than five times in a fit pose, the garment skinned by its own weights, comes out; the arms raised overhead aren't judged, where cloth under any arm is drawn out, nor is a skirt's cloth under `skirt_from`, which a stride draws out; a generated sleeve is often joined to the garment's side under the arm; at 3 the cut left holes that show with the arms down), `skirt_from` (the body's crotch height for a garment without legs: cloth under it gives 55% of its legs' shares to the trunk, so a skirt hangs from the hips and the legs only push it), `smooth_shapes` (12 rounds: each carried shape's moves are mixed with the neighbours', half way on the skin and all the way 5 cm off it, since a loose garment's corners are bound to far apart skin and a shape pulled them into spikes; with it the posed fit's following is limited too, `posed.rs`: a corner follows the skin it is bound to wholly within 1 cm of it and not at all past 4 cm, the following is smoothed six rounds over the cloth and held to 2.5 cm a corrective, where before a jacket's shoulder correctives moved neighbouring corners up to 22 cm apart, the tears seen in Idle) and `push_reach` (12 cm in place of 3: how deep inside the body a corner is still found and pushed out). `ImportRequest::texture_size` carries the source `.glb`'s base colour, normal and metal-roughness maps into the item's model, resized and stored as JPEG; shape moves under 0.1 mm are not written. Tools: `examples/import_garment.rs` (`--intake`, `--kind`, `--ease`, `--conform`, `--scale`, `--cut-for`, `--triangles`, `--textures`; and for any import `--also-fills`, `--tag`, `--limit`), `examples/read_garment.rs` (prints a reading), `examples/cover_map.rs` (makes items' cover maps again from their fitted models: `fit::remake_cover_map`), `assets/characters/realistic/clothing/generated/` (the list of generated garments and the batch import). **Limits:** the garment must stand upright facing +Z in a relaxed pose (a T-pose sleeve is placed but not turned); a garment made of loose sheets (pleats modelled as strips) has no trunk the reading can follow; items imported apart are not pushed clear of each other, only of what the import names with `under`: worn together, the outer one hides the inner one under it (below).

**Worn with the bottoms** (2026-11-24, fourth entry; ADR-0045's second amendment; `fit/hang.rs`, `FitOptions::hang`, `ImportRequest::worn_with`, the import tool's `--worn-with <item>`, `--tucked`, `--worn-most`). Every garment is fitted to the body and the underwear alone, so a top and a bottom worn together had no order round the hips: over the seat the bottom's cloth stood outside the top's over 29% to 90% of where they overlap, and in the setback of the top's hem, where the cover map leaves the bottom drawn, it showed in front of the top with a toothed outline (what looked like ragged hems). The import of a top or a jacket is now given the bottoms it will be worn with, and before the fit's push-out its cloth is moved, at rest:

| | |
| --- | --- |
| Measured | Each bottom's cloth over the skin, on its own: at every body corner from the waist to the top of the thighs (the map regions `waist`, `hips`, `thighs_top`), a ray out along the normal from 1 cm under the skin, up to 10 cm: where the cloth begins, and where it ends (hits again within 2.5 cm are the same garment's pockets and folds; farther is something else) |
| Moved | A corner of the garment whose nearest skin is that skin (a sleeve hanging beside the hip is nearest the arm). `Worn::Over`: out along the skin's normal until it is 3 mm outside the outside of every bottom. `Worn::Tucked`: in until it is 3 mm inside the inside of every bottom. By `most` at the most (3 cm, the list's `worn_most`); a bottom the corner would need more than twice that to clear (a skirt standing off the thigh) is left out for that corner. The moves are spread to the cloth round each corner and smoothed (6 rounds), and no corner gets less than it needs |
| Reported | `FitReport::hung` (`Hung`): the bottoms, the corners moved and the farthest, and the corners the cap left short with the most they are short by |
| Tucked | A top that lies on the body (the tank top, the polo, the off-shoulder top: `"worn": "tucked"` in `garments.json`) is given **layer 0**, so the bottoms are worn over it, and `resolve` lists the bottoms' cover maps for it twice: `ResolvedPart::under` and `ResolvedPart::tucked_into` (a `top` or `outerwear` under a `bottom`). The runtime reads the second in **blue** (`RegionHiding::whole`): its cloth is left out wherever the bottom's cloth is over it, with no setback from the waistband, since nothing is seen under a waistband as it is under a hem. Blue is lit that way only **round the hips** (the regions `waist`, `hips` and `thighs_top`); anywhere else it is as green, so the bib and the straps of overalls hide a tucked top as any garment hides one, with the setback (before, the polo's back was cut away between the straps) |
| Order of import | `import_generated.py` imports what is fitted to the body alone first (the bottoms among them), then the tops and jackets against the bottoms then in the library: a top's missing bottom is imported first, a re-imported bottom's tops are imported again after it, and `<library>/generated.worn-with.json` records the bottoms (by their models' SHA-256) each top was fitted for. `--check` fails when a top is out of date; the wardrobe pack's `build_pack.py` runs it |

Limits: done at rest on the default body only (a body shape or a pose can undo it by a few millimetres); the cap leaves a bottom that stands farther off still outside (the report's `short`); a tucked top still stands a few millimetres outside a tight waistband at the front (the fit lifts it clear of the underwear and of the body's extremes afterwards), where a strip of it up to about a centimetre shows over the waistband.

**Cover maps: what is worn hides what is under it** (2026-11-24, [ADR-0045](../../../wiki/Architecture/Architecture-Decisions/ADR-0045-What-Is-Worn-Over-Skin-Or-Cloth-Hides-It-By-A-Cover-Map.md); `fit/cover.rs`, `axcenity_assets`' `hidden.rs`). Every import writes the item's `cover_map` (1024 pixels a side) and, in its model, each corner's place on the skin (`_AXCENITY_SKIN_UV`: the body's texture coordinates at the corner's bound point; `Primitive::skin_uvs`).

| | |
|---|---|
| Covered | A body triangle with a region is under the garment as for coverage (the cloth out along its normal within 8 cm and not past an open edge), on the default body at rest, **and the cloth stays over it** (`cover::Proof`, 2026-11-24 evening): the garment and the body are posed together in every fit pose on every pose state, and the triangle counts only if in all of them each of its corners is under cloth by the gate's rule (`fit::uncovered` within 1 mm) or meets cloth out along its normal within 25 cm (a loose garment stands farther off the skin than the gate looks). The inner arm under a short sleeve is as near the shirt's side as its own sleeve at rest and comes out from under it when the arm lifts, so it stays drawn. Without poses (`cover_map` given no `Proof`) the stand-in from before is used: the cloth's skin weights at the nearest point share at least 0.8 of the triangle's, which left most skin under a loose garment drawn (the bomber jacket hid 566 of 3,349 covered triangles; tried in poses the puffer jacket hides 42% more skin and the cargo trousers 30% more, the cocktail dress as much as before; over the 32 garments a median of 36% of the skin under a garment is hidden, 8% to 72%). Skin in a crease the garment only closes over (the armpit) is not covered |
| Setback | From each covered triangle the way over the skin to the nearest uncovered one is measured (triangle middle to middle), with how far the cloth stands off the skin at that edge. The triangle is lit when that way, less how far its corners reach from its middle (the whole triangle is set back), is at least the setback: for **skin** (red) 2.5 times the stand-off, between 2.5 and 6 cm (a line of sight under the hem within 22 degrees of the skin is the first to reach where it stops; under 2.5 cm a sleeve's hem let hidden skin out in the walk); for **cloth worn beneath** (green) once the stand-off, between 3 mm and 4 cm |
| Wearing order (`ItemManifest::wearing_order`) | Garments only: by `layer`, then by slot (`underwear_*` 0, `shoes` 1, `bottom` 2, any other 3, `outerwear` 4). A top is over the bottoms of its layer; mid-layer bottoms (overalls) are over a base-layer top. Equal orders hide nothing of each other |
| Resolved | `Resolved::covers`: every worn garment's cover map. `ResolvedPart::under`: for a garment, the cover maps of the garments of greater order. `ResolvedPart::tucked_into`: those of them that are bottoms over a top or a jacket (read whole: "Worn with the bottoms", above) |
| Drawn | The body from a model without the triangles whose middle is lit in red in any map (with its hidden regions: one `RegionHiding`, one `…#h<key>` model). A garment with `under` from a model without the triangles whose three corners are all lit in green (`RegionHiding::garment`, `Model::without_covered`); a triangle left at the cover's edge has its lit corners sunk 15 mm against their normals, so it dives under the hem over it instead of standing through it |

So a waistband under a shirt, a shirt under a jacket and the skin under all three are not drawn where they are covered, and a garment taken off gives the plain models back; nothing is pushed or solved when an outfit changes. **A garment without skin coordinates borrows the body's** (2026-11-24 evening; `RegionHiding::skin`, `Model::with_skin_from`): one fitted by `shapes` (the underwear) or made on the body before this has no `_AXCENITY_SKIN_UV`, so when it is worn under another garment each of its corners takes the texture coordinates of the body's corner nearest it at rest (within 9 cm, in the files' own spaces), and it is hidden like a fitted one, with two differences for cloth that lies on the skin: only triangles **well** under the cover go (none of whose corners is shared with a triangle reaching out from under it, with a pinhole in the cover read as covered), and nothing sinks (`Model::without_well_covered`). So the briefs and the sports bra no longer show through a hole or a coarse patch of what is worn over them. What it can't do: the map is made on the default body, so on another build the setback is what guards skin and cloth near an opening. **The fit gate's holes** (`player`'s `fit_gate_tests::holes` and the motion check) count hidden skin with **no cloth over it** (`fit::uncovered`: the nearest cloth lies beside it, past an edge, and it isn't shut in a crease). Hidden skin that cloth has sunk a little under is no hole (cloth is what is seen there, which is what hiding it is for); an import still hides whole *regions* by the stricter `outside_cloth`. On the test tee the gate finds none at rest or in the held poses, and records a few corners at the walk's and the run's widest swing (up to 13 mm, where the cloth bridging the arm and the side follows the arm back).

**The realistic body's underwear** (`assets/characters/realistic/underwear.py`, 2026-11-16) is cut from the body's own surface, so it is fitted by `shapes`: its vertices are fixed mixes of body vertices and carry every body shape exactly. CO9 measured it against the transfer (`avatar`'s `tests/fitting.rs`): where its cloth lies on the skin the transferred shapes agree (90 to 96% of offsets within 1 mm), and where it bridges a hollow (the gusset, the bra between the breasts on the feminine frame) its own per-frame shapes are right and a transfer by binding would not be (up to 64 mm off), so it keeps `shapes`. Its regions keep their mask shapes, which sink the skin under it 5 mm (easing out 15 mm past the hems): it covers no map region whole, so map hiding can't stand in for them.

**Body hiding by regions** (CO9, built): the base's `model.regions_map` gives each body triangle a region (the map's value at the middle of the triangle's texture coordinates); `resolve` lists what the worn items hide (`Resolved::hidden_regions`), and the applier draws the body from a model with the triangles of every hidden **map** region left out (`axcenity_assets::RegionHiding`, `Assets::hide_regions`: the body's `MeshId` with a tag, `…#AxcenityRealistic/Body#h<16 hex>`, made from the base model on first use and shared; only the triangle lists change, at every level of detail, so live identity and posing apply unchanged; a bake on top is `…#h<key>#i<key>`). Taking the item off gives the plain `MeshId` back. An import hides the regions it covers whole (`fit::hidden_regions`, 99.5% of a region's triangles under it). A triangle is under the garment when the garment lies out along its normal (within 8 cm, not past an open edge), or (CO9 part 2, review fixes 1) when it is in a crease the garment closes over: a patch of such triangles each of whose rays out along its normal meets, within 15 cm, the skin across the crease facing back or the garment from inside (`coverage::closed_creases`; the armpit under a sleeve). Since then the realistic neck takes the trunk's skin round it above its joint and the upper arm has a third band, `upper_arms_mid` (map index 15, the last), so a crew-neck T-shirt covers `torso`, `waist`, `upper_arms_top` and `upper_arms_mid` whole at rest (`assets/characters/realistic/README.md`, "Regions"). **Covered at rest is not enough to hide** (review fixes 2): hidden skin that comes out from under the cloth in a pose is a hole the player sees through, and hiding takes it out of the fit gate's poke-through count. So an import keeps a covered region in `hides` only where, in every fit pose (`fit_poses`) on every pose state (`pose_states_of`) and at rest on each, none of its triangles' corners is more than `fit::HOLE` (1 mm) outside the fitted garment (`fit::pose_safe_hides`, with `fit::outside_cloth`: out along the cloth's normal where the cloth is over the skin, the whole distance where the nearest cloth is beside it past an edge, and nothing where the skin is shut in a crease, its ray out meeting the garment from inside within 15 cm or the skin facing back across the crease within 4 cm). What it leaves drawn is reported (`Imported::unhidden`, with how far it came out). The test tee hides only `waist` (the torso came out by 47 mm, the upper arm by 45 to 51 mm). The player's fit gate and motion check hold hidden skin to the same rule (holes). The stylized base keeps its mask shapes (`regions[].mask_shape`) until it is retired. Inner layers covered by an outer one are pushed under it, never through.

## 10. Character materials (ADR-0036)

Each material slot has a **shading model**. Values **modulate** the material's own maps (a tint multiplies the base colour map; a scale multiplies roughness); they never replace them, and a material with no values looks exactly as authored (every value's neutral setting changes nothing; audit §3.8). **In the World** a value is an attribute of the MeshPart whose model has the material, named `Mat_<material>_<value>` (`core::material_value_attribute`; a number or a `Color3`), written by the applier and read by `scene` (`material::modulate`), so it is saved, copied and replicated with the part. Until the renderer has the new models *(CO7)*, `scene` draws eye, cloth and hair as Standard; skin has its model since R5 (below). Since 2026-11-17 (CO7's first slice, `material::modulate`) it draws `melanin`, `undertone` and `redness` as one colour ratio in linear light on the base colour (`scene::skin_tone`: the tone table of `references/skin/README.md` §1.3, interpolated in optical density; all three at 0 give exactly 1), then `tint`, `iris_tint` and `sclera_tint` as multiplies, `roughness_scale` on the roughness and `detail_strength` on the normal map's strength (held to 0 to 2). The ratio is per material, not per texel: lips, nails, palms and soles tone with the skin until the Skin model has masks.

**How the renderer learns a slot's shading model (decided 2026-11-17, R5):** the model file says it. A glTF material's `extras` carry `"axcenity_shading": "skin"` (read by `assets` into `ModelMaterial::shading`, drawn by `scene` as `render::Shading::Skin`), and a skin may name a second, *defined* normal map beside its own smooth one (`"axcenity_defined_normal": {"index": <texture>}`). The realistic build writes both on `Skin` (`skin.py`'s `mark`). Chosen over an attribute the applier writes because the defined map must come from the file anyway, the model then draws alike wherever it is drawn (with or without a description), and it follows the `axcenity_sway` precedent; the manifest's `materials[].shading` stays the contract for which values a slot takes, and an `assets` test holds the realistic file and manifest in agreement on which materials are skin. `hair` and `cloth` use the same key since CO7's wave 4 (below the table); `eye` will; an unknown one draws as Standard. On a Skin-shaded material `scene` also draws `scatter_scale` (× how far light spreads under it) and `definition × (1 − softness)` (how much of the defined normal map shows); the realistic manifest drives `definition` from `muscle` and `softness` from `fat` (`manifests.py`'s `SKIN_DRIVERS`), so abdominal lines show only on lean, muscular bodies. What the Skin model draws: `docs/rendering/skin.md`. On a slot its file doesn't mark hair (the realistic `Brows` and `Lashes`), Hair's `melanin` and `redness` still take the skin's curve on the base colour. Still stored only: `pupil_size`, `limbal_ring`, `iris_depth`. The set is `avatar::ShadingModel::values`:

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
| | `anisotropy_rotation` | −90–90, 0 | Turns the stretch direction, degrees from the authored (a stretched highlight repeats every half turn) *(CO7 wave 4)* |
| `hair` | `tint` | colour, white | × colour after melanin and dye |
| | `melanin`, `redness` | −1–1, 0 | Lighter to darker; ash to red (pheomelanin's share of the melanin) |
| | `dye`, `dye_amount` | colour white; 0–1, 0 | A dye mixed in |
| | `tip_lightening` | 0–1, 0 | Root-to-tip gradient |
| | `root_melanin` | −1–1, 0 | Darker or lighter roots, over `root_length` *(CO7 wave 4)* |
| | `root_length` | 0–1, 0.15 | How far down the strand the roots' colour reaches *(CO7 wave 4)* |
| | `streaks` | 0–1, 0 | How much of the style's streak mask shows *(CO7 wave 4)* |
| | `streak_melanin` | −1–1, 0 | The streaks' melanin offset: highlights (−) or lowlights (+) *(CO7 wave 4)* |
| | `highlight` | 0–2, 1 | Secondary (coloured) highlight strength |
| | `shift_scale` | 0–2, 1 | × the cuticle tilt that shifts both highlights along the strand *(CO7 wave 4)* |
| | `roughness_scale` | 0.5–1.5, 1 | |

**Hair and Cloth (CO7, wave 4):** the renderer draws both models (`docs/rendering/hair.md`, `cloth.md`). A model file marks a material `"axcenity_shading": "hair"` or `"cloth"` in its glTF `extras`, as skin. A hair card atlas names its data map (root to tip, a value per strand, the streak mask, the strands' occlusion) and flow map there (`axcenity_hair_data`, `axcenity_hair_flow`), and a layer lying on the head is marked a shell (`axcenity_hair_shell: true`: its light fades over the head's terminator, review fixes 2); a cloth's sheen and anisotropy are glTF's `KHR_materials_sheen` and `KHR_materials_anisotropy`, which `assets` reads (factors only). On a material its file marks hair, `scene` sets the Hair values on `render::HairShading` and the renderer moves the colour along the two-pigment curve live (the base colour stays as made); on cloth, the Cloth values modulate the authored sheen and anisotropy. The six values marked *wave 4* are optional additions under ADR-0036's consequences: no enum grows, no `format_version` changes, a description without them reads as before.

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

`name` is the preset's label. The block rides in `Description::extra` (`Description::preset` reads it as `PresetInfo {id, kind, about, tags, thumbnail?, order?, feature?}`). `kind` says what applying it sets; everything else is left alone:

| `kind` | Sets | Left out of the file means |
|---|---|---|
| `body` | Every parameter of a `body` group | Its default. A parameter with `choices` (the frame) changes only when the file names it |
| `face` | Every parameter of a `face` group | The default: a face preset is a complete face (brief §5.1) |
| `hair` | The items in `hair` slots (with variant, materials, parameters), and `hair` group parameters | Bald |
| `outfit` | The items in `clothing` and `accessories` slots, except the slots a parameter's choices fill (`chosen_slots`) | Nothing worn there |
| `skin` | The base's `materials` values and `skin` group parameters | Neutral |
| `character` | Everything but the name (base, parameters, items, materials) | As a description |
| `feature` | Every parameter of one feature of the base model: the feature the block's `feature` names (`"feature": "nose"`), by `BaseManifest::feature_of` (a parameter's own `feature`, else its group's ID). No items, no material values, no makeup | Its default. A parameter with `choices` changes only when the file names it |

A `feature` preset (2026-11-24: a nose shape, a face shape) must name a feature its base model has. `check_preset` reports one that names none or an unknown one (`preset.feature`), one that sets a parameter of another feature or of none (`parameters.<id>`), and any item, material value or makeup in it; `apply_preset` refuses the first two, changing nothing, and otherwise sets only the feature's parameters. `PresetKind::Feature` owns no category (`owns` is false for every one) and no material values. An older engine reads the kind as `other`: listed, not applied. The other kinds don't read `feature`.

**One function**, `avatar::apply_preset(&mut Description, &preset, &Library) -> Result<(), Problem>`, is the rule for the studio, scripts and Randomize. It never copies the preset's `extra` (the block can't leak into a saved character), keeps the character's own `extra` and name, and puts the choices' items right afterwards (`wear_choices`). It refuses, changing nothing, a description without a block, a kind it doesn't know, or (but for `character`) a preset of another base. `matches_preset` says whether applying it would change nothing (the studio's "Custom" is no preset of a kind matching). `check_preset` holds a preset sound: its block (an ID, a sentence, a path), the description against the library, and nothing set outside what its kind owns; `Library::validate` runs it on every preset.

**Shipped presets** are written by `assets/characters/realistic/presets.py` into `realistic/presets/`: seven body presets per frame (brief §4.1's Slim, Average, Athletic, Muscular, Heavyset, Lean, Broad, from `references/body/README.md` section 6; tagged `male` or `female`, never naming `frame`), the nine face presets of brief §5.1 (`references/face/README.md` section 3), the six starting characters the editor had in code (kind `character`) and sixteen `feature` presets: ten noses (`axcenity/realistic-nose-<name>`, feature `nose`: Straight, Aquiline, Hooked, Snub, Button, Upturned, Bulbous, Long and thin, Wide, Low bridge) and six face shapes (`axcenity/realistic-face-shape-<name>`, feature `face_shape`: Oval, which is the face as built, Round, Square, Heart-shaped, Long, Narrow). A person's own presets are the same files in the workspace's `presets/` folder; favourites are editor state. `project::add_character_packages` and `packaging/stage.py` copy preset files with the rest of a package. The test `every_shipped_preset_is_sound_listed_and_resolves` holds them.

## 15b. Randomize (R1)

`avatar::randomize(&Description, &Scope, locks: &BTreeSet<String>, seed: u64, &Library) -> Description`, in `avatar` because games randomize NPCs too (section 0, decision 6).

- **Scope**: `All`; `Category(category)` (parameters of its groups and its slots); `Feature(id)` (parameters whose feature it is); `Parameters(ids)`. Only what it names changes; the name and everything else is kept.
- **Locks** are parameter IDs and slot IDs that never change.
- **Values** are drawn evenly inside `random` (`random_span`: the middle half of the range when absent), rounded to a thousandth. A parameter with choices takes one of its choices, so `All` and `Category(Body)` can switch `frame` (male and female); the Studio should lock `frame` by default (RA1). The base's constraints come next: every `follow` first (a target that wasn't drawn or locked moves with its drawn source, held in range), then every `max_sum` (what changed, drawn or moved by a follow, is scaled down; locked and out-of-scope values stay), so the limit holds whatever order the manifest lists them in (test `randomize_keeps_max_sum_and_moves_follow_targets`).
- **Items**: each slot in scope (not locked, not a choice's slot) gets one of the items that fit the base there, or nothing, drawn evenly; never a declared draft (`quality: "draft"`, asset-quality.md §5; test `a_declared_draft_is_never_drawn`); never one that conflicts with what is worn (`excludes`), needs what isn't (`requires`) or also fills a slot already taken. A `multiple` slot draws up to two. A drawn item gets a random variant and random values for its own parameters. Material values are left alone (skin is R5's).
- **Deterministic**: a small in-crate SplitMix64 generator, so a seed gives the same character on every machine. 1,000 seeds on every shipped base and on the test library validate and resolve with nothing left out, and locks hold (tests).

## 15c. Character Studio's editing contract (R1)

The workspace's panels never change the character themselves. Each draw function reads the workspace and returns `Intent`s (`editor/src/character_workspace/intent.rs`: set a parameter, wear, pick a category or feature or level, focus a view, undo, redo, save, reset a scope, apply a preset; since R1's second part also save as preset, toggle a favourite, randomize a scope, lock a value, hold the current character, show or keep or discard a held one, side by side, and recover or discard the autosaved one; since wave 3 the asymmetry tools, `Sides { tool, of }`, which mirror a side onto the other or reset a side for a feature's left and right pairs at a level or for one pair, one undo step each (`sides.rs`), and `PreviewMotion`, the preview's pose and motion: Still, Idle, Walk, Run or a gesture, `motion.rs`; since wave 4 the Hair and Clothing browsers, `browser.rs`: `Browse` (a page's search and filters, "Show drafts", the tile hovered or picked), `EndPreview` and `ToggleItemFavourite`, which are editor state, and the worn item's edits `SetItemVariant`, `SetItemMaterial` (values on several of its slots at once: a colour role's slots, a hair colour's melanin and redness) and `SetItemParameter`, one undo step each or one per drag; `Wear` stays the way an item goes on, one undo step, through `browser::dressed`, which also takes off what the item `excludes` or what fills its `also_fills` slots and puts on what it `requires`). Only the shell (`shell.rs`) applies them. `edit.rs` holds the history: snapshots of the description, with transactions (begin, commit, cancel) so a drag is one undo step, and reset by scope (everything, a category, a group). `selection.rs` holds what is being shown: the category, the feature and the level (Quick, Detailed, Advanced, read from each parameter's `level`, `feature` and `about`, §6). Selection, levels, locks, favourites (`favourites.json`), held versions and the second character side by side (`compare.rs`), the autosave (`recovery/character.json`, `autosave.rs`), the preview's expression and pose and motion (the Walk take at its own speed, a clip written to the workspace's `assets/animations/Humanoid/Preview/`, never to a character), the browsers' search, filters, item favourites and recently used items (`favourites.json`, beside the preset favourites) and the **tile tried on** (hovered or picked: shown in the character's place by `shown`, never an undo step or a change to the description until Wear; Esc, leaving the page or any change of the description put the character back: the workspace's `edit`, `transact`, `undo`, `redo` and `cancel_edit` end it, so Keep A and Randomize do too; after Esc or a click the tile under the pointer is muted until the pointer moves off it, and the Trying on bar is always drawn so a pick never moves the tiles), the camera and the stage are editor state:
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
           attributes, materials, problems,                       // items left out, and why
           hidden_regions, covers /* the worn garments' cover maps (ADR-0045) */ }
ResolvedPart { name, item, slot, model /* MeshId relative to the library */, fit: PartFit, materials,
               shapes /* the item's own parameters' shape weights (R1) */, transform /* a socket item's (R1) */,
               under /* the cover maps of the garments worn over it (ADR-0045) */ }
ItemManifest::wearing_order() -> Option<(layer, slot rank)>        // garments only, inside out
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

**What applying writes into the World:** the character's `Body` MeshPart gets the base body's `MeshId` (kept where the old model's skeleton stood, `anim::ragdoll::refit`; with the skin its items hide or cover left out it is that model's `…#h<key>`, and so is a garment's worn under another, ADR-0045), the attribute `Identity` (the key of the identity it wears live; R2a) and the body slider attributes; each part is a MeshPart inside the `Body` named after its object, with its base `MeshId`, its `Identity` and the attribute `DescriptionPart` (its item's ID, or `base:` and its model for the base's own parts and the `Body`); a `Body` with no Animator gets one, and an Animator with nothing to play poses a body that has parts at rest, so a still character wears its parts; parts already worn are kept, parts no longer worn are destroyed, and MeshParts without the attribute are the creator's own and left alone (one named like a library part is taken over, so the default humanoid's hand-placed parts become applied ones); a socket item gets an Attachment named after its socket, so `anim`'s sockets hang it; material values become `Mat_…` attributes; the description is kept as a `HumanoidDescription` inside the character's Humanoid (or named `AppliedDescription` inside a character without one). `DescriptionSync` applies kept descriptions that this run hasn't applied, or whose `Data` changed, once a frame from the game systems' step, so a loaded or replicated character registers its identity on each device (a character that was baked is baked again). The Game Editor steps one too in edit mode (R10, OPEN-21; `crates/editor/src/app/edit_characters.rs`), with no game system: it applies the kept descriptions and poses the described characters at rest (a copy of each in a World of its own, its Animators emptied, posed by `anim`'s Animators), so a saved described character shows its identity, items and materials in the viewport. It keeps what each character showed (body, model, identity, parts) and calls `DescriptionSync::forget` when a recorded step (an undo to a whole-world snapshot, Revert all) put back the unshown character with the same `Data`; a placed prefab copy is compared with its source shown the same way, in memory, so what edit mode derives is never a prefab override. `bake` swaps each owned MeshPart's `MeshId` for its derived one and removes its `Identity`; applying again undoes it.

**Fitting (CO9, `avatar::fit`; ADR-0041 decision 4 moved fitting from run time to build and import time):**

```rust
BodyMesh::from_model(&Model) -> Result<BodyMesh, FitError>        // a skinned model in its file space, its shapes
GarmentMesh::from_model(&Model);  Surface::new(positions, normals, triangles).closest(point, within) -> Option<Hit>
bind(&Surface, &[Vec3], &[Vec3]) -> Binding                         // garment points and their normals (empty: closest points)
place(&Binding, positions, normals, triangles) -> Vec<Vec3>
carried_shapes(&BaseManifest, &BodyMesh) -> Vec<String>          // identity (parameters, corners) + Corrective_*, no Mask_*
fit_garment(&BodyMesh, &GarmentMesh, &FitOptions { margin, shapes, under: Vec<Layer>, extremes, rounds, poses, pose_states,
    smooth_shapes, push_reach, cut_webs, weights_by_part /* the last four for garments from the intake */ }) -> Result<Fitted, FitError>
    // weights from the skin under each corner (both sides of a crease), the bound point's where none; normals turned with the push-out
Fitted { garment, joints, weights, shapes: Vec<CarriedShape>, binding, report: FitReport { layers: Vec<LayerOffset>, inside_most, .. } }
triangle_regions(&BodyMesh, &RegionMap) -> Vec<u8>;  coverage(&BodyMesh, &regions, &Surface) -> Coverage
hidden_regions(&BaseManifest, &Coverage) -> Vec<String>;  classify(&BaseManifest, &Coverage) -> Option<SlotGuess>
cover_map(&BodyMesh, &regions, &Surface, (joints, weights), size) -> (CoverMap, CoverReport)   // ADR-0045: what the garment hides, over the body's texture
skin_uvs(&BodyMesh, &Binding) -> Vec<[f32; 2]>                       // each garment corner's place on the skin (`_AXCENITY_SKIN_UV`)
uncovered(&[Surface], &Surface, p, n) -> f32                         // how far hidden skin is from under any cloth (the gate's holes)
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
