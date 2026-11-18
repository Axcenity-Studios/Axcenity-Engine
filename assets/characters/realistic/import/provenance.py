"""Step 1 of the intake: the provenance record (Paul's decision 11). Plain
Python.

Before anything of a downloaded asset is used, its licence is checked
against `licences.json` (accepted: CC0, public domain, CC BY 3.0 and
4.0, MIT, Apache-2.0; refused: share-alike, non-commercial,
no-derivatives, custom or unknown), each file is hashed as downloaded,
and the record is written in the engine's schema
(`axcenity_avatar::Provenance`, `docs/development/character-studio/
architecture.md`, "Provenance"). The export step adds what was changed
and the item IDs and writes it beside the item as
`<Object>.provenance.json`.

    python -I provenance.py --spec <asset>.intake.json --downloads <folder> --work <folder>
"""

import argparse
import hashlib
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
VERSION = 1
CREDIT = "Axcenity Engine contributors"

with open(os.path.join(HERE, "licences.json"), encoding="utf-8") as _f:
    LICENCES = json.load(_f)


class Refused(Exception):
    """A licence Axcenity doesn't accept, with the reason."""


def terms(licence):
    """`public_domain` or `attribution`, or `Refused` with why (the same
    rule as `axcenity_avatar::licence_terms`)."""
    for kind, ids in LICENCES["accepted"].items():
        if licence in ids:
            return kind
    upper = (licence or "").strip().upper()
    parts = upper.split("-")
    if upper in ("", "NOASSERTION", "NONE"):
        raise Refused("the licence is unknown: record it only after reading it on the source's own page")
    if "NC" in parts:
        raise Refused(f"`{licence}` is non-commercial: a game made with Axcenity may be sold")
    if "ND" in parts:
        raise Refused(f"`{licence}` is no-derivatives: an imported asset is always changed")
    if "SA" in parts or "GPL" in upper:
        raise Refused(f"`{licence}` is share-alike: a game ships its assets under its own terms")
    accepted = [i for ids in LICENCES["accepted"].values() for i in ids]
    raise Refused(f"`{licence}` isn't one Axcenity accepts ({', '.join(accepted)}); "
                  "a custom licence can't be checked")


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def hash_files(folder, names):
    """Each downloaded file's name (as downloaded, `/` between folders),
    size and SHA-256."""
    out = []
    for name in names:
        path = os.path.join(folder, *name.split("/"))
        out.append({"name": name, "sha256": sha256(path), "bytes": os.path.getsize(path)})
    return out


def attribution_line(spec, licence, changes):
    """The line the licence asks for (CC BY's TASL: title, author, source,
    licence, and that it was changed). Also written for a public-domain
    asset, as a courtesy."""
    name = LICENCES["names"].get(licence, licence)
    page = LICENCES["licence_pages"].get(licence)
    licence_text = f"{name} ({page})" if page else name
    changed = "; ".join(changes) if changes else "fitted to the Axcenity realistic body"
    return (f"\"{spec['asset']}\" by {spec['author']} ({spec['source_page']}), licensed under "
            f"{licence_text}; changed: {changed}, by {CREDIT}")


def make_record(spec, files, changes):
    """The record in the engine's schema. `spec` holds the source's facts
    (asset, source_page, author, licence, retrieved, items, evidence and,
    when the source gives its own wording, attribution)."""
    licence = spec["licence"]
    terms(licence)  # raises for a refused licence
    record = {
        "format_version": VERSION,
        "asset": spec["asset"],
        "source_page": spec["source_page"],
        "author": spec["author"],
        "licence": {"id": licence,
                    "url": spec.get("licence_url") or LICENCES["licence_pages"].get(licence, "")},
        "retrieved": spec["retrieved"],
        "files": files,
        "changes": list(changes),
        "attribution": spec.get("attribution") or attribution_line(spec, licence, changes),
        "items": list(spec.get("items", [])),
        "evidence": spec.get("evidence"),
    }
    if not record["evidence"]:
        del record["evidence"]
    return record


def problems(record):
    """What is wrong with a record, as `at: message` (the same checks as
    `Provenance::validate`)."""
    out = []
    url = re.compile(r"^https?://\S+$")
    for key in ("asset", "author"):
        if not str(record.get(key, "")).strip():
            out.append(f"{key}: is empty")
    for at, value in (("source_page", record.get("source_page", "")),
                      ("licence.url", record.get("licence", {}).get("url", ""))):
        if not url.match(value or ""):
            out.append(f"{at}: `{value}` isn't a web address")
    licence = record.get("licence", {}).get("id", "")
    try:
        if terms(licence) == "attribution" and not str(record.get("attribution") or "").strip():
            out.append(f"attribution: `{licence}` asks for credit: the attribution line is required")
    except Refused as why:
        out.append(f"licence.id: {why}")
    if not re.match(r"^\d{4}-\d{2}-\d{2}$", record.get("retrieved", "")):
        out.append(f"retrieved: `{record.get('retrieved')}` isn't a date (YYYY-MM-DD)")
    if not record.get("files"):
        out.append("files: lists no file as downloaded")
    for k, f in enumerate(record.get("files", [])):
        if not re.match(r"^[0-9a-f]{64}$", f.get("sha256", "")):
            out.append(f"files[{k}].sha256: isn't a SHA-256 (64 lower-case hex digits)")
    if not any(str(c).strip() for c in record.get("changes", [])):
        out.append("changes: an imported asset is always changed; say how")
    return out


def finish(work, item_ids, changes):
    """The record as it ships: what was changed (from the steps' reports)
    and the items it feeds; the attribution line is written again with
    the changes unless the source gave its own wording."""
    with open(os.path.join(work, "provenance.json"), encoding="utf-8") as f:
        record = json.load(f)
    record["changes"] = list(changes)
    record["items"] = list(item_ids)
    if not record.get("given_attribution"):
        record["attribution"] = attribution_line(record, record["licence"]["id"], changes)
    record.pop("given_attribution", None)
    found = problems(record)
    if found:
        raise SystemExit("the provenance record isn't sound: " + "; ".join(found))
    return record


def step(spec, downloads, work):
    """Checks the licence and hashes the downloads; writes
    `work/provenance.json` (changes and items are completed at export)."""
    source = dict(spec["source"])
    source["items"] = [spec["item"]["id"]]
    files = hash_files(downloads, source["files"])
    record = make_record(source, files, [])
    if source.get("attribution"):
        record["given_attribution"] = True
    os.makedirs(work, exist_ok=True)
    with open(os.path.join(work, "provenance.json"), "w", newline="\n", encoding="utf-8") as f:
        json.dump(record, f, indent=2)
        f.write("\n")
    return {"licence": source["licence"], "terms": terms(source["licence"]),
            "files": len(files), "bytes": sum(f["bytes"] for f in files)}


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--downloads", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    try:
        print(json.dumps(step(spec, args.downloads, args.work), indent=1))
    except Refused as why:
        raise SystemExit(f"refused: {why}")


if __name__ == "__main__":
    main()
