//! Terrain tiles: growing a landscape with another Terrain of the same size
//! beside one (`axcenity_terrain::tiles`, `docs/terrain.md`). The new tile
//! is a copy of the one it grows from (its layers, biomes, scatter rules
//! and water settings) with ground of its own, in a data folder of its own,
//! and its edge pinned to every tile it meets. One undo step.

use axcenity_terrain::tiles::{self, BLEND_BAND, Side};

use super::*;
use crate::terrain_studio::TileFill;

impl EditorApp {
    /// Adds a tile beside `from` on `side`: its ground generated on another
    /// thread (`Continue`, shown as a job; the tile goes in when it's done)
    /// or flat (now). Refused while another job runs or the side is taken.
    pub(in crate::app) fn add_terrain_tile(
        &mut self,
        from: InstanceId,
        side: Side,
        fill: TileFill,
    ) {
        if self.terrain.job.is_some() {
            self.output
                .info("Wait for the terrain's job to finish before adding a tile.");
            return;
        }
        let (Some(placement), Some(data)) = (
            self.world.get(from).and_then(Placement::of),
            self.world.terrain(from).cloned(),
        ) else {
            return;
        };
        if !tiles::free_sides(&self.world, from).contains(&side) {
            self.output.info(format!(
                "There's already a terrain on the {} side.",
                side.label()
            ));
            return;
        }
        let title = format!("Add Terrain Tile ({})", side.label());
        let (size_x, size_z) = data.size();
        match fill {
            TileFill::Flat => {
                let height = tiles::edge_height(&data, side);
                match TerrainData::flat(size_x, size_z, height) {
                    Ok(flat) => {
                        self.place_terrain_tile(from, side, flat, title);
                    }
                    Err(err) => self.output.error(format!("Could not add the tile: {err}")),
                }
            }
            TileFill::Continue => {
                // The tile's own landscape goes on; a terrain never
                // generated goes on with the Generate tool's settings.
                let mut settings = data
                    .generation
                    .as_deref()
                    .and_then(|text| GeneratorSettings::from_json(text).ok())
                    .unwrap_or_else(|| self.terrain.generator.clone());
                let (dx, dz) = side.step();
                settings.offset[0] += dx * placement.size.x;
                settings.offset[1] += dz * placement.size.z;
                let target = Target {
                    size_x,
                    size_z,
                    world_size: Vec2::new(placement.size.x, placement.size.z),
                };
                let generator = match generate::check(&settings, target) {
                    Ok(generator) => generator,
                    Err(err) => {
                        self.output.error(format!("Could not add the tile: {err}"));
                        return;
                    }
                };
                settings.algorithm = generator.name().to_owned();
                settings.version = generator.version();
                let job = Job::spawn(move |control| {
                    let heights = generator.generate(&settings, target, control)?;
                    let mut data = TerrainData::flat(size_x, size_z, 0.0).ok()?;
                    let all = data.bounds();
                    data.heights_mut_in(all).copy_from_slice(&heights);
                    data.clamp_heights_in(all);
                    data.generation = Some(settings.to_json());
                    Some(JobResult { data, size: None })
                });
                self.terrain.job = Some(RunningJob {
                    terrain: from,
                    label: "Making a tile",
                    title,
                    job,
                    erosion: false,
                    tile: Some(side),
                });
            }
        }
    }

    /// Puts a tile with ground `data` beside `from` on `side`, its edges
    /// eased into every tile already around where it goes, as one undo
    /// step `label`. It's selected and becomes the terrain edited.
    pub(in crate::app) fn place_terrain_tile(
        &mut self,
        from: InstanceId,
        side: Side,
        mut data: TerrainData,
        label: String,
    ) -> Option<InstanceId> {
        let folder = self.project.as_ref()?.folder.clone();
        let placement = Placement::of(self.world.get(from)?)?;
        // Taken while the ground was being made.
        if !tiles::free_sides(&self.world, from).contains(&side) {
            self.output.info(format!(
                "There's already a terrain on the {} side.",
                side.label()
            ));
            return None;
        }
        let at = tiles::neighbour_placement(placement, side);
        for (on, other) in tiles::neighbours_of(&self.world, at, data.size(), None) {
            if let Some(theirs) = self.world.terrain(other) {
                // `on` is the new tile's side; it sits on theirs opposite.
                tiles::blend_edge(&mut data, theirs, on.opposite(), BLEND_BAND);
            }
        }
        let parent = self.world.get(from)?.parent()?;
        let base = self.world.get(from)?.name().to_owned();
        let copy = match self.world.duplicate(from) {
            Ok(copy) => copy,
            Err(err) => {
                self.output.error(format!("Could not add the tile: {err}"));
                return None;
            }
        };
        let name = tree::unique_name(&self.world, parent, &base);
        let taken = terrain_folders(&self.world);
        let data_folder = axcenity_project::new_terrain_folder(&folder, &name, &taken);
        let position = Vector3::new(at.position.x, at.position.y, at.position.z);
        for (property, value) in [
            ("Position", Value::Vector3(position)),
            ("DataFolder", Value::String(data_folder)),
        ] {
            self.world
                .set_property(copy, property, value)
                .expect("a Terrain has these properties");
        }
        self.world.set_name(copy, &name).expect("just made");
        self.world.set_terrain(copy, Arc::new(data));
        let end = self.world.get(parent).map_or(0, |p| p.children().len());
        let place = Place { parent, index: end };
        if !self.place(copy, Some(place), label) {
            return None;
        }
        self.select_from_viewport(Some(copy));
        self.terrain.target = Some(copy);
        Some(copy)
    }
}
