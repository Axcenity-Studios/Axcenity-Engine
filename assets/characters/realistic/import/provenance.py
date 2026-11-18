"""Step 1 of the intake: the provenance record (Paul's decision 11). Plain
Python.

Before anything of a downloaded asset is used, its licence is checked
against `licences.json` (accepted: CC0, public domain, CC BY 3.0 and
4.0, MIT, Apache-2.0; refused: share-alike, non-commercial,
no-derivatives, custom or unknown), each file is hashed as downloaded,
the licence's notice is kept word for word where the licence asks for it
(MIT, Apache-2.0: `source.notice`, or `source.notice_file`, a
downloaded file read as text), and the record is written in the engine's
schema
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


def needs_notice(licence):
    """Whether the licence asks for its notice in every copy (the same
    list as `axcenity_avatar::needs_notice`)."""
    return licence in LICENCES["notice_required"]


def _bare(url):
    """A web address without its scheme, `www.` or a closing `/`."""
    url = re.sub(r"^https?://", "", (url or "").strip())
    return re.sub(r"^www\.", "", url).rstrip("/")


def is_licence_page(licence, url):
    """Whether `url` is the licence's own text (its page or a page under
    it, such as CC's legal code), the same rule as `Provenance::validate`.
    A licence without a page (a public-domain statement) takes any."""
    page = LICENCES["licence_pages"].get(licence)
    if not page:
        return True
    page, url = _bare(page), _bare(url)
    return url == page or url.startswith(page + "/") or url.startswith(page + ".")


def make_record(spec, files, changes, notice=None):
    """The record in the engine's schema. `spec` holds the source's facts
    (asset, source_page, author, licence, retrieved, items, evidence and,
    when the source gives its own wording, attribution). The licence's
    address is the page of its own text (`licences.json`); the source's
    page about the licence belongs in the evidence. `notice` is the
    licence's notice as given with the asset (MIT, Apache-2.0)."""
    licence = spec["licence"]
    terms(licence)  # raises for a refused licence
    record = {
        "format_version": VERSION,
        "asset": spec["asset"],
        "source_page": spec["source_page"],
        "author": spec["author"],
        "licence": {"id": licence,
                    "url": LICENCES["licence_pages"].get(licence) or spec.get("licence_url", "")},
        "retrieved": spec["retrieved"],
        "files": files,
        "changes": list(changes),
        "attribution": spec.get("attribution") or attribution_line(spec, licence, changes),
        "notice": notice if notice is not None else spec.get("notice"),
        "items": list(spec.get("items", [])),
        "evidence": spec.get("evidence"),
    }
    for optional in ("notice", "evidence"):
        if not record[optional]:
            del record[optional]
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
    licence_url = record.get("licence", {}).get("url", "")
    if url.match(licence_url or "") and not is_licence_page(licence, licence_url):
        out.append(f"licence.url: `{licence_url}` isn't the text of `{licence}` "
                   f"({LICENCES['licence_pages'][licence]})")
    notice = str(record.get("notice") or "")
    if needs_notice(licence):
        if not notice.strip():
            out.append(f"notice: `{licence}` asks for its notice in every copy: "
                       "record it as given with the asset")
        elif licence == "MIT" and "permission is hereby granted" not in notice.lower():
            out.append("notice: MIT's notice is the copyright and permission notice "
                       "(\"Permission is hereby granted...\"), not only a copyright line")
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


def read_notice(downloads, source):
    """The licence's notice as given with the asset: `notice` written in
    the spec, or `notice_file`, a downloaded file (one of `files`) read
    as text. Reading it runs nothing."""
    name = source.get("notice_file")
    if not name:
        return source.get("notice")
    if name not in source["files"]:
        raise SystemExit(f"the notice file `{name}` isn't one of the hashed `files`")
    with open(os.path.join(downloads, *name.split("/")), encoding="utf-8", errors="replace") as f:
        return f.read().strip()


def step(spec, downloads, work):
    """Checks the licence, then hashes the downloads; writes
    `work/provenance.json` (changes and items are completed at export).
    A refused licence stops the chain before any download is read."""
    source = dict(spec["source"])
    source["items"] = [spec["item"]["id"]]
    terms(source["licence"])  # raises for a refused licence, before any file is read
    if source.get("model") and source["model"] not in source["files"]:
        raise SystemExit(f"the model `{source['model']}` isn't one of the hashed `files`")
    files = hash_files(downloads, source["files"])
    record = make_record(source, files, [], notice=read_notice(downloads, source))
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
