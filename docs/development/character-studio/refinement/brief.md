# Axcenity Character Studio — Comprehensive Character Creator Refinement Plan

> Paul's brief of 2026-11-16, kept as given. The plan that carries it out is [plans/axcenity-engine-character-studio.md](../../../../plans/axcenity-engine-character-studio.md) (its section 11 maps this brief's Stages 0 to 11 onto the plan's milestones). Sections are numbered as in the brief, so "brief §5.2" is a stable reference.

**Project:** Axcenity Engine
**Subsystem:** Character Studio
**Objective:** Evolve the existing Character Studio into a sophisticated, approachable character creation experience inspired by The Sims 4, MetaHuman Creator, and professional character customization tools.

**Implementation philosophy:** Build upon the existing character architecture, improve the user experience, expand customization capabilities, and establish a scalable character content pipeline.

---

# 1. Project Vision

The objective is to create a character creator that makes it possible for virtually anyone to create an appealing character without requiring knowledge of 3D modeling, sculpting, rigging, or material authoring.

The experience should combine:

- The Sims 4's approachable character creation and direct manipulation.
- MetaHuman's realistic anatomy, facial detail, and material quality.
- Blender's sophisticated geometry and deformation capabilities.
- Axcenity's own customization, content, and runtime architecture.

The user should be able to create a recognizable character in minutes while retaining the ability to spend hours refining individual facial features, anatomical proportions, clothing, and appearance.

## Core principles

1. **Character-first interaction:** The character is the primary focus, not the collection of sliders.
2. **Progressive complexity:** Common adjustments should be accessible immediately; advanced controls should remain optional.
3. **Immediate feedback:** All modifications should update the viewport without requiring manual application.
4. **Anatomical consistency:** Facial and body adjustments must preserve believable anatomy.
5. **Non-destructive customization:** Users should be able to experiment, undo, redo, and reset individual changes.
6. **Visual discoverability:** Users should understand what a customization control changes before using it.
7. **Scalable content:** The system must accommodate additional body types, hairstyles, clothing, accessories, and materials without redesigning its architecture.
8. **Runtime consistency:** A character created in Character Studio must look the same when saved, reopened, and used in a game.
9. **Reference-driven development:** Geometry and appearance must be developed using documented anatomical and visual references.
10. **Capability-aware customization:** Only expose controls supported by the selected asset.
11. **Professional presentation:** The character creator should feel like a dedicated application, not an extension of a generic editor properties panel.

# 2. Existing Architecture and Implementation Constraints

Before implementation, inspect the current repository and establish the actual implementation status of every proposed feature.

The following existing systems should be preserved and extended:

- `crates/avatar`: Character descriptions, manifests, catalogues, libraries, and validation.
- `crates/editor/src/character_workspace.rs`: Character Studio workspace and state.
- `crates/editor/src/character_workspace/ui.rs`: Character Studio interface.
- `assets/characters/realistic/`: Realistic character family and its manifests.
- `assets/characters/humanoid/`: Existing stylized character family.
- `crates/anim`: Skeleton, animation, deformation, and corrective systems.
- Existing Blender build, fitting, and visual QA scripts.
- Existing character runtime application and Lua API.

The format 2 character description and manifest-driven parameter architecture should remain the foundation.

Do not introduce a second competing character-description system.

Do not replace working geometry, fitting, or runtime systems simply to accommodate a UI redesign.

The existing realistic and stylized families should remain supported.

The realistic family should be the default direction for new character creation.

The current implementation has already established much of the realistic body's shape system, facial identity controls, facial expressions, and character application pipeline. Verify the latest merged state before proposing further work.

# 3. Character Studio Interface Redesign

## 3.1 Main Workspace

Redesign the Character Studio interface to feel like a dedicated character creation application rather than a conventional editor properties panel.

Recommended layout:

**Left navigation**
- Body
- Face
- Hair
- Clothing
- Skin and Makeup
- Accessories
- Expressions
- Presets

**Central viewport**
- Large interactive character preview.
- Orbit, pan, and zoom.
- Front, side, rear, and three-quarter views.
- Full-body and face-focused camera presets.
- Neutral lighting and configurable studio lighting.
- Character selection and direct manipulation.
- Optional anatomical region highlighting.

**Right contextual panel**
- Context-sensitive customization tools.
- Feature-specific controls.
- Preset selection.
- Color and material pickers.
- Advanced controls.
- Numeric parameter entry.

**Top toolbar**
- Character name.
- New character.
- Open character.
- Save.
- Undo and redo.
- Reset.
- Randomize.
- Character comparison.
- Viewport settings.

**Bottom viewport controls**
- Camera presets.
- Pose selection.
- Lighting presets.
- Animation preview.
- Screenshot and turntable capture.

Avoid excessive borders, nested panels, and unnecessarily large controls.

The interface should feel modern, minimal, and polished.

Use Axcenity's existing visual language rather than copying The Sims interface literally.

## 3.2 Contextual Customization

The interface should display only controls relevant to the selected category or feature.

For example:

Selecting Face opens facial customization.

Selecting Nose displays:
- Nose width.
- Nose length.
- Bridge height.
- Bridge width.
- Tip projection.
- Tip rotation.
- Nostril width.
- Nostril height.
- Nostril angle.

Selecting Eyes displays:
- Eye size.
- Eye spacing.
- Eye height.
- Eye depth.
- Eye tilt.
- Upper eyelid shape.
- Lower eyelid shape.
- Eye opening.
- Eye corner position.

Controls should be organized into logical groups rather than presented as one long list.

## 3.3 Progressive Customization

Implement three levels of customization.

**Level 1 — Quick customization**
- Body presets.
- Face presets.
- Hair presets.
- Clothing presets.
- Skin tones.
- Overall proportions.
- Broad facial adjustments.

**Level 2 — Detailed customization**
- Individual facial parameters.
- Regional body adjustments.
- Hairline and hairstyle adjustments.
- Clothing variations.
- Skin details.
- Material adjustments.

**Level 3 — Advanced customization**
- Independent left/right facial adjustments.
- Precise numeric values.
- Anatomical proportions.
- Fine geometry adjustments.
- Advanced material parameters.
- Detailed feature positioning.
- Advanced asymmetry controls.

The existing Advanced toggle should evolve into this more deliberate progressive customization system.

## 3.4 Interactive Viewport Editing — Core Requirement

The viewport must become an actual customization interface rather than simply a place to inspect the character.

Users should be able to interact directly with the character to modify supported features.

Examples:

**Facial manipulation**
- Select the nose and adjust its width, bridge, or tip position.
- Select the eyes and adjust spacing, height, or tilt.
- Select the jaw and adjust width or definition.
- Select the mouth and adjust width or lip proportions.
- Select the ears and adjust size or protrusion.

**Body manipulation**
- Select shoulders to adjust shoulder width.
- Select the waist to adjust waist proportions.
- Select hips to adjust hip width.
- Select legs to adjust supported proportions.
- Select the head to adjust its size.

### Interaction behavior

1. Hovering over a supported feature should provide subtle visual feedback.
2. Clicking selects the feature.
3. The viewport focuses the selected region when appropriate.
4. Relevant manipulation handles appear.
5. Dragging a handle changes the corresponding parameter.
6. The contextual panel updates to reflect the new value.
7. Users can switch between direct manipulation and slider-based adjustments without losing their changes.

### Technical requirements

- Use the existing parameter and deformation systems.
- Direct manipulation must update the same underlying values as the conventional controls.
- Do not introduce a separate geometry editing state.
- Support undo and redo for direct manipulation.
- Provide predictable interaction sensitivity.
- Prevent camera movement from accidentally modifying the character.
- Support symmetry-aware editing.
- Allow direct manipulation to be disabled when users prefer conventional controls.

For complex features, prioritize a small number of meaningful manipulation handles over an unnecessarily complicated sculpting interface.

The initial implementation should prioritize eyes, nose, jaw, mouth, shoulders, waist, and hips.

# 4. Body Customization

Expand the existing body customization system without replacing the current shape-manifest architecture.

## 4.1 Overall Body Shape

Provide intuitive controls for:

- Body frame.
- Height.
- Overall build.
- Muscle definition.
- Body fat.
- Shoulder width.
- Waist width.
- Hip width.
- Chest proportions.
- Leg proportions.
- Arm proportions.
- Head-to-body ratio.

Create visual presets such as:

- Slim.
- Average.
- Athletic.
- Muscular.
- Heavyset.
- Lean.
- Broad.
- Custom.

Presets establish starting values rather than locking characters into fixed body types.

## 4.2 Regional Body Customization

Allow users to select individual anatomical regions through the character viewport or a simplified anatomical selection interface.

Regions should include:

- Head and neck.
- Shoulders.
- Chest.
- Upper arms.
- Forearms.
- Hands.
- Abdomen.
- Waist.
- Back.
- Hips.
- Thighs.
- Calves.
- Feet.

Each region should expose appropriate controls.

For example, selecting the abdomen could expose:
- Abdominal fullness.
- Abdominal muscle definition.
- Waist transition.
- Side profile depth.

Selecting shoulders could expose:
- Shoulder width.
- Deltoid volume.
- Shoulder slope.
- Upper-body proportions.

Do not expose meaningless combinations of controls for every region. Parameters should reflect the actual anatomical properties of the selected area.

## 4.3 Direct Body Manipulation

Implement viewport-based body customization.

Users should be able to select a region and manipulate supported parameters through intuitive handles.

Examples:

- Dragging shoulder handles changes shoulder width.
- Dragging waist handles changes waist width.
- Dragging height handles adjusts overall height.
- Dragging hip handles adjusts hip width.

Direct manipulation must update the same underlying parameters as conventional controls.

## 4.4 Body Deformation Quality

Maintain strict quality requirements:

- No visible holes or intersections.
- No abrupt deformation transitions.
- No broken anatomy.
- No unexpected changes to unrelated body regions.
- No excessive volume loss.
- No clothing penetration.
- No broken joint deformation.
- No eye, teeth, or other attached-part intersections.

Test extreme combinations, not just individual sliders.

Continue using the existing Blender and in-engine QA infrastructure.

# 5. Facial Customization

Facial customization should receive particular attention because it is one of the most important components of a sophisticated character creator.

The existing realistic face system should be extended rather than replaced.

## 5.1 Facial Presets

Introduce a facial preset library.

Presets should provide complete starting configurations that users can subsequently modify.

Examples:

- Round face.
- Oval face.
- Square face.
- Heart-shaped face.
- Long face.
- Broad face.
- Narrow face.
- Soft features.
- Angular features.

Avoid treating these as rigid biological categories. They are simply useful visual starting points.

Presets should be data-driven and compatible with the existing character description.

## 5.2 Facial Feature Categories

**Eyes**
- Eye size.
- Eye spacing.
- Eye depth.
- Eye height.
- Eye tilt.
- Eye shape.
- Upper eyelid.
- Lower eyelid.
- Inner and outer eye corners.
- Eyeball proportions.
- Iris and pupil appearance.

**Eyebrows**
- Brow height.
- Brow width.
- Brow arch.
- Brow length.
- Brow thickness.
- Brow angle.
- Brow ridge.

**Nose**
- Nose length.
- Bridge width.
- Bridge height.
- Bridge profile.
- Tip size.
- Tip projection.
- Tip rotation.
- Nostril width.
- Nostril height.
- Nostril angle.
- Overall nose width.

**Cheeks**
- Cheekbone prominence.
- Cheek fullness.
- Midface width.
- Under-eye fullness.
- Cheek projection.

**Jaw**
- Jaw width.
- Jaw angle.
- Jaw depth.
- Jaw definition.
- Jaw asymmetry.

**Chin**
- Chin width.
- Chin height.
- Chin projection.
- Chin shape.
- Chin cleft where supported.

**Mouth and Lips**
- Mouth width.
- Mouth height.
- Upper lip fullness.
- Lower lip fullness.
- Cupid's bow.
- Lip corners.
- Mouth projection.
- Philtrum length.

**Forehead**
- Forehead height.
- Forehead slope.
- Forehead width.
- Temple width.
- Brow-to-forehead transition.

**Ears**
- Ear size.
- Ear height.
- Ear protrusion.
- Ear rotation.
- Earlobe size.
- Ear shape.

Not every parameter needs to be exposed in the initial interface. Prioritize controls that produce visibly meaningful results.

## 5.3 Direct Facial Manipulation

Implement an interactive facial editing mode.

When users select a facial region:

1. Focus the camera on the selected area.
2. Highlight the selected feature.
3. Display relevant adjustment handles.
4. Show contextual parameters.
5. Update the character immediately.

For example, selecting the nose should provide handles for bridge width, tip position, and overall proportions.

Use appropriate camera framing and interaction sensitivity.

The system should support conventional sliders alongside direct manipulation.

## 5.4 Facial Asymmetry

Retain the existing symmetry system but improve its usability.

Provide:

- Symmetry enabled by default.
- Independent left/right editing.
- Mirror current values.
- Copy left to right.
- Copy right to left.
- Reset individual sides.

Allow natural facial asymmetry without forcing it.

# 6. Skin, Makeup, and Surface Details

Expand the existing material system into a dedicated appearance customization category.

## 6.1 Skin Appearance

Provide:

- Skin tone.
- Skin undertone.
- Skin roughness.
- Skin variation.
- Subsurface scattering controls.
- Freckles.
- Moles.
- Blemishes.
- Scars.
- Skin detail intensity.

Skin tone should use a carefully designed palette and a more detailed color picker for advanced users.

Avoid relying on a single RGB color parameter for realistic skin customization.

Support appropriate material layering and consistent rendering under different lighting conditions.

## 6.2 Makeup

Introduce optional makeup layers:

- Foundation.
- Blush.
- Eyeliner.
- Eyeshadow.
- Lip color.
- Eyebrow color.

Each layer should support:
- Color.
- Opacity.
- Roughness where appropriate.
- Presets.
- Independent enable/disable.

## 6.3 Tattoos and Markings

Establish a scalable system for:

- Tattoos.
- Birthmarks.
- Scars.
- Decorative markings.
- Custom decals.

Support placement, rotation, scale, opacity, and material blending.

Do not implement a complex decal authoring system inside the initial character creator milestone. Establish the architecture first.

# 7. Hair and Grooming

Hair needs to become a substantial customization category rather than simply a selectable mesh.

## 7.1 Hairstyle Library

Support:

- Short hairstyles.
- Medium hairstyles.
- Long hairstyles.
- Curly hairstyles.
- Wavy hairstyles.
- Straight hairstyles.
- Braided hairstyles.
- Ponytails.
- Buns.
- Bald and shaved options.

Provide a visual thumbnail browser with search, filtering, and favorites.

## 7.2 Hair Customization

Where supported by the hairstyle, expose:

- Length.
- Volume.
- Parting.
- Hairline.
- Fringe/bangs.
- Curl intensity.
- Strand density.
- Hair color.
- Highlight color.
- Root color.
- Roughness.

Do not promise arbitrary deformation of every hairstyle. Each style should declare which parameters it supports.

## 7.3 Facial Hair

Support:

- Mustaches.
- Beards.
- Stubble.
- Sideburns.
- Different beard lengths.
- Beard shapes.
- Color matching.

Hair assets should use the existing groom architecture and tiered quality approach.

# 8. Clothing and Wardrobe

Build a complete visual wardrobe browser.

## 8.1 Clothing Categories

- Tops.
- Shirts.
- T-shirts.
- Sweaters.
- Jackets.
- Coats.
- Pants.
- Shorts.
- Skirts where supported.
- Footwear.
- Underwear.
- Full outfits.

## 8.2 Wardrobe Browser

The wardrobe should provide:

- Visual thumbnails.
- Category filtering.
- Search.
- Favorites.
- Recently used items.
- Outfit presets.
- Color variations.
- Compatibility information.

Users should be able to preview items directly on their character without committing to them.

## 8.3 Clothing Materials

Allow supported clothing assets to expose:

- Primary color.
- Secondary color.
- Accent color.
- Material selection.
- Roughness.
- Metallic properties where appropriate.
- Texture variants.

Material customization should be data-driven and constrained by each item's material manifest.

## 8.4 Clothing Fit

Continue developing the existing fitting pipeline.

Require:

- Compatibility with supported body shapes.
- Reliable deformation.
- No visible clipping.
- Appropriate garment layering.
- Proper sleeve and waist behavior.
- Correct footwear coverage.
- Stable fitting under animation.

A clothing item must declare its supported body families and any fitting limitations.

# 9. Accessories

Introduce a dedicated accessory system.

Potential categories:

- Glasses.
- Sunglasses.
- Earrings.
- Necklaces.
- Watches.
- Hats.
- Headwear.
- Belts.
- Bags.
- Other wearable accessories.

Use attachment sockets and existing character binding architecture.

Accessories should support position, rotation, scale, and material variants where appropriate.

# 10. Expressions and Animation Preview

Expand the existing facial expression infrastructure into a character preview system.

**Facial expressions**
- Neutral.
- Happy.
- Sad.
- Angry.
- Surprised.
- Disgusted.
- Fearful.
- Confused.
- Custom.

**Facial controls**
- Smile intensity.
- Brow movement.
- Eye squint.
- Mouth opening.
- Jaw movement.
- Blink intensity.

**Animation preview**
- Idle.
- Walking.
- Running.
- Standing.
- Basic gestures.

Expressions and animations should be preview-only unless the user explicitly chooses to save them as part of an appropriate animation or character asset.

The character's identity should remain independent of its current expression.

# 11. Character Presets and Randomization

Introduce a proper preset system.

Users should be able to:

- Create characters from a starting preset.
- Save custom presets.
- Save facial presets.
- Save body presets.
- Save hairstyles and outfits.
- Favorite presets.
- Randomize the entire character.
- Randomize only the selected category.
- Randomize selected facial features.
- Lock features while randomizing other areas.

Randomization must produce valid characters.

Use controlled parameter ranges and compatible asset combinations rather than completely arbitrary values.

# 12. Character Comparison and Iteration Tools

Introduce a dedicated comparison workflow.

Character customization often involves making many small adjustments and deciding whether the new result is actually better. Users need an easy way to compare alternatives.

## 12.1 Before and After Comparison

Allow users to:
- Capture a character state before making changes.
- Compare the current state against the captured state.
- Toggle between before and after.
- Compare using identical camera angles and lighting.
- Restore the original state.

## 12.2 Side-by-Side Comparison

Support:
- Two-character comparison.
- Side-by-side facial comparisons.
- Body proportion comparisons.
- Preset comparisons.
- Hairstyle comparisons.
- Clothing comparisons.

Comparison should use synchronized camera controls where appropriate.

## 12.3 Alternative Character States

Allow temporary character variants without creating permanent duplicate character files.

For example, users should be able to compare three nose variations or several facial presets before selecting one.

These variants should be isolated editing states that can be discarded or committed.

Do not allow comparison workflows to interfere with undo/redo history or saved character data.

# 13. Reference Research and Asset Sourcing

This is a major part of the project.

Do not develop new facial or anatomical deformation shapes based exclusively on imagination or arbitrary slider values.

Establish a structured reference library before producing major new geometry.

## 13.1 Reference Sources

**Character creation UX**
- The Sims 4 Create a Sim.
- MetaHuman Creator.
- Character Creator 4.
- Ready Player Me.
- Blender character modeling interfaces.

**Facial anatomy**
- Anatomy for Sculptors.
- 3D.sk.
- ArtStation.
- Blender Studio.
- Sketchfab.
- Human anatomy educational references.

**Body anatomy**
- Anatomy for Sculptors.
- Visible Body.
- Blender Studio.
- MakeHuman.
- 3D.sk.
- ArtStation.

**Hair**
- Blender Studio.
- ArtStation.
- MetaHuman documentation.
- Unreal Engine groom documentation.
- XGen documentation.

**Clothing**
- Marvelous Designer.
- CLO3D.
- Blender Studio.
- ArtStation.
- Fashion photography and clothing reference photography.

**Skin and materials**
- TexturingXYZ.
- Poly Haven.
- ambientCG.
- Adobe Substance 3D documentation.
- Unreal Engine material documentation.

## 13.2 Facial Reference Collection

For each facial feature, collect references showing:

- Front view.
- Side profile.
- Three-quarter view.
- Close-up.
- Different shapes and proportions.
- Different levels of detail.

Prioritize consistent photographic references.

Create a reference collection for:

- Eye shapes.
- Eyelid anatomy.
- Brow structure.
- Nose profiles.
- Nose tips.
- Nostril shapes.
- Cheekbone structures.
- Jawlines.
- Chin shapes.
- Lip shapes.
- Ear anatomy.

Use these references to establish sensible deformation limits and parameter names.

## 13.3 Anatomical Reference Collection

Create a body anatomy collection covering:

- Front.
- Back.
- Side.
- Three-quarter.
- Arms raised.
- Arms lowered.
- Bent elbows.
- Bent knees.
- Walking and standing poses.

Collect examples of different:

- Body proportions.
- Muscle development.
- Body fat distributions.
- Shoulder structures.
- Waist shapes.
- Hip proportions.
- Leg proportions.

Include both anatomical references and real-world photographic references.

Do not use bodybuilding photographs as the sole reference for normal human anatomy.

## 13.4 Reference Management

Create a structured directory:

`docs/development/character-studio/references/`

Recommended organization:

- `ux/`
- `face/eyes/`
- `face/nose/`
- `face/jaw/`
- `face/lips/`
- `face/ears/`
- `body/overall/`
- `body/upper/`
- `body/lower/`
- `hair/`
- `clothing/`
- `skin/`
- `materials/`

Maintain a reference index documenting:

- Reference name.
- Source URL.
- Author or publisher.
- Date collected.
- Relevant feature.
- What the reference demonstrates.
- License or usage restrictions.
- Whether the reference is for visual guidance or an actual asset.

Pinterest can be used to discover references, but trace important images back to their original creators and sources.

Never assume that an image being publicly accessible means it can be redistributed or used in commercial assets.

## 13.5 Asset Licensing

Separate assets into:

1. CC0 and public-domain assets.
2. Open-source assets with compatible licenses.
3. Licensed commercial assets.
4. Reference-only material.
5. Original Axcenity-created assets.

Record attribution and license requirements.

Do not incorporate reference meshes or textures into shipped assets without verifying their permissions.

# 14. Character Asset Production Pipeline

Continue using Blender as the primary character asset authoring environment.

The asset pipeline should support:

- Base body meshes.
- Shape keys.
- Facial deformation shapes.
- Corrective shapes.
- UV maps.
- Skin materials.
- Hair.
- Clothing.
- Accessories.
- Skeletons.
- Attachment sockets.

Each asset should have a corresponding manifest describing its supported functionality.

## 14.1 Asset Capability Manifests

Make capability-aware customization a core architectural requirement.

Every hairstyle, clothing item, accessory, and other customizable asset should declare which properties it supports.

For example:

A hairstyle might support:
- Primary color.
- Highlight color.
- Length.
- Volume.
- Parting.

A different hairstyle might only support:
- Primary color.
- Highlight color.

The interface must dynamically expose only supported controls.

Capability metadata should describe:

- Supported parameters.
- Supported parameter ranges.
- Default values.
- Material slots.
- Compatible body families.
- Attachment requirements.
- Supported variants.
- Supported customization operations.
- Required dependencies.
- Known compatibility limitations.

Avoid creating universal controls that do nothing for particular assets.

The manifest should be extensible and versioned so new asset capabilities can be introduced without breaking existing characters.

## 14.2 Asset Quality Requirements

- Clean topology.
- Appropriate edge flow.
- Consistent UVs.
- Correct normals.
- Appropriate deformation weights.
- Stable skeleton compatibility.
- Proper material separation.
- No unintended mesh intersections.
- Correct LOD generation.
- Documented dependencies.

Use automated Blender scripts for repeatable generation and QA wherever possible.

# 15. Technical Architecture Improvements

Preserve the existing `avatar` crate as the source of truth for character data.

The Character Studio should remain a consumer of the shared character description and asset systems.

Extend existing architecture rather than introducing redundant subsystems.

Potential additions:

**Parameter metadata**
- Friendly descriptions.
- Categories and subcategories.
- Tooltips.
- Visual icons.
- Supported ranges.
- Default values.
- Preset groupings.
- Symmetry relationships.
- Viewport manipulation metadata.

**Asset metadata**
- Preview thumbnails.
- Supported body families.
- Supported customization options.
- Material variants.
- Compatibility rules.
- Asset tags.
- Capability declarations.

**Character editing**
- Reliable undo and redo.
- Transactional edits.
- Category-level reset.
- Preset application.
- Randomization.
- Autosave and recovery.
- Temporary comparison states.

**Viewport interaction**
- Feature selection.
- Region selection.
- Contextual manipulation.
- Camera focus.
- Selection highlighting.
- Direct manipulation handles.

All editor interactions must update the underlying character description consistently.

Avoid storing editor-only temporary selections as character identity data.

# 16. Performance Requirements

Character customization must remain responsive.

Prioritize:

- Incremental geometry updates.
- Efficient shape blending.
- Cached deformation data.
- Avoiding unnecessary mesh rebuilds.
- Efficient material updates.
- Asynchronous thumbnail loading.
- Efficient wardrobe browsing.
- Appropriate LOD behavior.
- Efficient viewport selection and picking.

Avoid rebuilding the entire character when changing a material color or another isolated property.

Measure performance with representative characters and complex combinations.

Establish performance budgets based on actual hardware and measured results rather than arbitrary assumptions.

# 17. Standardized Character Quality Benchmark

Establish a permanent benchmark collection that is used throughout development.

This should become part of the acceptance process for character geometry, rendering, clothing, and deformation changes.

## 17.1 Benchmark Characters

Maintain representative characters covering:

- Average male character.
- Average female character.
- Slim character.
- Athletic character.
- Muscular character.
- Heavyset character.
- Character with broad facial proportions.
- Character with narrow facial proportions.
- Character with distinctive facial features.
- Character with natural facial asymmetry.

Include both realistic and stylized families.

## 17.2 Benchmark Appearance Tests

Every major visual change should be checked against:

**Facial tests**
- Front.
- Side.
- Three-quarter.
- Neutral expression.
- Smile.
- Frown.
- Eyes closed.
- Mouth open.

**Body tests**
- Front.
- Side.
- Back.
- A-pose.
- T-pose where applicable.
- Arms raised.
- Bent elbows.
- Bent knees.
- Walking animation.

**Wardrobe tests**
- Multiple body shapes.
- Multiple clothing layers.
- Different materials.
- Different poses.
- Extreme supported body proportions.

**Rendering tests**
- Neutral studio lighting.
- Warm lighting.
- Cool lighting.
- Outdoor lighting.
- Close-up skin inspection.

## 17.3 Visual Regression

Use consistent camera positions, lighting, resolution, and character states for automated captures.

Store benchmark evidence alongside the relevant implementation work.

Automated measurements should be supplemented by human visual review.

Do not loosen geometric acceptance thresholds simply because a visual result looks acceptable. Investigate discrepancies and document any justified changes.

# 18. Implementation Roadmap

Implement this as a sequence of independently reviewable stages.

## Stage 0 — Repository Audit and Reference Research

- Review current Character Studio implementation.
- Identify completed and incomplete CO12 requirements.
- Review the existing realistic and stylized families.
- Audit the existing UI and viewport interactions.
- Review parameter manifests.
- Collect UX references.
- Collect facial anatomy references.
- Collect body anatomy references.
- Review asset licensing.
- Establish visual QA baselines.
- Audit existing benchmark characters.
- Identify missing asset capability metadata.

**Deliverables:** Audit, reference index, implementation breakdown, updated visual QA requirements, benchmark plan.

## Stage 1 — Character Studio UX Redesign

- Redesign the three-panel workspace.
- Improve category navigation.
- Implement contextual properties.
- Improve viewport controls.
- Improve camera presets.
- Introduce cleaner control grouping.
- Improve preset presentation.
- Implement undo/redo and category reset UX.
- Introduce progressive customization.
- Introduce the character comparison workflow.
- Establish viewport interaction architecture.

**Acceptance criteria:** A new user can create and modify a character without needing to understand the underlying parameter system.

## Stage 2 — Interactive Viewport Customization

- Implement feature selection.
- Implement body region selection.
- Implement facial feature selection.
- Add visual highlighting.
- Implement contextual manipulation handles.
- Connect manipulation handles to existing parameters.
- Implement symmetry-aware interaction.
- Add camera focus behavior.
- Add direct manipulation undo/redo.
- Prevent conflicts between camera navigation and feature editing.

Prioritize:
1. Nose.
2. Eyes.
3. Jaw.
4. Mouth.
5. Shoulders.
6. Waist.
7. Hips.

**Acceptance criteria:** Users can make meaningful changes by interacting directly with the character, with identical results available through conventional controls.

## Stage 3 — Body Customization Experience

- Improve overall body controls.
- Introduce visual body presets.
- Improve regional customization.
- Add viewport region selection.
- Implement direct body manipulation.
- Improve parameter descriptions and tooltips.
- Validate deformation quality.

**Acceptance criteria:** Users can intuitively adjust overall body shape and individual anatomical regions without introducing geometry defects.

## Stage 4 — Facial Customization Experience

- Reorganize existing facial parameters.
- Introduce facial presets.
- Improve feature-specific customization.
- Implement facial camera focusing.
- Introduce direct facial manipulation.
- Improve asymmetry controls.
- Add missing high-value facial parameters.
- Validate facial deformation combinations.

**Acceptance criteria:** Users can create substantially different and believable faces through both presets and individual adjustments.

## Stage 5 — Skin and Appearance

- Expand skin customization.
- Implement skin tone and undertone selection.
- Add surface detail options.
- Implement makeup layers.
- Establish the decal architecture.
- Improve material previews.

**Acceptance criteria:** Characters can have visually distinct skin appearances without compromising material quality.

## Stage 6 — Hair and Grooming

- Improve hairstyle browser.
- Expand hairstyle library.
- Add hair color customization.
- Add supported style-specific parameters.
- Improve facial hair options.
- Validate grooming deformation and rendering.
- Implement asset capability-aware controls.

**Acceptance criteria:** Users can create significantly different hairstyles and grooming appearances using a consistent browser, without unsupported controls being displayed.

## Stage 7 — Clothing and Wardrobe

- Build visual clothing browser.
- Expand clothing categories.
- Add outfit presets.
- Improve color and material variants.
- Improve compatibility filtering.
- Expand fitting QA.
- Validate clothing under different body shapes and animations.
- Implement clothing capability manifests.

**Acceptance criteria:** Users can assemble complete outfits with reliable fitting and no major visual defects.

## Stage 8 — Accessories

- Implement accessory browser.
- Expand attachment support.
- Add material variants.
- Add accessory positioning.
- Validate attachment behavior under animation.
- Implement accessory capability manifests.

**Acceptance criteria:** Accessories can be equipped, adjusted, saved, and correctly reproduced at runtime.

## Stage 9 — Expressions and Animation Preview

- Expand expression browser.
- Improve expression controls.
- Add facial preview modes.
- Add animation preview.
- Validate expressions against different facial identities.

**Acceptance criteria:** Users can inspect their character's appearance under multiple expressions and animations.

## Stage 10 — Presets, Randomization, and Workflow Polish

- Expand preset management.
- Implement selective randomization.
- Implement feature locking.
- Improve favorites.
- Improve character save/open workflows.
- Complete comparison tools.
- Complete the Character Studio walkthrough.
- Complete Game Editor integration.

**Acceptance criteria:** The complete character creation workflow is polished and reliable.

## Stage 11 — Final QA and Documentation

- Run automated tests.
- Run Blender asset QA.
- Run in-engine visual QA.
- Validate runtime consistency.
- Test extreme parameter combinations.
- Test all supported body families.
- Validate clothing and accessories.
- Review performance.
- Run benchmark character comparisons.
- Review visual regression evidence.
- Document the completed system.

**Acceptance criteria:** Character Studio is ready for regular use by Axcenity developers and eventual end-user distribution.

# 19. Agent Separation and Implementation Workflow

Use specialized agents to work on independent research and implementation areas.

The lead agent must retain responsibility for architecture, integration, and acceptance decisions.

Recommended agents:

**Agent 1 — UX and Character Studio**
- Interface research.
- Workspace redesign.
- Category navigation.
- Contextual controls.
- Viewport interactions.
- Character comparison.

**Agent 2 — Facial Customization**
- Facial reference collection.
- Facial anatomy research.
- Parameter design.
- Facial presets.
- Blender shape authoring.
- Facial QA.

**Agent 3 — Body and Anatomy**
- Anatomical references.
- Body parameter design.
- Regional deformation.
- Corrective shapes.
- Body QA.

**Agent 4 — Hair, Clothing, and Accessories**
- Groom research.
- Clothing reference collection.
- Asset library expansion.
- Capability manifests.
- Fitting and attachment QA.

**Agent 5 — Skin and Materials**
- Skin material research.
- Material controls.
- Makeup layers.
- Surface detail systems.

**Agent 6 — QA and Integration**
- Automated tests.
- Blender validation.
- Visual regression.
- Benchmark maintenance.
- Runtime consistency.
- Performance testing.

Agents must not independently redesign shared architecture or modify the same core files without coordination.

Before parallel implementation:

1. Assign ownership of files and subsystems.
2. Define interfaces and expected deliverables.
3. Agree on shared parameter and asset contracts.
4. Identify dependencies.
5. Keep independent work on separate branches.

The lead agent must review and integrate all changes.

# 20. TDD and Quality Assurance

Continue the existing RED/GREEN workflow.

For each feature:

1. Inspect existing implementation.
2. Write failing tests for the intended behavior.
3. Implement the smallest appropriate change.
4. Run targeted tests.
5. Run related subsystem tests.
6. Perform Blender validation where geometry is involved.
7. Capture in-engine visual evidence.
8. Review results before proceeding.

Do not accept a feature solely because its unit tests pass.

Visual defects can exist even when every automated test passes.

Maintain the existing benchmark character collection and extend it as additional features become available.

Test:
- Default characters.
- Extreme body shapes.
- Extreme facial parameters.
- Symmetry and asymmetry.
- Clothing combinations.
- Material variations.
- Expression combinations.
- Runtime reproduction.
- Save/reopen consistency.
- Comparison state isolation.
- Direct manipulation consistency.
- Asset capability restrictions.

# 21. Final Definition of Done

Character Studio should ultimately provide:

- An intuitive, dedicated character creation workspace.
- Realistic and stylized character families.
- Comprehensive body customization.
- Detailed facial customization.
- Direct viewport manipulation.
- Skin and makeup customization.
- A substantial hairstyle library.
- A functional clothing wardrobe.
- Accessories.
- Facial expressions.
- Animation previews.
- Character presets.
- Selective randomization.
- Reliable undo and redo.
- Character comparison.
- Capability-aware asset customization.
- Consistent runtime appearance.
- Scalable asset import and authoring.
- Automated geometry and visual QA.
- Standardized benchmark characters.
- Clear user documentation.

The final experience should feel approachable enough for someone unfamiliar with 3D modeling while offering enough depth to satisfy experienced game developers.

---

# 22. Instructions to the Implementation Agent

Do not begin by implementing every feature simultaneously.

Start with Stage 0.

Read the existing Character Studio architecture, implementation plan, progress documentation, research, visual QA documentation, and current code.

Determine which requirements are already implemented, which are partially implemented, and which are missing.

Do not recreate completed work.

Present a revised implementation plan with:
- Existing functionality that will be retained.
- Functionality requiring refinement.
- New functionality.
- Required architecture changes.
- Dependencies between stages.
- Reference research requirements.
- TDD acceptance criteria.
- Agent ownership and file boundaries.

Prioritize the overall interaction experience before expanding the asset catalogue.

The objective is not to produce the largest possible number of sliders. It is to make every customization control meaningful, intuitive, visually understandable, and capable of producing high-quality results.

Preserve the existing architecture unless a documented technical limitation justifies changing it.

Use the existing TDD, Blender automation, and visual QA workflows throughout implementation.

Do not mark a stage complete until its automated tests, visual inspection, and acceptance criteria have been satisfied.

**The ultimate goal: Build an Axcenity Character Studio that combines the accessibility of The Sims 4 with the depth and realism of professional character creation software.**
