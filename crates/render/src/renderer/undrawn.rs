//! What stays on the GPU once it stops being drawn.
//!
//! By default a caller's mesh or color texture leaves the GPU when it
//! hasn't been drawn for [`KEEP_UNDRAWN_FRAMES`] frames and
//! [`KEEP_UNDRAWN_SECONDS`] seconds, and is uploaded again the next time
//! it's drawn. That upload stalls the frame: a camera cut back to a view
//! left ten seconds ago cost 75 to 105 ms in Desert Homestead's tour
//! (docs/rendering/performance.md, "Kept within a budget").
//!
//! A renderer that shows a world to someone (the player's window, the
//! editor's viewport) is told to keep them instead
//! ([`ScenePipeline::keep_undrawn`]): meshes stay while they fit
//! `EffectiveGraphics::mesh_budget` and color textures while they fit
//! `texture_budget`. Past a budget, what wasn't drawn lately goes first,
//! the least recently drawn before the rest, and no more than it takes.
//! What was drawn lately is never freed here: textures still over their
//! budget then lose their largest levels (`texture_budget.rs`).
//!
//! Materials, UI images and terrain weights leave by the clock either
//! way: they are small, quick to make again, and some are new every frame
//! (a material whose values are tweened, weights being painted), so
//! keeping them would only pile them up.

use std::cmp::Reverse;
use std::collections::HashSet;

use super::{KEEP_UNDRAWN_FRAMES, KEEP_UNDRAWN_SECONDS, RenderStats, ScenePipeline};
use crate::EffectiveGraphics;

/// Something that could be freed: its key, the frame it was last drawn in
/// and the bytes freeing it gives back.
#[derive(Clone, Copy, Debug)]
pub(super) struct Undrawn {
    pub(super) key: u64,
    pub(super) last_drawn: u64,
    pub(super) bytes: u64,
}

/// The keys of those of `undrawn` to free so that `held` bytes fit in
/// `budget`: the least recently drawn first (of those last drawn in one
/// frame the largest, then by key, so every run frees the same ones), and
/// no more than it takes. Everything may still not fit.
pub(super) fn to_free(mut undrawn: Vec<Undrawn>, held: u64, budget: u64) -> Vec<u64> {
    let mut over = held.saturating_sub(budget);
    undrawn.sort_unstable_by_key(|one| (one.last_drawn, Reverse(one.bytes), one.key));
    let mut freed = Vec::new();
    for one in undrawn {
        if over == 0 {
            break;
        }
        over = over.saturating_sub(one.bytes);
        freed.push(one.key);
    }
    freed
}

impl ScenePipeline {
    /// The first frame that still counts as lately: what was last drawn
    /// before it has gone undrawn for [`KEEP_UNDRAWN_FRAMES`] frames and
    /// [`KEEP_UNDRAWN_SECONDS`] seconds.
    fn lately(&self) -> u64 {
        let frame = self.frame;
        let since = self.frame_clock.front().map_or(frame, |&(first, _)| first);
        since.min((frame + 1).saturating_sub(KEEP_UNDRAWN_FRAMES))
    }

    /// While keeping what's undrawn: frees color textures not drawn lately
    /// until textures fit their budget, and returns their keys (so the
    /// materials and sprite bindings holding them are dropped). Call after
    /// this frame's textures are noted and before the budget shrinks any:
    /// one that isn't being drawn is given up whole before one that is
    /// loses a level.
    pub(super) fn free_undrawn_textures(&mut self) -> HashSet<u64> {
        let (held, budget) = (self.texture_bytes.total(), self.graphics.texture_budget);
        if !self.keep_undrawn || held <= budget {
            return HashSet::new();
        }
        let lately = self.lately();
        let undrawn = self
            .textures
            .iter()
            .filter(|(_, cached)| cached.last_drawn < lately)
            .filter_map(|(&key, cached)| {
                Some(Undrawn {
                    key,
                    last_drawn: cached.last_drawn,
                    bytes: cached.gpu.as_ref()?.bytes.bytes(),
                })
            })
            .collect();
        let freed: HashSet<u64> = to_free(undrawn, held, budget).into_iter().collect();
        self.textures.retain(|key, _| !freed.contains(key));
        self.freed_for_budget += freed.len() as u64;
        freed
    }

    /// While keeping what's undrawn: frees meshes not drawn lately until
    /// meshes fit their budget.
    fn free_undrawn_meshes(&mut self, lately: u64) {
        let (held, budget) = (self.mesh_bytes.total(), self.graphics.mesh_budget);
        if held <= budget {
            return;
        }
        let undrawn = self
            .custom_meshes
            .iter()
            .filter(|(_, cached)| cached.last_drawn < lately)
            .filter_map(|(&key, cached)| {
                Some(Undrawn {
                    key,
                    last_drawn: cached.last_drawn,
                    bytes: cached.gpu.as_ref()?._bytes.bytes(),
                })
            })
            .collect();
        let freed: HashSet<u64> = to_free(undrawn, held, budget).into_iter().collect();
        self.custom_meshes.retain(|key, _| !freed.contains(key));
        self.freed_for_budget += freed.len() as u64;
    }

    /// Frees what hasn't been drawn lately: by the clock, or, while
    /// keeping what's undrawn, meshes and color textures only past their
    /// budgets (textures in [`free_undrawn_textures`](Self::free_undrawn_textures)).
    pub(super) fn forget_undrawn(&mut self) {
        let frame = self.frame;
        // The first frame of the last `KEEP_UNDRAWN_SECONDS`.
        let now = std::time::Instant::now();
        self.frame_clock.push_back((frame, now));
        while self
            .frame_clock
            .front()
            .is_some_and(|&(_, at)| now.duration_since(at).as_secs_f32() > KEEP_UNDRAWN_SECONDS)
        {
            self.frame_clock.pop_front();
        }
        let lately = self.lately();
        let recent = |last_drawn: u64| last_drawn >= lately;
        if self.keep_undrawn {
            self.free_undrawn_meshes(lately);
        } else {
            self.custom_meshes
                .retain(|_, cached| recent(cached.last_drawn));
            self.textures.retain(|_, cached| recent(cached.last_drawn));
        }
        self.full_textures
            .retain(|_, cached| recent(cached.last_drawn));
        self.ui_image_groups
            .retain(|key, _| self.full_textures.contains_key(key));
        self.sprites.forget(|key| self.textures.contains_key(&key));
        self.decal_pass
            .forget(|key| self.textures.contains_key(&key));
        self.materials.retain(|_, cached| recent(cached.last_drawn));
        self.terrain.forget(frame);
        self.surface_shaders.forget(frame);
    }
}

impl RenderStats {
    /// What to tell someone looking at the graphics settings when the
    /// world drawn needs more memory than `graphics`'s budgets allow, each
    /// a sentence or two with what it costs and what to change. Empty
    /// while everything fits.
    pub fn memory_warnings(&self, graphics: &EffectiveGraphics) -> Vec<String> {
        let megabytes = |bytes: u64| bytes.div_ceil(1024 * 1024);
        let mut warnings = Vec::new();
        if self.texture_bytes_wanted > graphics.texture_budget {
            warnings.push(format!(
                "Textures here take {} MB at full size and the texture memory budget is {} MB, \
                 so the least recently seen are drawn at lower detail. Raise Texture memory if \
                 the graphics card has room, or lower Texture quality.",
                megabytes(self.texture_bytes_wanted),
                graphics.texture_budget >> 20,
            ));
        }
        if self.mesh_bytes > graphics.mesh_budget {
            warnings.push(format!(
                "Meshes here take {} MB and their budget is {} MB (half of Texture memory). \
                 Raise Texture memory if the graphics card has room.",
                megabytes(self.mesh_bytes),
                graphics.mesh_budget >> 20,
            ));
        }
        if self.freed_for_budget > 0 {
            let what = match self.freed_for_budget {
                1 => "1 mesh or texture has".to_owned(),
                many => format!("{many} meshes and textures have"),
            };
            warnings.push(format!(
                "{what} left the graphics card to stay within the memory budgets. Each is sent \
                 again when it is next seen, which can stall a frame."
            ));
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(key: u64, last_drawn: u64, bytes: u64) -> Undrawn {
        Undrawn {
            key,
            last_drawn,
            bytes,
        }
    }

    #[test]
    fn within_the_budget_nothing_is_freed() {
        let undrawn = vec![one(1, 0, 40), one(2, 5, 40)];
        assert!(to_free(undrawn.clone(), 100, 100).is_empty());
        assert!(to_free(undrawn, 80, 100).is_empty());
    }

    #[test]
    fn past_the_budget_the_least_recently_drawn_go_first_and_no_more_than_it_takes() {
        let undrawn = vec![one(1, 30, 40), one(2, 10, 40), one(3, 20, 40)];
        // 20 over: the oldest alone makes room.
        assert_eq!(to_free(undrawn.clone(), 120, 100), [2]);
        // 50 over: the two oldest.
        assert_eq!(to_free(undrawn.clone(), 150, 100), [2, 3]);
        // More over than there is to free: all of it, oldest first.
        assert_eq!(to_free(undrawn, 500, 100), [2, 3, 1]);
    }

    #[test]
    fn of_those_last_drawn_together_the_largest_goes_first() {
        let undrawn = vec![one(1, 7, 10), one(2, 7, 90), one(3, 7, 10)];
        assert_eq!(to_free(undrawn.clone(), 150, 100), [2]);
        assert_eq!(to_free(undrawn, 195, 100), [2, 1]);
    }

    #[test]
    fn the_settings_are_told_when_a_world_needs_more_than_the_budgets() {
        const MIB: u64 = 1024 * 1024;
        let graphics = EffectiveGraphics {
            texture_budget: 1024 * MIB,
            mesh_budget: 512 * MIB,
            ..EffectiveGraphics::default()
        };
        let fits = RenderStats {
            texture_bytes: 900 * MIB,
            texture_bytes_wanted: 900 * MIB,
            mesh_bytes: 174 * MIB,
            ..RenderStats::default()
        };
        assert!(fits.memory_warnings(&graphics).is_empty());

        // Textures shrunk to fit: what they'd take at full size is over.
        let shrunk = RenderStats {
            texture_bytes: 1018 * MIB,
            texture_bytes_wanted: 1579 * MIB,
            ..fits
        };
        let warnings = shrunk.memory_warnings(&graphics);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("1579 MB") && warnings[0].contains("1024 MB"));
        assert!(warnings[0].contains("Raise Texture memory"));

        let heavy = RenderStats {
            mesh_bytes: 600 * MIB,
            freed_for_budget: 12,
            ..fits
        };
        let warnings = heavy.memory_warnings(&graphics);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("600 MB") && warnings[0].contains("512 MB"));
        assert!(warnings[1].starts_with("12 meshes and textures have left"));
        let one = RenderStats {
            freed_for_budget: 1,
            ..fits
        };
        assert!(one.memory_warnings(&graphics)[0].starts_with("1 mesh or texture has left"));
    }
}
