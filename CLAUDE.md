# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust port of the Daugman branch of [Qingbao/iris](https://github.com/Qingbao/iris) — iris recognition on CASIA Iris Image Database v1.0. Targets the same EER (~0.017) as the MATLAB reference but runs the full 756-image pipeline in seconds instead of hours.

The CASIA dataset is **not** in the repo (gitignored under `samples/`). Most work can be done without it, but `enroll` / integration tests need it. See [README.md](README.md) for the expected directory layout and download link.

## Commands

```sh
cargo build --release          # produces target/release/iris(.exe); ~4 min cold, fast incrementally
cargo build                    # debug; iris is ~10x slower in debug, fine for compile-checking
cargo check                    # fastest feedback loop while editing
cargo test                     # no integration tests currently; unit tests live alongside modules
cargo test <name>              # run a single test by substring match
cargo clippy --all-targets
```

The release profile uses `lto = "thin"` and `codegen-units = 1` — full release builds are slow, so prefer `cargo check` / `cargo build` (debug) during development and only do `--release` when running the actual pipeline.

End-to-end run (requires CASIA samples):

```sh
./target/release/iris enroll --casia "<path-to-CASIA v1.0 root>" --out templates.bin
./target/release/iris match --templates templates.bin --intra --out hd_same.csv
./target/release/iris match --templates templates.bin --inter --out hd_diff.csv
./target/release/iris eer --same hd_same.csv --diff hd_diff.csv
```

Smoke test without the full dataset is impossible — but `--limit N` on `enroll` processes only the first N images, and `--diagnostics <DIR>` dumps per-image overlay/polar/noise JPGs which are the primary debugging surface when segmentation looks wrong.

Expected EER on CASIA v1.0 is `0.0172` (MATLAB ref: `0.0157`). EER outside `[0.012, 0.022]` means something upstream broke — inspect the diagnostics JPGs first.

## Architecture

The pipeline is a strict five-stage feed-forward sequence, each stage in its own module. Data flows: BMP → grayscale `Array2<f64>` → `Segmentation` → polar `Array2<Option<f64>>` → `(Template, Mask)` → Hamming distance.

```
casia::load_grayscale  ──►  segment::segment_iris  ──►  normalize::normalize  ──►  encode::encode  ──►  matching::hamming_distance
                              │                          │                         │
                              ├─ daugman (boundaries)    └─ rubber-sheet polar     └─ 1-D log-Gabor + phase quantize → 9600 bits
                              └─ eyelid (Radon line)        (RADIAL_RES=20 ×
                                                             ANGULAR_RES=240)
```

Key cross-module contracts (in [src/types.rs](src/types.rs)):

- `RADIAL_RES = 20`, `ANGULAR_RES = 240`, `TEMPLATE_BITS = 9600`. These are constants shared by `normalize`, `encode`, and `matching` — changing one without the others silently corrupts templates.
- `Array2<Option<f64>>` is the "image with noise mask" type. `None` means a masked-out pixel (eyelid, eyelash, out-of-iris). It flows from `segment` → `normalize` → `encode`, where `None` regions become set bits in the `Mask` bitvec.
- `Template` and `Mask` are both `BitVec<u64, Lsb0>` of length `TEMPLATE_BITS`. Matching XORs templates and ORs masks, then divides by the count of unmasked bits.
- `EnrolledTemplate` (id + circles + template + mask) is the bincode-serialized payload of `templates.bin`.

Segmentation specifics:

- `daugman::search_inner_boundary` / `search_outer_boundary` are the integro-differential operator from Daugman's paper. Pupil radius bounds (`lpupilradius`/`upupilradius` and the iris equivalents) in [daugman.rs](src/daugman.rs) are **tuned for CASIA's 320×280 NIR images** — they will likely need adjusting for any other dataset.
- `eyelid::find_line` runs Canny + Radon transform on strips above/below the pupil to find a straight eyelid line. Returned line is fed to `segment` which sets `None` in the mask above/below it.
- `EYELASH_THRESHOLD = 80.0` in [segment.rs](src/segment.rs) is also CASIA-specific (NIR eyelash intensity).

Matching specifics ([src/matching.rs](src/matching.rs)):

- `MAX_SHIFT = 8` circular shifts of the template in the angular direction compensate for head tilt. Each shift moves by `2 * SCALES * |shift|` columns (where `SCALES = 1` and `COLS = ANGULAR_RES * 2`).
- HD is `popcount((t1 ^ t2) & !(m1 | m2)) / popcount(!(m1 | m2))`, minimized over shifts.

## Parallelism

`enroll` and `match` use rayon (`par_iter`) at the per-image / per-pair level. Templates are independent so this scales linearly; matching does ~283k inter-class pairs and is the main thing release builds buy you. Don't introduce shared mutable state into the per-image closure in [src/bin/iris.rs](src/bin/iris.rs).

`ndarray` is built with the `rayon` feature — array-level parallel ops are available but the current code keeps parallelism at the outer level only.

## When segmentation goes wrong

The single most useful debugging tool is `enroll --diagnostics <DIR>` on a small `--limit`. Three JPGs per image:
- `*-segmented.jpg` — iris/pupil circle overlay. If circles are way off, the issue is in `daugman.rs`.
- `*-polar.jpg` — the unwrapped iris. Should look like a horizontal strip of texture.
- `*-noise.jpg` — the mask. Black = masked. Excessive black usually means eyelid detection over-fired.

Don't try to fix segmentation by inspecting code alone — open the JPGs first to localize which stage is producing bad output.
