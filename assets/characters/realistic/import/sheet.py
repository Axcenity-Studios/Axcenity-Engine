"""The intake's review sheet from the engine's captures (Pillow): a row per
cast body and pose, the front, side and rear views whole and the item's
area close up, labelled. Plain Python with Pillow:

    python -I sheet.py --captures <pose folder>[,<pose folder>...] --out sheet.jpg
        [--crop 0.38,0.0,0.62,0.26] [--width 420]

Each pose folder is an `AXCENITY_CAPTURE_DIR` of
`cargo test -p axcenity-player --release -- --ignored capture_characters`
(one sub-folder per description, `studio-front.png`, `-side`, `-back`);
`--crop` is the close-up's box as shares of the picture (left, top,
right, bottom: by default the head and shoulders).
"""

import argparse
import os

from PIL import Image, ImageDraw

VIEWS = ("front", "side", "back")


def row(folder, crop, width):
    whole, close = [], []
    for view in VIEWS:
        path = os.path.join(folder, f"studio-{view}.png")
        im = Image.open(path).convert("RGB")
        w, h = im.size
        # The body stands in the middle third of a capture.
        body = im.crop((int(w * 0.3), 0, int(w * 0.7), h))
        whole.append(body.resize((width, int(body.height * width / body.width))))
        box = (int(w * crop[0]), int(h * crop[1]), int(w * crop[2]), int(h * crop[3]))
        part = im.crop(box)
        close.append(part.resize((width, int(part.height * width / part.width))))
    return whole, close


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--captures", required=True, help="pose folders, `,` between")
    parser.add_argument("--out", required=True)
    parser.add_argument("--crop", default="0.43,0.02,0.57,0.30")
    parser.add_argument("--width", type=int, default=420)
    args = parser.parse_args()
    crop = [float(x) for x in args.crop.split(",")]
    rows = []
    for folder in args.captures.split(","):
        pose = os.path.basename(folder.rstrip("/\\"))
        for name in sorted(os.listdir(folder)):
            if os.path.isdir(os.path.join(folder, name)):
                rows.append((f"{name}, {pose}", *row(os.path.join(folder, name), crop, args.width)))
    label_h = 28
    cell_w = args.width
    whole_h = rows[0][1][0].height
    close_h = rows[0][2][0].height
    sheet = Image.new("RGB", (cell_w * 6, len(rows) * (label_h + max(whole_h, close_h))), (40, 40, 44))
    draw = ImageDraw.Draw(sheet)
    y = 0
    for label, whole, close in rows:
        draw.text((8, y + 8), f"{label}: front, side, rear | close up: front, side, rear", fill=(235, 235, 235))
        for k, im in enumerate(whole + close):
            sheet.paste(im, (k * cell_w, y + label_h))
        y += label_h + max(whole_h, close_h)
    sheet.save(args.out, quality=88)
    print(args.out, sheet.size, os.path.getsize(args.out))


if __name__ == "__main__":
    main()
