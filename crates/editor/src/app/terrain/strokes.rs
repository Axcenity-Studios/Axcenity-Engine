use super::*;
use crate::shortcuts::{self, Action};

/// The samples along a side `length` studs long that keep cells square
/// with `samples` across `width` studs.
pub(super) fn samples_along(samples: usize, width: f32, length: f32) -> usize {
    if width <= 0.0 || !width.is_finite() || !length.is_finite() {
        return samples;
    }
    let cells = (samples - 1) as f32 * length / width;
    (cells.round() as usize + 1).clamp(MIN_TERRAIN_SAMPLES, MAX_TERRAIN_SAMPLES)
}

impl EditorApp {
    /// 1 to 0 pick tools, `[` and `]` size the brush, Escape puts it down.
    pub(super) fn terrain_keys(&mut self, ctx: &egui::Context) {
        if self.terrain.tool.is_none() || ctx.egui_wants_keyboard_input() {
            return;
        }
        let (tool, smaller, bigger, escape) = ctx.input_mut(|input| {
            let tools: Vec<Action> = TerrainTool::all().map(Action::TerrainTool).collect();
            let tool = match shortcuts::consume_first(input, &tools) {
                Some(Action::TerrainTool(tool)) => Some(tool),
                _ => None,
            };
            (
                tool,
                shortcuts::consume(input, Action::BrushSmaller),
                shortcuts::consume(input, Action::BrushBigger),
                shortcuts::consume(input, Action::PutBrushDown),
            )
        });
        if let Some(tool) = tool {
            self.terrain.pick(Some(tool));
        }
        let brush = &mut self.terrain.brush;
        if smaller {
            brush.radius = (brush.radius / BRUSH_STEP).max(0.5);
        }
        if bigger {
            brush.radius = (brush.radius * BRUSH_STEP).min(256.0);
        }
        if escape {
            self.terrain.pick(None);
        }
    }

    /// The brush to show in the viewport, while one is being used.
    pub(in crate::app) fn terrain_brush_view(&self, ctx: &egui::Context) -> Option<TerrainBrush> {
        if self.playing() || !self.terrain.brushing() {
            return None;
        }
        let terrain = self.terrain.target?;
        self.world.get(terrain)?;
        let brush = &self.terrain.brush;
        Some(TerrainBrush {
            terrain,
            radius: brush.radius,
            falloff: brush.falloff,
            square: brush.shape == Shape::Square,
            color: self
                .terrain
                .brush_color(&self.world, theme::palette(ctx).accent),
        })
    }

    /// The mouse over the terrain while a brush is held: starts, continues
    /// and ends strokes, and Ctrl+click samples.
    pub(in crate::app) fn terrain_pointer(&mut self, pointer: TerrainPointer) {
        if self.playing() {
            return;
        }
        let (Some(terrain), Some(tool)) = (self.terrain.target, self.terrain.tool) else {
            return;
        };
        if pointer.pressed && pointer.modifiers.command {
            if let Some(point) = pointer.point {
                self.sample_terrain(terrain, tool, point);
            }
            return;
        }
        if pointer.pressed
            && self.terrain.stroke.is_none()
            && let Some(point) = pointer.point
            && let Some((operation, label)) =
                self.stroke_operation(terrain, tool, point, pointer.modifiers.shift)
        {
            self.terrain.stroke = Some(ActiveStroke {
                terrain,
                once: matches!(operation, Operation::Stamp(_)),
                operation,
                stroke: Stroke::new(),
                recorder: EditRecorder::new(),
                label,
            });
            self.stroke_to(point, pointer.seconds);
        } else if pointer.down
            && let Some(point) = pointer.point
        {
            self.stroke_to(point, pointer.seconds);
        }
        if pointer.released || !pointer.down {
            self.finish_stroke();
        }
    }

    /// What a stroke of `tool` does, starting at `point`, and its undo
    /// step's name. Shift does the opposite.
    pub(super) fn stroke_operation(
        &mut self,
        terrain: InstanceId,
        tool: TerrainTool,
        point: Vec3,
        shift: bool,
    ) -> Option<(Operation, String)> {
        let studio = &self.terrain;
        Some(match tool {
            TerrainTool::Raise if shift => (Operation::Lower, "Lower Terrain".to_owned()),
            TerrainTool::Raise => (Operation::Raise, "Raise Terrain".to_owned()),
            TerrainTool::Smooth => (Operation::Smooth, "Smooth Terrain".to_owned()),
            TerrainTool::Flatten => {
                let target = if studio.flatten_from_stroke {
                    point.y
                } else {
                    studio.flatten_height
                };
                (
                    Operation::Flatten {
                        target,
                        mode: studio.flatten_mode,
                    },
                    "Flatten Terrain".to_owned(),
                )
            }
            TerrainTool::Sculpt => {
                let mode = match (studio.sculpt, shift) {
                    (SculptMode::Hill, true) => SculptMode::Basin,
                    (SculptMode::Basin, true) => SculptMode::Hill,
                    (SculptMode::Ridge, true) => SculptMode::Valley,
                    (SculptMode::Valley, true) => SculptMode::Ridge,
                    (mode, _) => mode,
                };
                (
                    Operation::Sculpt {
                        mode,
                        step: studio.terrace_step,
                    },
                    format!("Sculpt {}", mode.label()),
                )
            }
            TerrainTool::Stamp => {
                if studio.stamp_image.is_some() && studio.stamp.image.is_none() {
                    self.load_stamp_image();
                }
                let mut stamp = self.terrain.stamp.clone();
                if shift {
                    stamp.height = -stamp.height;
                }
                (Operation::Stamp(stamp), "Stamp Terrain".to_owned())
            }
            TerrainTool::Noise => {
                let sign = if shift { -1.0 } else { 1.0 };
                (
                    Operation::Noise {
                        amplitude: studio.noise_amplitude * sign,
                        scale: studio.noise_scale,
                        seed: studio.noise_seed,
                    },
                    "Roughen Terrain".to_owned(),
                )
            }
            TerrainTool::Holes if shift => (
                Operation::Holes { fill: true },
                "Fill Terrain Holes".to_owned(),
            ),
            TerrainTool::Holes => (
                Operation::Holes { fill: false },
                "Cut Terrain Holes".to_owned(),
            ),
            TerrainTool::Water if shift => (
                Operation::Water {
                    level: 0.0,
                    erase: true,
                },
                "Remove Terrain Water".to_owned(),
            ),
            TerrainTool::Water => {
                let level = if studio.water_from_stroke {
                    point.y + studio.water_depth
                } else {
                    studio.water_level
                };
                (
                    Operation::Water {
                        level,
                        erase: false,
                    },
                    "Paint Terrain Water".to_owned(),
                )
            }
            TerrainTool::Paint if shift => (
                Operation::Erase {
                    locked: studio.locked,
                },
                "Erase Terrain Paint".to_owned(),
            ),
            TerrainTool::Paint => {
                let layers = materials::layers(&self.world, terrain);
                let Some(&layer) = layers.get(studio.paint_layer) else {
                    self.output
                        .info("Add a layer to the terrain first: painting puts a layer down.");
                    return None;
                };
                let name = self
                    .world
                    .get(layer)
                    .map_or("Layer".to_owned(), |l| l.name().to_string());
                (
                    Operation::Paint {
                        layer: studio.paint_layer,
                        mode: studio.paint_mode,
                        locked: studio.locked,
                    },
                    format!("Paint {name}"),
                )
            }
            TerrainTool::Auto | TerrainTool::Generate | TerrainTool::Erode | TerrainTool::Masks => {
                return None;
            }
        })
    }

    /// Moves the stroke under way to `point`, stamping along the way.
    pub(super) fn stroke_to(&mut self, point: Vec3, seconds: f32) {
        let Some(mut active) = self.terrain.stroke.take() else {
            return;
        };
        let brush = self.terrain.brush.clone();
        stroke_active(&mut self.world, &mut active, &brush, point, seconds);
        self.terrain.stroke = Some(active);
    }

    /// Ends the stroke under way, as one undo step if it changed anything.
    pub(super) fn finish_stroke(&mut self) {
        if let Some(active) = self.terrain.stroke.take() {
            self.record_stroke(active);
        }
    }

    /// Makes `active`'s changes one undo step, named by its label, if it
    /// changed anything. True if it did.
    pub(in crate::app) fn record_stroke(&mut self, active: ActiveStroke) -> bool {
        let Some(edit) = self
            .world
            .terrain(active.terrain)
            .and_then(|data| active.recorder.finish(data))
        else {
            return false;
        };
        let fields = history::snapshot(&self.world, active.terrain);
        self.history.record_terrain(
            &mut self.world,
            active.terrain,
            active.label,
            TerrainChange::Stroke(edit),
            fields.clone(),
            fields,
        );
        true
    }

    /// Ctrl+click: Flatten and Water take the height there, Paint the
    /// layer showing.
    pub(super) fn sample_terrain(&mut self, terrain: InstanceId, tool: TerrainTool, point: Vec3) {
        match tool {
            TerrainTool::Flatten => {
                self.terrain.flatten_height = point.y;
                self.terrain.flatten_from_stroke = false;
            }
            TerrainTool::Water => {
                self.terrain.water_level = point.y;
                self.terrain.water_from_stroke = false;
            }
            TerrainTool::Paint => {
                let Some(placement) = self.world.get(terrain).and_then(Placement::of) else {
                    return;
                };
                let Some(data) = self.world.terrain(terrain) else {
                    return;
                };
                let rules = MaterialRules::of(&self.world, terrain);
                if let Some(layer) =
                    materials::dominant_layer(data, &placement, &rules, point.x, point.z)
                {
                    self.terrain.paint_layer = layer;
                }
            }
            _ => {}
        }
    }
}

/// Moves `active` to `point` with `brush`, stamping `world`'s terrain along
/// the way and keeping what each stamp changes for undo.
pub(in crate::app) fn stroke_active(
    world: &mut World,
    active: &mut ActiveStroke,
    brush: &Brush,
    point: Vec3,
    seconds: f32,
) {
    let placement = world.get(active.terrain).and_then(Placement::of);
    let stamps = if active.once {
        // A stamp happens once, where the click was.
        let first = active.stroke.advance(point, f32::MAX, seconds, None);
        first.into_iter().take(1).collect()
    } else {
        // Scatter doesn't repeat while held still: that would pile items.
        let repeat = match active.operation {
            Operation::Scatter(_) => None,
            _ => Some(STROKE_REPEAT),
        };
        active
            .stroke
            .advance(point, brush.spacing_studs(), seconds, repeat)
    };
    if let (Some(placement), Some(data)) = (placement, world.terrain_mut(active.terrain)) {
        for center in stamps {
            for rect in brush::affected_rects(data, &placement, brush, center) {
                active.recorder.before_change(data, rect);
            }
            brush::apply(data, &placement, brush, &active.operation, center);
        }
    }
}
