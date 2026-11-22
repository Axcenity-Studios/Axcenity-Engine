"""Builds the installer and disk-image pictures from the engine branding.

Run from the repository root: `python packaging/generate-assets.py`
Needs Pillow (3.11+ for `tomllib`). Uses the editor's fonts from
`assets/fonts` and the mark/wordmark PNGs from `assets/branding`.

The colours are the editor's dark theme and its default blue accent
(`crates/editor-kit/src/theme.rs`), so setup looks like the program it
installs. Everything is drawn at 4x and scaled down, for smooth edges on
the small Windows bitmaps.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT / "assets" / "branding"
FONTS = ROOT / "assets" / "fonts"
OUT = Path(__file__).resolve().parent / "assets"

# The editor's dark palette (Palette::new(dark = true, Accent::Blue)).
APP_BG = (0x17, 0x17, 0x17)
PANEL = (0x1E, 0x1E, 0x1E)
SURFACE = (0x24, 0x24, 0x24)
BORDER = (0x30, 0x30, 0x30)
BORDER_STRONG = (0x42, 0x42, 0x42)
TEXT_STRONG = (0xFA, 0xFA, 0xFA)
TEXT_SECONDARY = (0xA1, 0xA1, 0xAA)
TEXT_MUTED = (0x63, 0x63, 0x6B)
ACCENT = (0x6C, 0x8C, 0xFF)
ACCENT_SOLID = (0x45, 0x62, 0xE6)
# The light theme, for the parts of the disk image Finder labels sit on
# (Finder always draws them in black over a background picture).
LIGHT_BG = (0xF4, 0xF4, 0xF5)
LIGHT_BORDER = (0xE4, 0xE4, 0xE7)
LIGHT_TEXT = (0x27, 0x27, 0x2A)
LIGHT_TEXT_WEAK = (0x71, 0x71, 0x7A)
WHITE = (255, 255, 255)

SS = 4  # supersampling factor


def channel() -> str:
    """The release channel from the workspace version: "Alpha", "Beta" or ""."""
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    version = cargo["workspace"]["package"]["version"]
    for name in ("alpha", "beta", "rc"):
        if f"-{name}" in version:
            return "Preview" if name == "rc" else name.capitalize()
    return ""


def font(name: str, size: float) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(str(FONTS / name), round(size * SS))


def title_font(size: float) -> ImageFont.FreeTypeFont:
    return font("PlusJakartaSans-Bold.ttf", size)


def load(name: str) -> Image.Image:
    return Image.open(BRAND / name).convert("RGBA")


def scaled(im: Image.Image, width: float) -> Image.Image:
    w = round(width * SS)
    return im.resize((w, round(im.height * w / im.width)), Image.LANCZOS)


def canvas(w: int, h: int, colour: tuple[int, int, int]) -> Image.Image:
    return Image.new("RGBA", (w * SS, h * SS), colour + (255,))


def finish(im: Image.Image) -> Image.Image:
    return im.resize((im.width // SS, im.height // SS), Image.LANCZOS)


def s(*values: float) -> tuple[int, ...]:
    return tuple(round(v * SS) for v in values)


def text_width(text: str, fnt: ImageFont.FreeTypeFont) -> int:
    x0, _, x1, _ = fnt.getbbox(text)
    return x1 - x0


def draw_text(
    im: Image.Image,
    text: str,
    fnt: ImageFont.FreeTypeFont,
    xy: tuple[float, float],
    fill: tuple[int, int, int],
    anchor: str = "la",
    tracking: float = 0.0,
) -> None:
    """Text at `xy` (in 1x points), optionally letter-spaced."""
    draw = ImageDraw.Draw(im)
    x, y = xy[0] * SS, xy[1] * SS
    if not tracking:
        draw.text((x, y), text, font=fnt, fill=fill, anchor=anchor)
        return
    gap = tracking * SS
    total = sum(fnt.getlength(c) for c in text) + gap * (len(text) - 1)
    if anchor[0] == "m":
        x -= total / 2
    elif anchor[0] == "r":
        x -= total
    for c in text:
        draw.text((x, y), c, font=fnt, fill=fill, anchor="l" + anchor[1])
        x += fnt.getlength(c) + gap


def glow(im: Image.Image, centre: tuple[float, float], radius: float, colour, alpha: int) -> None:
    """A soft round light, like the accent glow behind the editor's start screen."""
    layer = Image.new("RGBA", im.size, colour + (0,))
    draw = ImageDraw.Draw(layer)
    cx, cy = centre[0] * SS, centre[1] * SS
    r = radius * SS
    draw.ellipse((cx - r, cy - r, cx + r, cy + r), fill=colour + (alpha,))
    layer = layer.filter(ImageFilter.GaussianBlur(r * 0.55))
    im.alpha_composite(layer)


def vertical_gradient(im: Image.Image, box, top, bottom) -> None:
    draw = ImageDraw.Draw(im)
    x0, y0, x1, y1 = (round(v * SS) for v in box)
    for y in range(y0, y1):
        t = (y - y0) / max(1, y1 - y0 - 1)
        c = tuple(round(a + (b - a) * t) for a, b in zip(top, bottom))
        draw.line((x0, y, x1 - 1, y), fill=c + (255,))


def grid_floor(im: Image.Image, box, horizon: float, colour, alpha: int) -> None:
    """A perspective grid like the viewport's floor, fading toward the horizon."""
    x0, y0, x1, y1 = box
    layer = Image.new("RGBA", im.size, colour + (0,))
    draw = ImageDraw.Draw(layer)
    cx = (x0 + x1) / 2
    width = x1 - x0
    # Lines toward the vanishing point.
    for i in range(-12, 13):
        bx = cx + i * width / 7
        draw.line(s(cx + (bx - cx) * 0.02, horizon, bx, y1), fill=colour + (alpha,), width=SS)
    # Cross lines, closer together near the horizon.
    depth = y1 - horizon
    for i in range(1, 12):
        t = (i / 11) ** 2.2
        y = horizon + depth * t
        draw.line(s(x0, y, x1, y), fill=colour + (alpha,), width=SS)
    # Fade toward the horizon.
    mask = Image.new("L", im.size, 0)
    mdraw = ImageDraw.Draw(mask)
    top, bottom = round(horizon * SS), round(y1 * SS)
    for y in range(top, bottom):
        t = (y - top) / max(1, bottom - top)
        mdraw.line((round(x0 * SS), y, round(x1 * SS), y), fill=round(255 * min(1.0, t * 1.6)))
    faded = Image.new("RGBA", im.size, colour + (0,))
    faded.paste(layer, (0, 0), mask)
    im.alpha_composite(faded)


def chip(im: Image.Image, text: str, centre_x: float, y: float) -> None:
    """A small outlined label in the accent colour, like the editor's badges."""
    fnt = font("JetBrainsMono-Bold.ttf", 7.5)
    tracking = 1.1
    w = sum(fnt.getlength(c) for c in text) / SS + tracking * (len(text) - 1) + 16
    h = 15
    x = centre_x - w / 2
    draw = ImageDraw.Draw(im)
    draw.rounded_rectangle(
        s(x, y, x + w, y + h),
        radius=4 * SS,
        fill=tuple(round(a * 0.82 + b * 0.18) for a, b in zip(PANEL, ACCENT)) + (255,),
        outline=ACCENT + (255,),
        width=SS,
    )
    draw_text(im, text, fnt, (centre_x, y + h / 2 + 0.3), mix(ACCENT, WHITE, 0.45), "mm", tracking)


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def header() -> Image.Image:
    """NSIS page header, 150x57, on the right of the white header bar:
    the black wordmark, as in the light theme."""
    w, h = 150, 57
    im = canvas(w, h, WHITE)
    mark = scaled(load("wordmark.png"), 116)
    im.alpha_composite(mark, (round((w - 12) * SS) - mark.width, (h * SS - mark.height) // 2))
    return finish(im).convert("RGB")


def sidebar() -> Image.Image:
    """NSIS Welcome and Finish bitmap, 164x314: the editor's start screen in
    miniature (dark panel, accent glow, the mark, the viewport's grid)."""
    w, h = 164, 314
    im = canvas(w, h, APP_BG)
    vertical_gradient(im, (0, 0, w, h), PANEL, (0x12, 0x12, 0x12))
    glow(im, (w / 2, 92), 70, ACCENT_SOLID, 70)
    grid_floor(im, (-40, 0, w + 40, h), 232, BORDER_STRONG, 255)
    glow(im, (w / 2, 300), 60, ACCENT_SOLID, 28)

    mark = scaled(load("mark-white.png"), 70)
    im.alpha_composite(mark, (round((w * SS - mark.width) / 2), round(48 * SS)))

    draw_text(im, "Axcenity", title_font(21), (w / 2, 140), TEXT_STRONG, "ma")
    draw_text(im, "Editor", font("Inter-Medium.ttf", 12), (w / 2, 168), TEXT_SECONDARY, "ma")
    label = channel()
    if label:
        chip(im, label.upper(), w / 2, 192)

    # A hairline in the accent along the right edge, where the page begins.
    draw = ImageDraw.Draw(im)
    draw.rectangle(s(w - 1, 0, w, h), fill=BORDER + (255,))
    return finish(im).convert("RGB")


def dmg_background(scale: int = 1) -> Image.Image:
    """The disk-image window, 720x438 points: a dark band with the wordmark
    over a light stage where the app and Applications sit (Finder labels
    them in black). Icon centres match `packaging/macos/dmgbuild_settings.py`."""
    global SS
    saved, SS = SS, SS * scale
    try:
        w, h = 720, 438
        band = 150
        im = canvas(w, h, LIGHT_BG)

        # The band: the editor's title bar and start screen.
        top = canvas(w, band, APP_BG)
        vertical_gradient(top, (0, 0, w, band), PANEL, APP_BG)
        glow(top, (150, 40), 140, ACCENT_SOLID, 60)
        grid_floor(top, (0, 0, w, band), 70, BORDER, 255)
        im.alpha_composite(top)
        draw = ImageDraw.Draw(im)
        draw.rectangle(s(0, band - 2, w, band), fill=ACCENT_SOLID + (255,))

        wordmark = scaled(load("wordmark-white.png"), 190)
        im.alpha_composite(wordmark, (round(44 * SS), round((band - 2) * SS / 2 - wordmark.height / 2)))
        draw_text(im, "Drag Axcenity Editor into Applications", font("Inter-Medium.ttf", 13.5),
                  (w - 44, band / 2 - 10), TEXT_STRONG, "rm")
        draw_text(im, "to install it.", font("Inter-Regular.ttf", 12.5),
                  (w - 44, band / 2 + 9), TEXT_SECONDARY, "rm")
        label = channel()
        if label:
            chip_x = 44 + 190 + 14
            fnt = font("JetBrainsMono-Bold.ttf", 7.5)
            cw = sum(fnt.getlength(c) for c in label.upper()) / SS + 1.1 * (len(label) - 1) + 16
            chip(im, label.upper(), chip_x + cw / 2, band / 2 - 8)

        # The stage: two soft slots and an arrow between them.
        icon_y = 270
        for cx in (200, 520):
            draw.rounded_rectangle(s(cx - 92, icon_y - 86, cx + 92, icon_y + 100), radius=14 * SS,
                                   fill=WHITE + (255,), outline=LIGHT_BORDER + (255,), width=SS)
        ax0, ax1 = 318, 402
        draw.line(s(ax0, icon_y, ax1 - 10, icon_y), fill=ACCENT_SOLID + (255,), width=3 * SS)
        draw.polygon([s(ax1, icon_y), s(ax1 - 14, icon_y - 9), s(ax1 - 14, icon_y + 9)],
                     fill=ACCENT_SOLID + (255,))
        for i, dx in enumerate((-18, -30, -42)):
            r = 2.2 - i * 0.5
            draw.ellipse(s(ax0 + dx - r, icon_y - r, ax0 + dx + r, icon_y + r),
                         fill=mix(LIGHT_BG, ACCENT_SOLID, 0.7 - i * 0.2) + (255,))

        draw_text(im, "Open Axcenity Editor from Launchpad or the Applications folder.",
                  font("Inter-Regular.ttf", 11.5), (w / 2, 398), LIGHT_TEXT_WEAK, "mm")
        return im.resize((w * scale, h * scale), Image.LANCZOS).convert("RGB")
    finally:
        SS = saved


def icon_tile() -> Image.Image:
    """Square icon tile: the app icon, a white A on a rounded charcoal tile."""
    return load("icon.png").resize((256, 256), Image.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    header_im = header()
    header_im.save(OUT / "nsis-header.bmp")
    header_im.save(OUT / "nsis-header.png", optimize=True)
    sidebar_im = sidebar()
    sidebar_im.save(OUT / "nsis-sidebar.bmp")
    sidebar_im.save(OUT / "nsis-sidebar.png", optimize=True)
    dmg_background().save(OUT / "dmg-background.png", optimize=True)
    dmg_background(2).save(OUT / "dmg-background@2x.png", optimize=True)
    icon_tile().save(OUT / "icon-tile.png", optimize=True)
    print(f"wrote installer pictures in {OUT}")


if __name__ == "__main__":
    main()
