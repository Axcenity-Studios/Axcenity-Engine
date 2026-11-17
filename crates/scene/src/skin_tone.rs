//! Skin tone from pigment amounts (Character Studio refinement R5, CO7's
//! first slice): the `melanin`, `undertone` and `redness` values of a
//! character's Skin material (`avatar`'s `material.rs`) turned into one
//! colour ratio that multiplies the authored albedo **in linear light**.
//! All three at 0 give exactly (1, 1, 1): the map is drawn as authored.
//!
//! The curve is the tone table of
//! `docs/development/character-studio/references/skin/README.md` §1.3:
//! nine depths (`melanin` -0.25 to 0.75 in steps of 0.125, depth 3 the
//! authored tone) by four undertones (cool -0.7, neutral, warm 0.7, and
//! olive, which is `undertone` 0.5 with `redness` -0.6). Between entries
//! it is interpolated in **optical density** (minus the log of the
//! reflectance), where pigments add (Beer and Lambert; Tsumura 2003), so a
//! step of melanin darkens blue more than red and deep tones don't turn
//! grey. Past the table's ends the curve keeps going and slows down, so
//! the whole -1 to 1 range does something and stays inside real albedos.
//!
//! This is step A of the references (one ratio per material, on the CPU):
//! lips, nails, palms and soles tone with the rest because they are in the
//! same albedo. Per-texel toning with masks waits for the Skin shading
//! model. The table's colours are indicative; the build is meant to
//! recompute them from the pigment spectra, and only [`TONES`] changes then.

/// The tone table, sRGB: rows are depths 1 to 9, columns cool, neutral,
/// warm and olive (references/skin §1.3; tone 10, depth 3 neutral, is the
/// authored albedo's tone).
const TONES: [[u32; 4]; 9] = [
    [0xd9aca0, 0xd9ac9e, 0xdaac9d, 0xd9ad93],
    [0xcb9986, 0xcc9984, 0xce9980, 0xcc9a7a],
    [0xbb8871, 0xbd876e, 0xbf8669, 0xbd8865],
    [0xaa7760, 0xac765c, 0xaf7557, 0xad7655],
    [0x976652, 0x99664e, 0x9c644a, 0x9b6549],
    [0x835847, 0x855743, 0x88553f, 0x87563f],
    [0x6d4a3d, 0x6f493a, 0x724836, 0x714837],
    [0x573d34, 0x593d31, 0x5b3c2e, 0x5a3c2f],
    [0x41312a, 0x433128, 0x4b3327, 0x473227],
];
/// `melanin` at the table's first depth, and per depth.
const FIRST_DEPTH: f32 = -0.25;
const DEPTH_STEP: f32 = 0.125;
/// The `undertone` of the cool and warm columns.
const UNDERTONE_SPAN: f32 = 0.7;
/// The olive column: this `undertone` with this `redness`.
const OLIVE: (f32, f32) = (0.5, -0.6);
/// How many depth steps the curve still goes past the table's light and
/// deep ends at most: it slows down as it goes (see [`depth_density`]).
/// One step past the light end is about the lightest the authored map can
/// honestly give (all its melanin taken out: references/skin §1.3); two
/// past the deep end reach about Monk 10.
const PAST_LIGHT_END: f32 = 1.0;
const PAST_DEEP_END: f32 = 2.0;

/// The ratio (linear red, green, blue) the albedo is multiplied by for
/// `melanin`, `undertone` and `redness` (each -1 to 1, 0 neutral; values
/// outside are held to it, a NaN counts as 0). Exactly 1 for all three 0.
pub(crate) fn ratio(melanin: f32, undertone: f32, redness: f32) -> [f32; 3] {
    let held = |value: f32| {
        if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    };
    let (melanin, undertone, redness) = (held(melanin), held(undertone), held(redness));
    let neutral = density(0.0, 0.0, 0.0);
    let toned = density(melanin, undertone, redness);
    std::array::from_fn(|channel| (neutral[channel] - toned[channel]).exp())
}

/// The table's optical density for these values.
fn density(melanin: f32, undertone: f32, redness: f32) -> [f32; 3] {
    let column = |column: usize| depth_density(melanin, column);
    let (cool, neutral, warm, olive) = (column(0), column(1), column(2), column(3));
    // Undertone: along the line from neutral to warm or to cool, kept
    // going past them (to ±1) at the same rate.
    let toward = if undertone < 0.0 { cool } else { warm };
    let along = undertone.abs() / UNDERTONE_SPAN;
    // Redness per unit: what olive changes against the same undertone
    // without it.
    let olive_undertone = OLIVE.0 / UNDERTONE_SPAN;
    std::array::from_fn(|c| {
        let base = neutral[c] + (toward[c] - neutral[c]) * along;
        let unreddened = neutral[c] + (warm[c] - neutral[c]) * olive_undertone;
        let per_redness = (olive[c] - unreddened) / OLIVE.1;
        base + per_redness * redness
    })
}

/// The density of `column` at `melanin`: interpolated between the table's
/// depths, and past its ends going on at the end's rate but slowing down,
/// to at most [`PAST_LIGHT_END`] or [`PAST_DEEP_END`] steps further.
fn depth_density(melanin: f32, column: usize) -> [f32; 3] {
    let at = (melanin - FIRST_DEPTH) / DEPTH_STEP;
    let last = (TONES.len() - 1) as f32;
    let row = |depth: usize| entry_density(TONES[depth][column]);
    let lerp = |a: [f32; 3], b: [f32; 3], t: f32| -> [f32; 3] {
        std::array::from_fn(|c| a[c] + (b[c] - a[c]) * t)
    };
    // `most × (1 - e^(-x / most))`: as steep as the end at first, never
    // more than `most` steps.
    let slowed = |x: f32, most: f32| most * (1.0 - (-x / most).exp());
    if at < 0.0 {
        let (end, next) = (row(0), row(1));
        lerp(end, next, -slowed(-at, PAST_LIGHT_END))
    } else if at > last {
        let (end, before) = (row(TONES.len() - 1), row(TONES.len() - 2));
        lerp(end, before, -slowed(at - last, PAST_DEEP_END))
    } else {
        let below = (at.floor() as usize).min(TONES.len() - 2);
        lerp(row(below), row(below + 1), at - below as f32)
    }
}

/// An sRGB `0xrrggbb` as optical density per channel (`-ln` of linear).
fn entry_density(hex: u32) -> [f32; 3] {
    let byte = |shift: u32| ((hex >> shift) & 0xff) as f32 / 255.0;
    [byte(16), byte(8), byte(0)].map(|c| -srgb_to_linear(c).max(1e-4).ln())
}

pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub(crate) fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The depth row of the authored tone.
    const AUTHORED_DEPTH: usize = 2;

    /// The authored tone (references/skin §1.3, tone 10) as linear RGB.
    fn authored() -> [f32; 3] {
        let hex = TONES[AUTHORED_DEPTH][1];
        let byte = |shift: u32| ((hex >> shift) & 0xff) as f32 / 255.0;
        [byte(16), byte(8), byte(0)].map(srgb_to_linear)
    }

    fn toned(melanin: f32, undertone: f32, redness: f32) -> [f32; 3] {
        let ratio = ratio(melanin, undertone, redness);
        std::array::from_fn(|c| authored()[c] * ratio[c])
    }

    fn luminance([r, g, b]: [f32; 3]) -> f32 {
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    #[test]
    fn neutral_values_leave_the_albedo_exactly_as_authored() {
        assert_eq!(ratio(0.0, 0.0, 0.0), [1.0; 3]);
        // A NaN counts as neutral, and values past the range are held.
        assert_eq!(ratio(f32::NAN, 0.0, 0.0), [1.0; 3]);
        assert_eq!(ratio(5.0, 0.0, 0.0), ratio(1.0, 0.0, 0.0));
    }

    #[test]
    fn melanin_darkens_steadily_and_more_in_blue_than_in_red() {
        let mut last = f32::INFINITY;
        for step in -20..=20 {
            let melanin = step as f32 / 20.0;
            let here = luminance(toned(melanin, 0.0, 0.0));
            assert!(
                here < last,
                "melanin {melanin}: {here} isn't darker than {last}"
            );
            last = here;
        }
        let deep = ratio(0.5, 0.0, 0.0);
        assert!(deep[2] < deep[0], "not a plain darkening: {deep:?}");
        let light = ratio(-0.5, 0.0, 0.0);
        assert!(light.iter().all(|c| *c > 1.0), "{light:?}");
    }

    #[test]
    fn the_table_tones_reproduce_the_palette() {
        for (depth, row) in TONES.iter().enumerate() {
            let melanin = FIRST_DEPTH + DEPTH_STEP * depth as f32;
            for (column, undertone, redness) in [
                (0, -UNDERTONE_SPAN, 0.0),
                (1, 0.0, 0.0),
                (2, UNDERTONE_SPAN, 0.0),
                (3, OLIVE.0, OLIVE.1),
            ] {
                let wanted = entry_density(row[column]).map(|d| (-d).exp());
                let got = toned(melanin, undertone, redness);
                for c in 0..3 {
                    assert!(
                        (got[c] - wanted[c]).abs() < 2e-3,
                        "depth {} column {column}: {got:?} isn't {wanted:?}",
                        depth + 1
                    );
                }
            }
        }
    }

    #[test]
    fn redness_reddens_and_undertone_warms() {
        let [r0, g0, b0] = toned(0.0, 0.0, 0.0);
        let [r, g, _] = toned(0.0, 0.0, 0.8);
        assert!(r / g > r0 / g0, "redness up isn't redder");
        let [r, g, _] = toned(0.0, 0.0, -0.8);
        assert!(r / g < r0 / g0, "redness down isn't paler");
        // Warmer: more yellow, red and green over blue.
        let warmth = |[r, g, b]: [f32; 3]| (r + g) / (2.0 * b);
        let neutral = warmth([r0, g0, b0]);
        assert!(warmth(toned(0.0, 0.8, 0.0)) > neutral);
        assert!(warmth(toned(0.0, -0.8, 0.0)) < neutral);
    }

    #[test]
    fn every_tone_stays_inside_real_albedos() {
        // references/skin §1.5: nothing above about 0.8 linear or below
        // about 0.02. The lightest, coolest, palest corner reaches 0.81 in
        // red, and the deepest tones go a little under 0.02, as Monk 10
        // does.
        for m in -10..=10 {
            for u in -2..=2 {
                for r in -2..=2 {
                    let tone = toned(m as f32 / 10.0, u as f32 / 2.0, r as f32 / 2.0);
                    for c in tone {
                        assert!((0.005..=0.82).contains(&c), "{m} {u} {r}: {tone:?}");
                    }
                }
            }
        }
    }
}
