# Licence record: `body_shapes.npz`

`body_shapes.npz` is derived data: 16 body shapes made by `../transfer.py` from two CC0 sources. Nothing else from either source is in this repository. It is a spike artefact (on the bundle's pose, not the engine's rest pose): CO5 regenerates it.

The same record covers the file the realistic package vendors, `../../realistic/source/body_shapes.npz`: the same 16 bodies made the same way (`--method filtered`), and since R3 (2026-11-17) 8 regional targets besides (below).

## Sources

| Source | What was taken | Where from | Licence |
|---|---|---|---|
| MakeHuman | The base mesh and 30 targets (list below), read as data | `https://github.com/makehumancommunity/makehuman`, branch `master`, commit `a8bc2d54ff0ac92e78ff71431b1023eda42bf482` (2024-06-26), fetched 2026-11-14 by `../fetch_makehuman.py` | CC0 1.0 Universal |
| Blender Studio Human Base Meshes v1.4.1 | The realistic male and female bodies (`GEO-body_male_realistic`, `GEO-body_female_realistic`): vertex positions, faces, sculpt face sets | `https://download.blender.org/demo/asset-bundles/human-base-meshes/human-base-meshes-bundle-v1.4.1.zip`, SHA-256 `811f43accbb31a88266d932f8f5563b2d13586fca0ba2693aad1f5fe582b3515` (as recorded in `../../humanoid/source/extract.py`; the spike read the unpacked `.blend` and did not re-hash the archive) | CC0 1.0 Universal (the bundle the stylized humanoid is already vendored from, `../../humanoid/source/`) |

## What makes the MakeHuman files CC0

Checked in the files as downloaded, not on the website:

- `LICENSE.md`, section C, lines 52 to 61: "The assets are defined as any data contributing to the graphical output of MakeHuman. This includes: * The base mesh and proxies * Targets and modifiers * Textures * Clothes (any MHCLO-based asset) * Poses and expressions" and "These assets have been released under CC0 1.0 Universal."
- `LICENSE.ASSETS.md`: the full text of CC0 1.0 Universal.
- Every one of the 31 data files used (`base.obj` and each `.target`) says so itself, lines 3 and 4: "This asset was explicitly released as CC0 in september 2020. The license text for CC0 can be found in the root of this repository." (`grep` confirmed all 31.)
- `LICENSE.md`, section B, lines 23 to 29: the source code ("files that contain program logic. This includes python files, bat scripts, shell scripts and glsl shaders") is AGPL. **No code was fetched, read or adapted.** The clone is partial and sparse (licence files, `base.obj`, `data/targets/`); the two file formats (OBJ; `.target`: `index dx dy dz` per line after `#` comments) were read from the files themselves. The macro sliders' mixing rule (a slider splits into min, average and max factors; a corner target's weight is the product of its factors) was written from the Stage 1 research note's statement of it; it is an interpolation rule, not code.

CC0 asks for no attribution. Crediting MakeHuman where the repository credits its other sources would still be right when shapes ship.

Not used: MPFB2's asset pack and Anny's `data/mpfb2` (the same targets, repackaged); MakeHuman's rig and weight files (`data/rigs/`: data, but not named in the licence's list of assets, so the spike takes joint positions only from the joint cubes that are part of `base.obj`); any third-party MakeHuman asset.

## Files read

SHA-256 of each is in `../makehuman.sha256`; `python3 ../fetch_makehuman.py DIR --check` verifies a checkout against it (33 of 33 at the time of writing). Paths are under `makehuman/data/` except the two licence files.

- `LICENSE.md`, `LICENSE.ASSETS.md`
- `3dobjs/base.obj` (the base mesh `hm08`: 19,158 vertices, of which the body is the first 13,380; the joint cubes give the joint centres)
- `targets/macrodetails/{african,asian,caucasian}-{male,female}-young.target` (6: each gender's neutral is the base plus one third of each)
- `targets/macrodetails/universal-{male,female}-young-{min,average,max}muscle-{min,average,max}weight.target` (18, of which the two `averagemuscle-averageweight` ones are empty)
- `targets/stomach/stomach-pregnant-incr.target`, `targets/stomach/stomach-tone-incr.target`, `targets/buttocks/buttocks-volume-incr.target`, `targets/torso/torso-muscle-pectoral-incr.target`, `targets/breast/female-young-averagemuscle-averageweight-{max,min}cup-averagefirmness.target` (6 regional targets, in the sheets but not in `body_shapes.npz`)

## Added in R3 (2026-11-17): the fat the `fat` slider adds

Eight more MakeHuman targets, read from the same checkout at the same commit (`a8bc2d54ff0ac92e78ff71431b1023eda42bf482`), carried by `../transfer.py --method filtered` and vendored in `../../realistic/source/body_shapes.npz` by `--pack-extras` (`realistic/shapes.py`'s `SOFT` and `FLAT` say how they are mixed). Each file's own header says, lines 3 and 4: "This asset was explicitly released as CC0 in september 2020" (the `stomach`, `neck` and `armslegs` targets) or "in October 2021" (the four `bodyshapes`), "The license text for CC0 can be found in the root of this repository" (checked in each file as downloaded, 2026-11-17). `LICENSE.md` section C covers "Targets and modifiers" as above. Data only; no code was read.

| Target (under `makehuman/data/targets/`) | Our name | SHA-256 of the file as checked out | Used on |
|---|---|---|---|
| `stomach/stomach-pregnant-decr.target` | `pregnant_decr` | `68d67754b5c38e4922531e3fc8e146faf8853c949c84d1f18824724f9d16ec40` | masculine (`Masculine_Belly_Flat`) |
| `bodyshapes/bodyshapes-elvs-man-thyroid.target` | `m_thyroid` | `fc60100fe3fe0d4582fb725bdd2f22e5ac5f880631ba9936e0120637219a9583` | masculine (`Fat_Soft`) |
| `bodyshapes/bodyshapes-elvs-man-adrenal.target` | `m_adrenal` | `55d72dce46b7cced6b06a05097abe66703ffb6545fbb520b0dbc98392e16eb5b` | masculine (`Fat_Soft`) |
| `bodyshapes/bodyshapes-elvs-fem-ovary.target` | `f_ovary` | `19a71f83c8eba348d81aeb987ae575ec3de8b50d1ad5cc544d3013e62cd071ab` | feminine (`Fat_Soft`) |
| `bodyshapes/bodyshapes-elvs-fem-liver.target` | `f_liver` | `987a295615dd8762798766d482f1b0f55d6035bc383311b12d8585efcbdb6cd1` | feminine (`Fat_Soft`) |
| `neck/neck-double-incr.target` | `neck_double` | `faf7229632f2b531d0d5f96990545aa120bed26bb3949625e7c8eda79e99eda9` | both (`Fat_Soft`) |
| `armslegs/l-upperarm-fat-incr.target` and `armslegs/r-upperarm-fat-incr.target` (summed: one control's two sides) | `upperarm_fat` | `0502128d4661484bac746466b2135e035fd69ecddf737573ef62b0a95ebbf036`, `c5d1588a8ccfc4b2de56588d1108ff815ebf571c2a4467e15691fdb4bfbe7be2` | feminine (`Fat_Soft`) |

The hashes are in `../makehuman.sha256` (41 files now; `python3 ../fetch_makehuman.py DIR --check`: 41 of 41 on 2026-11-17). They are of the files as checked out on Windows with Git's `core.autocrlf` on, as the spike's 33 were (CRLF line ends; the blobs in MakeHuman's repository have LF): a checkout with LF line ends will not match any of the 41. The data read from either is the same.

Read while choosing and **not** vendored (all CC0 by the same header, at the same commit): the other `bodyshapes-elvs-*` targets (man apple, liver, gonadal; fem apple, triangle, full hourglass, adrenal), `measure-{waist,hips,thigh,upperarm,neck}-circ-incr`, `torso-scale-depth-incr`, `upperleg-fat-incr`, `head-fat-incr`, `pelvis-tone-{incr,decr}`, `stomach-navel-in`, `hip-scale-depth-decr`.

## The file's format

A NumPy `.npz` (zip, deflate), 0.43 MB (the realistic package's, with R3's regional targets, 0.55 MB). `step` is 0.00002 (metres per unit). Per shape `<frame>_<muscle>muscle_<weight>weight` (frame `masculine` or `feminine`; the 16 gender x muscle x weight corners at the young age, without the two averages): `<shape>_index` (uint16, the vertices it moves by more than 0.05 mm) and `<shape>_offset` (int16, n x 3, offsets in units of `step`). Vertex indices are the bundle's own order (10,582 vertices); axes are Blender's (Z up, the figure faces -Y); offsets are from that frame's body in the bundle's pose with the base vertices on the sculpt, as CO4 vendors it. `transfer.unpack` reads it. The regional targets are stored the same way as `<frame>_<our name>` (`masculine_m_thyroid`); `realistic/shapes.unpack` reads both kinds.
