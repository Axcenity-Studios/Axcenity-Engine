//! The Hair and Clothing browsers (refinement R6 and R7, milestones CO25
//! and CO26; brief §7.1, §8.2): what each page lists and the editor state
//! behind it. Drawing is `panel/browser.rs` and `panel/worn.rs`; this file
//! is the part tests hold.
//!
//! - **What is offered.** Only items for the character's base model, in a
//!   slot of the page's category that the base has and a person picks
//!   (the slots a body type's choice fills, the underwear, are listed as
//!   what the body type wears, not offered), and whose `also_fills` slots
//!   are all such slots too. An item a slot can't take is not offered.
//! - **Drafts** (`asset-quality.md` §5) are marked "Draft". A page with
//!   finished items hides its drafts unless "Show drafts" is on (the page
//!   offers the switch only then); a page with nothing but drafts lists
//!   them without asking, so it never opens empty. One that is worn is
//!   always listed. Randomize never draws a draft, whatever is listed.
//! - **Search** matches every word typed against the label, ID, tags and
//!   `about`. **Filters**: a slot (the page's categories), a kind (one of
//!   the listed items' tags), "Recolourable" (a colour role or variants),
//!   "Fits this body" (no `limitations` range the character is outside)
//!   and "Favourites".
//! - **Who an item is made for** ([`Cut`]): the Hair and Clothing pages
//!   list what is made for the character's own body type (male or female,
//!   with the unisex items), or one body type's, or everything: a row of
//!   its own under the search, lit on the character's body type until
//!   another is picked. An item says which with a tag (`masculine`,
//!   `feminine`, `unisex`; one that says nothing is for both). Listed
//!   beside the others, an item made for the other body type comes after
//!   them in its slot and is marked with that body type's name.
//! - **The online wardrobe** (`wardrobe.rs`, `Browser::online`): the
//!   Clothing page shows the library's clothes or, in their place, the
//!   Axcenity Wardrobe's garments to download into the library.
//! - **Favourites and recently used** are editor state, kept beside the
//!   preset favourites (`presets.rs`, `favourites.json`).
//! - **A click wears, a hover previews.** A click on a tile wears it
//!   ([`CharacterWorkspace::wear`], one undo step); a click on the worn
//!   tile takes it off. Hovering a tile shows the character wearing it
//!   ([`Browser::preview`], shown in the character's place by
//!   `compare.rs`'s `shown`), with no undo step and no change to the
//!   description. Esc, moving off the tile, leaving the page or any edit
//!   puts the character back. After a click or Esc the tile under the
//!   still pointer is not previewed again until the pointer moves off it
//!   ([`Browser::muted`]).
//! - **A preview waits for the pointer to rest, and for its models.** The
//!   panel says which tile the pointer is over ([`Browse::Over`]); what
//!   that tile would show is read on the loader's thread
//!   ([`CharacterWorkspace::warm`], [`Browser::ready`]), and the panel
//!   asks for the preview ([`Browse::Hover`]) only once the pointer has
//!   stayed on the tile for [`REST`] seconds without scrolling and those
//!   models are in. Before (measured 2026-11-25, `timing_tests.rs`
//!   `browser_costs`) every tile the pointer crossed read its model on the
//!   interface's thread, 24 to 147 ms each: scrolling the page with the
//!   pointer over it stalled once per tile.
//! - **The worn items' own controls** fold to one line each
//!   ([`Browser::unfolded`], editor state).
//!
//! [`dressed`] is the one rule for what wearing an item takes off: the
//! items in its slots and in its `also_fills`, and those it `excludes` (or
//! that exclude it); it puts on what it `requires` where that fits.

use std::collections::BTreeSet;

use axcenity_avatar::{
    BaseManifest, Category as Data, Description, ItemManifest, Library, MaterialRole,
    MaterialValue, Slot, WornItem, chosen_slots,
};

use super::intent::Gesture;
use super::thumbnails::Subject;
use super::{Category, CharacterWorkspace, presets, value_of};

/// How long the pointer stays on a tile before the tile is previewed, in
/// seconds: long enough that tiles crossed on the way to another, or
/// scrolled past, aren't, and short enough to feel like a hover.
pub const REST: f64 = 0.15;

/// How many items "Recently used" keeps.
pub const MAX_RECENT_ITEMS: usize = 12;
/// How many of them the page shows in its row.
pub const RECENT_SHOWN: usize = 6;

/// A browser page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shelf {
    Hair,
    Clothing,
}

impl Shelf {
    /// The page `category` is, if it's a browser.
    pub fn of(category: Category) -> Option<Shelf> {
        match category {
            Category::Hair => Some(Shelf::Hair),
            Category::Clothing => Some(Shelf::Clothing),
            _ => None,
        }
    }

    /// The manifest's category its slots are in.
    pub fn data(self) -> Data {
        match self {
            Shelf::Hair => Data::Hair,
            Shelf::Clothing => Data::Clothing,
        }
    }
}

/// Whose garments and hairstyles a page lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cut {
    /// The ones cut for the character's own frame, and the unisex ones.
    #[default]
    Body,
    /// The ones cut for the masculine frame, and the unisex ones.
    Masculine,
    /// The ones cut for the feminine frame, and the unisex ones.
    Feminine,
    /// Every garment.
    All,
}

impl Cut {
    /// Whether it lists a garment with `tags` on a body that is feminine
    /// or not (`None`: a body without frames, which every cut fits).
    pub fn lists(self, tags: &[String], feminine_body: Option<bool>) -> bool {
        let wanted = match self {
            Cut::All => return true,
            Cut::Body => feminine_body,
            Cut::Masculine => Some(false),
            Cut::Feminine => Some(true),
        };
        axcenity_avatar::made_for_body(tags, wanted)
    }
}

pub use axcenity_avatar::{CUT_TAGS, FRAME, made_for as cut_for};

/// What `base` calls its two body types, masculine then feminine: the
/// labels of its frame's choices ("Male", "Female") where it has them.
pub fn cut_labels(base: &BaseManifest) -> (String, String) {
    let choices = base
        .parameter(FRAME)
        .map(|frame| frame.choices.as_slice())
        .unwrap_or_default();
    let label = |feminine: bool, otherwise: &str| {
        choices
            .iter()
            .find(|choice| (choice.value >= 0.5) == feminine)
            .map_or_else(|| otherwise.to_owned(), |choice| choice.label.clone())
    };
    (label(false, "Masculine"), label(true, "Feminine"))
}

impl Cut {
    /// The choice a page shows lit for this cut on a body that is feminine
    /// or not: [`Cut::Body`] is that body's own, and every cut is "all"
    /// on a body without frames.
    pub fn shown(self, feminine_body: Option<bool>) -> Cut {
        match (self, feminine_body) {
            (_, None) => Cut::All,
            (Cut::Body, Some(true)) => Cut::Feminine,
            (Cut::Body, Some(false)) => Cut::Masculine,
            (cut, _) => cut,
        }
    }

    /// The cut to keep when `picked` is chosen on such a body: the body's
    /// own is kept as [`Cut::Body`], so the page goes on following the
    /// body type when it is changed.
    pub fn picked(picked: Cut, feminine_body: Option<bool>) -> Cut {
        if picked == Cut::Body.shown(feminine_body) && feminine_body.is_some() {
            Cut::Body
        } else {
            picked
        }
    }
}

/// Whether `description`'s body is the feminine frame rather than the
/// masculine one (`avatar::feminine_frame`: by the nearer choice of a
/// frame that is one); `None` when its base has no frames.
pub fn feminine_body(description: &Description, base: &BaseManifest) -> Option<bool> {
    axcenity_avatar::feminine_frame(base, description)
}

/// A page's search and filters (editor state).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filters {
    pub search: String,
    /// Whose garments are listed.
    pub cut: Cut,
    /// One slot of the page (`None`: all of them).
    pub slot: Option<String>,
    /// One tag (`None`: any).
    pub kind: Option<String>,
    /// Only items with a colour role or variants.
    pub recolourable: bool,
    /// Only items with no `limitations` range the character is outside.
    pub fits: bool,
    /// Only favourites.
    pub favourites: bool,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            search: String::new(),
            cut: Cut::default(),
            slot: None,
            kind: None,
            recolourable: false,
            fits: true,
            favourites: false,
        }
    }
}

impl Filters {
    /// Whether anything narrows the list (the "Recently used" row shows
    /// only on the whole list). Who the items are made for doesn't: it is
    /// which wardrobe is open, with its own "nothing worn" tiles and
    /// recently used row.
    pub fn narrowed(&self) -> bool {
        let default = Filters::default();
        !self.search.trim().is_empty()
            || self.slot.is_some()
            || self.kind.is_some()
            || self.recolourable
            || self.fits != default.fits
            || self.favourites
    }
}

/// A tile: wearing `item` in `slot`, or nothing there (`None`: "Bald").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub slot: String,
    pub item: Option<String>,
}

/// What a browser page asks for: its own view, never the character.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Browse {
    /// A page's search and filters, whole.
    Filters(Shelf, Filters),
    /// Lists drafts (both pages) beside the finished items, marked
    /// "Draft". A page with nothing but drafts lists them either way.
    ShowDrafts(bool),
    /// The Clothing page shows the online wardrobe (`wardrobe.rs`) in
    /// place of the library's clothes, or the library's again.
    Online(bool),
    /// The tile the pointer is over (`None`: none), whether or not it is
    /// previewed yet: what it would show is loaded in the background.
    Over(Option<Pick>),
    /// The tile to preview (`None`: none): the one under the pointer, once
    /// the pointer has rested on it and its models are loaded.
    Hover(Option<Pick>),
    /// A tile was clicked (what the click wears follows as its own
    /// intent): its preview ends and it is muted, so the tile under the
    /// still pointer isn't previewed again until the pointer moves off it.
    Clicked(Pick),
    /// Unfolds (or folds) the worn item's own controls, by its ID.
    Unfold(String, bool),
}

/// The browsers' editor state: never in a description, never undone.
#[derive(Clone, Debug, Default)]
pub struct Browser {
    pub hair: Filters,
    pub clothing: Filters,
    pub show_drafts: bool,
    /// The Clothing page shows the online wardrobe, not the library.
    pub online: bool,
    pub hovered: Option<Pick>,
    /// The tile the pointer is over, previewed yet or not.
    pub over: Option<Pick>,
    /// Whether what [`Browser::over`] would show is loaded (or can't be):
    /// its preview can start without reading a model on the interface's
    /// thread. Brought up to date each frame by the shell.
    pub ready: bool,
    /// What the preview shows in the character's place while a tile is
    /// hovered: the character wearing it.
    pub preview: Option<Description>,
    /// The tile under the pointer when a preview ended (Esc, an edit such
    /// as the click that wears it or takes it off): no Hover is taken
    /// until the pointer moves off it, or the panel, still seeing the
    /// pointer there (or a tile shifted under it), would bring a preview
    /// back.
    pub muted: Option<Pick>,
    /// The worn items whose own controls are unfolded, by ID (folded to
    /// one line each otherwise).
    pub unfolded: BTreeSet<String>,
}

impl Browser {
    pub fn filters(&self, shelf: Shelf) -> &Filters {
        match shelf {
            Shelf::Hair => &self.hair,
            Shelf::Clothing => &self.clothing,
        }
    }

    pub fn filters_mut(&mut self, shelf: Shelf) -> &mut Filters {
        match shelf {
            Shelf::Hair => &mut self.hair,
            Shelf::Clothing => &mut self.clothing,
        }
    }

    /// The tile being previewed: the one under the pointer.
    pub fn trying(&self) -> Option<&Pick> {
        self.hovered.as_ref()
    }

    /// Whether a tile is shown on the character in its place.
    pub fn previewing(&self) -> bool {
        self.preview.is_some()
    }

    /// Forgets the tile hovered: the character is shown again. The tile
    /// under the pointer is muted (see [`Browser::muted`]).
    pub fn end_preview(&mut self) {
        if let Some(pick) = self.hovered.take() {
            self.muted = Some(pick);
        }
        self.preview = None;
    }

    /// Whether the panel should report `shown`, the tile to preview now
    /// (the one the pointer has rested on, or none): when it changed, and
    /// once the pointer has moved off a muted tile (so the mute ends).
    /// `over` is the tile under the pointer, rested or not, and `moved`
    /// whether the pointer moved this frame: a still pointer never ends a
    /// mute, though the tiles may shift under it, and nor does a pointer
    /// that moves about on the muted tile.
    pub fn hover_changed(&self, over: &Option<Pick>, shown: &Option<Pick>, moved: bool) -> bool {
        match &self.muted {
            Some(_) => moved && *over != self.muted,
            None => *shown != self.hovered,
        }
    }

    /// The pointer is over `pick` now (`None`: no tile). Returns false,
    /// changing nothing, for the muted tile.
    pub fn hover(&mut self, pick: Option<Pick>) -> bool {
        if pick.is_some() && pick == self.muted {
            return false;
        }
        self.muted = None;
        self.hovered = pick;
        true
    }

    /// Works out the preview again for `description` (after a hover):
    /// `None` when the tile is what's worn already.
    pub fn update_preview(&mut self, description: &Description, library: &Library) {
        self.preview = self.trying().and_then(|pick| {
            let shown = dressed(description, library, &pick.slot, pick.item.as_deref());
            (shown != *description).then_some(shown)
        });
    }
}

/// `description` wearing `item` in `slot` (or nothing there): what was in
/// its slots taken off (its own and `also_fills`), and anything it
/// conflicts with (`excludes`, either way); what it `requires` put on
/// where the library has it for this base. Nothing else changes.
pub fn dressed(
    description: &Description,
    library: &Library,
    slot: &str,
    item: Option<&str>,
) -> Description {
    let mut out = description.clone();
    let wearing = item.and_then(|id| library.item(id));
    // The slots it frees: the one asked for and, for an item, all it fills.
    let mut freed: BTreeSet<&str> = BTreeSet::from([slot]);
    if let Some(wearing) = wearing {
        freed.extend(wearing.slots().map(String::as_str));
    }
    out.items.retain(|worn| match library.item(&worn.id) {
        // An item the library doesn't know stays as it was.
        None => true,
        Some(other) => {
            !other.slots().any(|s| freed.contains(s.as_str()))
                && wearing.is_none_or(|wearing| !wearing.conflicts_with(other))
        }
    });
    if let Some(wearing) = wearing {
        out.items.push(WornItem::new(wearing.id.clone()));
        for needed in &wearing.requires {
            let fits = library
                .item(needed)
                .is_some_and(|needed| needed.bases.contains(&description.base));
            if fits && !out.items.iter().any(|worn| worn.id == *needed) {
                out.items.push(WornItem::new(needed.clone()));
            }
        }
    }
    out
}

/// One tile of a page.
#[derive(Clone, Debug)]
pub struct Entry<'a> {
    pub item: &'a ItemManifest,
    /// The slot's label ("Top").
    pub slot: &'a str,
    pub draft: bool,
    pub worn: bool,
    pub favourite: bool,
    /// A limitation the character is outside of (its note).
    pub misfit: Option<String>,
    /// Made for the other body type than the character's (`Some(true)`:
    /// for the feminine one), when it says so and the body has a type.
    pub other_cut: Option<bool>,
    /// What wearing it takes off besides its own slot (labels).
    pub takes_off: Vec<String>,
}

/// What a page lists.
#[derive(Clone, Debug, Default)]
pub struct Listing<'a> {
    /// The page's slots a person picks in (its categories).
    pub slots: Vec<&'a Slot>,
    /// The tiles that pass the search and filters, favourites first.
    pub entries: Vec<Entry<'a>>,
    /// The items last worn, newest first (only on the whole list).
    pub recent: Vec<Entry<'a>>,
    /// The tags of the items offered, for the kind filter (but the ones
    /// that say who a garment is cut for: [`Cut`] is their filter).
    pub kinds: Vec<String>,
    /// Whether any offered item says who it is cut for (so the page
    /// offers the choice).
    pub cuts: bool,
    /// Offered drafts left out because "Show drafts" is off.
    pub hidden_drafts: usize,
    /// Offered items that aren't drafts. With none, the page lists its
    /// drafts whatever "Show drafts" says.
    pub finished: usize,
    /// Offered drafts, listed or not. The page offers "Show drafts" only
    /// when it has these and finished items both.
    pub drafts: usize,
    /// Items offered at all before the search and filters.
    pub offered: usize,
}

/// The page's slots on `base` that a person picks in.
pub fn page_slots<'a>(base: &'a BaseManifest, shelf: Shelf, chosen: &[String]) -> Vec<&'a Slot> {
    base.slots
        .iter()
        .filter(|slot| slot.category == shelf.data() && !chosen.contains(&slot.id))
        .collect()
}

/// Whether `item` is offered on a page whose pickable slots are `slots`,
/// for base `base`: made for it, in one of those slots, every slot it also
/// fills one of them too.
pub fn offered(item: &ItemManifest, base: &str, slots: &[&Slot]) -> bool {
    let takes = |id: &String| slots.iter().any(|slot| slot.id == *id);
    item.bases.iter().any(|b| b == base) && item.slots().all(takes)
}

/// Whether a person can recolour `item`: a material slot with a colour
/// role, or variants to pick from.
pub fn recolourable(item: &ItemManifest) -> bool {
    !item.variants.is_empty()
        || item
            .materials
            .iter()
            .any(|slot| matches!(slot.role, Some(role) if role != MaterialRole::Other))
}

/// The first of `item`'s limitations on a base parameter whose range the
/// character is outside, as its note.
pub fn misfit(
    item: &ItemManifest,
    description: &Description,
    base: &BaseManifest,
) -> Option<String> {
    misfits(item, description, base, false)
}

/// As [`misfit`]; `beside_frame` leaves out limits on the body's frame
/// (for a page that was asked for another frame's garments: [`Cut`]).
fn misfits(
    item: &ItemManifest,
    description: &Description,
    base: &BaseManifest,
    beside_frame: bool,
) -> Option<String> {
    item.limitations.iter().find_map(|limit| {
        let parameter = base.parameter(limit.parameter.as_deref()?)?;
        if beside_frame && parameter.id == FRAME {
            return None;
        }
        let [low, high] = limit.range?;
        let value = value_of(description, Some(base), &parameter.id);
        (value < low.min(high) || value > low.max(high)).then(|| limit.note.clone())
    })
}

/// Whether every word of `search` is in the item's label, ID, tags or
/// `about` (case aside).
pub fn matches(item: &ItemManifest, search: &str) -> bool {
    let haystack = format!(
        "{} {} {} {}",
        item.label,
        item.id,
        item.tags.join(" "),
        item.about.as_deref().unwrap_or("")
    )
    .to_lowercase();
    search
        .split_whitespace()
        .all(|word| haystack.contains(&word.to_lowercase()))
}

/// What the body type wears on page `shelf` (the underwear), by slot
/// label, item label: listed as said, never offered.
pub fn body_type_wears(
    library: &Library,
    description: &Description,
    shelf: Shelf,
) -> Vec<(String, String)> {
    let Some(base) = library.base(&description.base) else {
        return Vec::new();
    };
    let chosen = chosen_slots(library, &base.id);
    base.slots
        .iter()
        .filter(|slot| slot.category == shelf.data() && chosen.contains(&slot.id))
        .filter_map(|slot| {
            let worn = description
                .items
                .iter()
                .find_map(|worn| library.item(&worn.id).filter(|item| item.slot == slot.id))?;
            Some((slot.label.clone(), worn.label.clone()))
        })
        .collect()
}

/// What the page `shelf` lists for `description`. `show_drafts` is the
/// page's switch: drafts are listed with it on, and also when the page has
/// no finished item to list at all.
#[allow(clippy::too_many_arguments)]
pub fn list<'a>(
    library: &'a Library,
    description: &Description,
    shelf: Shelf,
    filters: &Filters,
    show_drafts: bool,
    favourites: &BTreeSet<String>,
    recent: &[String],
) -> Listing<'a> {
    let Some(base) = library.base(&description.base) else {
        return Listing::default();
    };
    let chosen = chosen_slots(library, &base.id);
    let slots = page_slots(base, shelf, &chosen);
    let worn_ids: BTreeSet<&str> = description.items.iter().map(|w| w.id.as_str()).collect();
    let feminine = feminine_body(description, base);
    let entry = |item: &'a ItemManifest| -> Entry<'a> {
        let slot = slots
            .iter()
            .find(|slot| slot.id == item.slot)
            .map_or(item.slot.as_str(), |slot| slot.label.as_str());
        let after = dressed(description, library, &item.slot, Some(&item.id));
        let takes_off = description
            .items
            .iter()
            .filter(|worn| !after.items.iter().any(|kept| kept.id == worn.id))
            .filter_map(|worn| library.item(&worn.id))
            .filter(|other| other.slot != item.slot)
            .map(|other| other.label.clone())
            .collect();
        Entry {
            item,
            slot,
            draft: item.is_draft(),
            worn: worn_ids.contains(item.id.as_str()),
            favourite: favourites.contains(&item.id),
            misfit: misfit(item, description, base),
            other_cut: cut_for(&item.tags).filter(|cut| feminine.is_some_and(|body| body != *cut)),
            takes_off,
        }
    };
    let all: Vec<&ItemManifest> = library
        .items
        .values()
        .filter(|item| offered(item, &base.id, &slots))
        .collect();
    let drafts = all.iter().filter(|item| item.is_draft()).count();
    let finished = all.len() - drafts;
    // A page with nothing but drafts lists them: it never opens empty.
    let show_drafts = show_drafts || finished == 0;
    let mut hidden_drafts = 0;
    let shown: Vec<&ItemManifest> = all
        .into_iter()
        .filter(|item| {
            let keep = show_drafts || !item.is_draft() || worn_ids.contains(item.id.as_str());
            hidden_drafts += usize::from(!keep);
            keep
        })
        .collect();
    let mut kinds: Vec<String> = shown.iter().flat_map(|item| item.tags.clone()).collect();
    kinds.sort();
    kinds.dedup();
    let cuts = kinds.iter().any(|kind| CUT_TAGS.contains(&kind.as_str()));
    kinds.retain(|kind| !CUT_TAGS.contains(&kind.as_str()));
    // Asked for another frame's garments, the frame is no misfit.
    let fits = |entry: &Entry| {
        entry.worn
            || match filters.cut {
                Cut::Body => entry.misfit.is_none(),
                _ => misfits(entry.item, description, base, true).is_none(),
            }
    };
    let mut entries: Vec<Entry> = shown
        .iter()
        .filter(|item| filters.slot.as_ref().is_none_or(|slot| item.slot == *slot))
        .filter(|item| {
            filters
                .kind
                .as_ref()
                .is_none_or(|kind| item.tags.contains(kind))
        })
        .filter(|item| !filters.recolourable || recolourable(item))
        .filter(|item| matches(item, &filters.search))
        .filter(|item| {
            filters.cut.lists(&item.tags, feminine) || worn_ids.contains(item.id.as_str())
        })
        .map(|item| entry(item))
        .filter(|entry| !filters.fits || fits(entry))
        .filter(|entry| !filters.favourites || entry.favourite)
        .collect();
    // By slot in the base's order, what is made for this body type (and
    // the unisex items) before the other's, then label; favourites first
    // (a stable sort keeps the rest in order).
    let slot_order = |entry: &Entry| slots.iter().position(|slot| slot.id == entry.item.slot);
    entries.sort_by(|a, b| {
        slot_order(a)
            .cmp(&slot_order(b))
            .then_with(|| a.other_cut.is_some().cmp(&b.other_cut.is_some()))
            .then_with(|| a.item.label.cmp(&b.item.label))
    });
    entries.sort_by_key(|entry| !entry.favourite);
    let recent = if filters.narrowed() {
        Vec::new()
    } else {
        // What is worn is lit in the grid already.
        recent
            .iter()
            .filter(|id| !worn_ids.contains(id.as_str()))
            .filter_map(|id| shown.iter().find(|item| item.id == *id))
            .filter(|item| filters.cut.lists(&item.tags, feminine))
            .take(RECENT_SHOWN)
            .map(|item| entry(item))
            .collect()
    };
    Listing {
        offered: shown.len() + hidden_drafts,
        slots,
        entries,
        recent,
        kinds,
        cuts,
        hidden_drafts,
        finished,
        drafts,
    }
}

/// `recent` with `id` at the front, once, at most [`MAX_RECENT_ITEMS`].
pub fn remember(recent: &mut Vec<String>, id: &str) {
    recent.retain(|other| other != id);
    recent.insert(0, id.to_owned());
    recent.truncate(MAX_RECENT_ITEMS);
}

/// What the worn item's controls can change: only what the item declares
/// (brief §7.2, §8.3, §14.1; plan R6 "every style declares only what it
/// supports").
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Declared {
    /// Colour buttons by role: the role and the material slots that have
    /// it (a hair item's primary is its colour, secondary its highlights,
    /// accent its roots). Only slots whose role is set, and that offer a
    /// colour value.
    pub roles: Vec<(MaterialRole, Vec<String>)>,
    /// Hair only: the slots whose natural colour can be set (they offer
    /// `melanin`), by role.
    pub natural: Vec<(MaterialRole, Vec<String>)>,
    /// The item's variants, by ID.
    pub variants: Vec<String>,
    /// The item's own parameters, by ID.
    pub parameters: Vec<String>,
}

impl Declared {
    /// What `item` declares.
    pub fn of(item: &ItemManifest) -> Self {
        let roles_of = |wanted: &dyn Fn(&axcenity_avatar::MaterialSlot) -> bool| {
            let mut out: Vec<(MaterialRole, Vec<String>)> = Vec::new();
            for role in [
                MaterialRole::Primary,
                MaterialRole::Secondary,
                MaterialRole::Accent,
            ] {
                let slots: Vec<String> = item
                    .materials
                    .iter()
                    .filter(|slot| slot.role == Some(role) && wanted(slot))
                    .map(|slot| slot.name.clone())
                    .collect();
                if !slots.is_empty() {
                    out.push((role, slots));
                }
            }
            out
        };
        let offers = |slot: &axcenity_avatar::MaterialSlot, name: &str| {
            slot.offered().iter().any(|spec| spec.name == name)
        };
        let groom = item.kind == axcenity_avatar::ItemKind::Groom;
        Self {
            roles: roles_of(&|slot| offers(slot, COLOUR_VALUE)),
            natural: if groom {
                roles_of(&|slot| {
                    slot.shading == axcenity_avatar::ShadingModel::Hair && offers(slot, "melanin")
                })
            } else {
                Vec::new()
            },
            variants: item.variants.iter().map(|v| v.id.clone()).collect(),
            parameters: item.parameters.iter().map(|p| p.id.clone()).collect(),
        }
    }

    /// Whether the item offers nothing to change.
    pub fn is_empty(&self) -> bool {
        self.roles.is_empty()
            && self.natural.is_empty()
            && self.variants.is_empty()
            && self.parameters.is_empty()
    }
}

/// The value a colour button sets: every shading model's `tint`.
pub const COLOUR_VALUE: &str = "tint";

/// What a colour role is called on an item of `kind`: a hair item's
/// secondary is its highlights, its accent its roots.
pub fn role_label(role: MaterialRole, groom: bool) -> &'static str {
    match (role, groom) {
        (MaterialRole::Primary, true) => "Hair colour",
        (MaterialRole::Secondary, true) => "Highlights",
        (MaterialRole::Accent, true) => "Roots",
        (MaterialRole::Primary, false) => "Primary colour",
        (MaterialRole::Secondary, false) => "Secondary colour",
        (MaterialRole::Accent, false) => "Accent colour",
        (MaterialRole::Other, _) => "Colour",
    }
}

/// Natural hair colours: a name, `melanin` and `redness` (the Hair
/// shading model's values, relative to the colour the style is made in,
/// which the built-in styles make a medium brown) and a swatch to draw.
pub const HAIR_COLOURS: [(&str, f32, f32, [u8; 3]); 9] = [
    ("Black", 1.0, 0.0, [24, 20, 18]),
    ("Dark brown", 0.6, 0.0, [52, 36, 26]),
    ("Brown", 0.0, 0.0, [92, 62, 40]),
    ("Light brown", -0.35, 0.05, [132, 96, 64]),
    ("Dark blonde", -0.6, 0.1, [168, 132, 88]),
    ("Blonde", -0.85, 0.15, [214, 184, 128]),
    ("Strawberry blonde", -0.7, 0.5, [204, 148, 102]),
    ("Auburn", 0.2, 0.6, [118, 52, 30]),
    ("Red", -0.2, 0.9, [168, 74, 38]),
];

impl CharacterWorkspace {
    /// Does what a browser page asked of its own view.
    pub fn browse(&mut self, change: Browse) {
        match change {
            Browse::Filters(shelf, filters) => *self.browser.filters_mut(shelf) = filters,
            Browse::ShowDrafts(on) => {
                self.browser.show_drafts = on;
                // A draft tile previewed goes when drafts are hidden.
                if !on {
                    let library = self.library();
                    let draft = self
                        .browser
                        .hovered
                        .as_ref()
                        .and_then(|pick| pick.item.as_deref())
                        .and_then(|id| library.item(id))
                        .is_some_and(|item| {
                            item.is_draft()
                                && !self.description.items.iter().any(|w| w.id == item.id)
                        });
                    if draft {
                        self.browser.hovered = None;
                    }
                }
            }
            Browse::Online(on) => {
                self.browser.online = on;
                // A tile previewed was one of the library's.
                self.browser.hovered = None;
            }
            Browse::Over(pick) => {
                self.browser.over = pick;
                self.warm_tile();
                return;
            }
            Browse::Hover(pick) => {
                if !self.browser.hover(pick) {
                    return;
                }
            }
            Browse::Clicked(pick) => {
                self.browser.hovered = None;
                self.browser.muted = Some(pick);
            }
            Browse::Unfold(item, open) => {
                if open {
                    self.browser.unfolded.insert(item);
                } else {
                    self.browser.unfolded.remove(&item);
                }
                return;
            }
        }
        let library = self.library();
        self.browser.update_preview(&self.description, &library);
        self.refresh();
    }

    /// Starts reading what the tile under the pointer would show (the
    /// character wearing it) on the loader's thread, and notes whether it
    /// is all in ([`Browser::ready`]). Called when the pointer comes over a
    /// tile and each frame while it waits there.
    pub(super) fn warm_tile(&mut self) {
        let ready = match &self.browser.over {
            Some(pick) => {
                let library = self.library();
                let shown = dressed(
                    &self.description,
                    &library,
                    &pick.slot,
                    pick.item.as_deref(),
                );
                self.warm(&shown, &library)
            }
            None => false,
        };
        self.browser.ready = ready;
    }

    /// Asks the loader for every model `description` is drawn from that
    /// isn't loaded, without waiting: whether they are all in, or can't
    /// be (a character that can't be resolved, a model that failed, which
    /// the preview then says as it always did).
    pub(super) fn warm(&self, description: &Description, library: &Library) -> bool {
        use axcenity_assets::LoadStatus;
        let Ok(resolved) = axcenity_avatar::resolve(library, description) else {
            return true;
        };
        let folder = axcenity_avatar::LIBRARY_FOLDER;
        let mut ready = true;
        for part in std::iter::once(&resolved.body).chain(&resolved.parts) {
            let ticket = self
                .assets
                .request_model(&format!("{folder}/{}", part.model));
            ready &= ticket.status() != LoadStatus::Loading;
        }
        ready
    }

    /// Takes a tile's preview off the character.
    pub fn end_preview(&mut self) {
        if self.drop_preview() {
            self.refresh();
        }
    }

    /// Ends a tile's preview without showing the character again (the
    /// caller does): whether one was on. Every change to the description
    /// calls it, so what is edited is what is seen.
    pub(super) fn drop_preview(&mut self) -> bool {
        let was = self.browser.trying().is_some() || self.browser.previewing();
        self.browser.end_preview();
        was
    }

    /// Marks item `id` a favourite, or not (editor state).
    pub fn toggle_item_favourite(&mut self, id: &str) {
        if !self.item_favourites.remove(id) {
            self.item_favourites.insert(id.to_owned());
        }
        presets::write_item_lists(&self.folder, &self.item_favourites, &self.recent_items);
    }

    /// Changes the worn item `item` (nothing when it isn't worn), as one
    /// undo step or part of a drag's.
    fn change_worn(&mut self, item: &str, change: impl FnOnce(&mut WornItem), gesture: Gesture) {
        if !self.description.items.iter().any(|worn| worn.id == item) {
            return;
        }
        self.transact(
            |description| {
                if let Some(worn) = description.items.iter_mut().find(|worn| worn.id == item) {
                    change(worn);
                }
            },
            gesture,
        );
    }

    /// Picks the worn item's variant (`None`: as made). What the variant
    /// sets is the variant's again: the person's own colours and values
    /// for those slots and parameters go.
    pub fn set_item_variant(&mut self, item: &str, variant: Option<&str>) {
        let library = self.library();
        let Some(manifest) = library.item(item) else {
            return;
        };
        let chosen = match variant {
            Some(id) => match manifest.variant(id) {
                Some(chosen) => Some(chosen.clone()),
                // A variant the item doesn't declare is never set.
                None => return,
            },
            None => None,
        };
        self.change_worn(
            item,
            |worn| {
                if let Some(chosen) = &chosen {
                    for slot in chosen.materials.keys() {
                        worn.materials.remove(slot);
                    }
                    for id in chosen.parameters.keys() {
                        worn.parameters.remove(id);
                    }
                }
                worn.variant = chosen.map(|chosen| chosen.id);
            },
            Gesture::Commit,
        );
    }

    /// Sets (or clears) `values` on each of the worn item's material
    /// `slots` that it declares and that offers the value.
    pub fn set_item_material(
        &mut self,
        item: &str,
        slots: &[String],
        values: &[(String, Option<MaterialValue>)],
        gesture: Gesture,
    ) {
        let library = self.library();
        let Some(manifest) = library.item(item) else {
            return;
        };
        let offered = |slot: &str, value: &str| {
            manifest
                .material(slot)
                .is_some_and(|slot| slot.offered().iter().any(|spec| spec.name == value))
        };
        self.change_worn(
            item,
            |worn| {
                for slot in slots {
                    for (value, to) in values {
                        if !offered(slot, value) {
                            continue;
                        }
                        let held = worn.materials.entry(slot.clone()).or_default();
                        match to {
                            Some(to) => {
                                held.insert(value.clone(), *to);
                            }
                            None => {
                                held.remove(value);
                            }
                        }
                        if held.is_empty() {
                            worn.materials.remove(slot);
                        }
                    }
                }
            },
            gesture,
        );
    }

    /// Sets the worn item's own parameter `id`, held in its range.
    pub fn set_item_parameter(&mut self, item: &str, id: &str, value: f32, gesture: Gesture) {
        let library = self.library();
        let Some(parameter) = library.item(item).and_then(|item| item.parameter(id)) else {
            return;
        };
        let [low, high] = parameter.range;
        let value = value.clamp(low.min(high), low.max(high));
        self.change_worn(
            item,
            |worn| {
                worn.parameters.insert(id.to_owned(), value);
            },
            gesture,
        );
    }
}

impl CharacterWorkspace {
    /// The pictures page `shelf`'s tiles want: each listed item, and
    /// wearing nothing in each of its slots, on the character being edited
    /// (a hairstyle framed on the head and shoulders, clothes on the whole
    /// body, with the garments on layers over the tile's taken off so its
    /// own shows: [`pictured`]).
    pub(super) fn browser_tiles(&self, library: &Library, shelf: Shelf) -> Vec<(String, Subject)> {
        let description = &self.description;
        let listing = list(
            library,
            description,
            shelf,
            self.browser.filters(shelf),
            self.browser.show_drafts,
            &self.item_favourites,
            &self.recent_items,
        );
        let subject = |slot: &str, item: Option<&str>| {
            let shown = dressed(description, library, slot, item);
            match shelf {
                Shelf::Hair => Subject::Hair(shown),
                Shelf::Clothing => Subject::Character(pictured(shown, library, slot, item)),
            }
        };
        let mut tiles: Vec<(String, Subject)> = listing
            .slots
            .iter()
            .map(|slot| {
                (
                    super::panel::none_picture(&slot.id),
                    subject(&slot.id, None),
                )
            })
            .collect();
        for entry in listing.entries.iter().chain(&listing.recent) {
            if tiles.iter().any(|(id, _)| *id == entry.item.id) {
                continue;
            }
            let item = &entry.item;
            tiles.push((item.id.clone(), subject(&item.slot, Some(&item.id))));
        }
        tiles
    }
}

/// What a clothing tile's picture shows: `shown` (the character wearing
/// the tile's `item` in `slot`, or nothing there) without the garments on
/// layers over it, so underwear isn't hidden under a jacket. The layer of
/// an empty slot's tile is the lowest of the items made for that slot.
pub fn pictured(
    mut shown: Description,
    library: &Library,
    slot: &str,
    item: Option<&str>,
) -> Description {
    let layer = match item.and_then(|id| library.item(id)) {
        Some(item) => item.layer,
        None => library
            .items
            .values()
            .filter(|other| other.slot == slot && other.bases.contains(&shown.base))
            .filter_map(|other| other.layer)
            .min(),
    };
    let Some(layer) = layer else {
        return shown;
    };
    shown.items.retain(|worn| {
        library
            .item(&worn.id)
            .and_then(|other| other.layer)
            .is_none_or(|other| other <= layer)
    });
    shown
}
