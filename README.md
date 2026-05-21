# iris-rust

Iris recognition in Rust. A port of the Daugman branch of [Qingbao/iris](https://github.com/Qingbao/iris) — same algorithm, same EER ballpark on CASIA v1.0, but a single static binary and an end-to-end run in seconds instead of hours.

See [`IMPLEMENTATION.html`](IMPLEMENTATION.html) for a one-page summary of the pipeline and results.

## Setup

### Prerequisites

- Rust 1.75 or newer (`rustup` recommended — <https://rustup.rs>).
- ~1 GB free disk for the CASIA dataset.
- About 4 minutes for the first release build (compiles all deps).

### Get the source

```sh
git clone https://github.com/Qingbao/iris-rust
cd iris-rust
```

### Get the CASIA Iris Image Database v1.0

The dataset is **not** included in this repository (the `samples/` directory is gitignored).

1. Register and download from <http://biometrics.idealtest.org/dbDetailForUser.do?id=1>.
2. Unpack so the resulting layout looks like:

    ```text
    iris-rust/
    └── samples/
        └── CASIA-IrisV1/
            └── CASIA Iris Image Database (version 1.0)/
                ├── 001/
                │   ├── 1/001_1_1.bmp, 001_1_2.bmp, 001_1_3.bmp
                │   └── 2/001_2_1.bmp, 001_2_2.bmp, 001_2_3.bmp, 001_2_4.bmp
                ├── 002/...
                └── 108/...
    ```

    Subject IDs are 3-digit zero-padded (`001`–`108`). Each subject has 7 BMPs: 3 from session 1 and 4 from session 2. 756 images total.

You can place the dataset anywhere — the `--casia <DIR>` flag below takes the path to the inner `CASIA Iris Image Database (version 1.0)/` folder.

### Build

```sh
cargo build --release
```

This produces `./target/release/iris` (or `iris.exe` on Windows).

## Usage

The CLI has three subcommands: `enroll`, `match`, `eer`.

### 1. Enroll — segment every eye image and save its 9600-bit template

```sh
./target/release/iris enroll \
    --casia "samples/CASIA-IrisV1/CASIA Iris Image Database (version 1.0)" \
    --out templates.bin
```

Optional flags:

| Flag | Effect |
|---|---|
| `--diagnostics <DIR>` | Write three JPGs per image: segmented overlay, polar unwrap, noise mask. Useful for sanity-checking the iris/pupil detection. |
| `--limit N` | Process only the first N images (subjects iterated in order). Handy for smoke tests. |

Example smoke test on subject 001 only:

```sh
./target/release/iris enroll \
    --casia "samples/CASIA-IrisV1/CASIA Iris Image Database (version 1.0)" \
    --out smoke.bin --limit 7 --diagnostics smoke_diag
```

After the full enroll you should see roughly `enrolled 756 templates -> templates.bin`. Wall time on a modern desktop: ~15 s.

### 2. Match — compute Hamming distance for every relevant image pair

Intra-class (same subject):

```sh
./target/release/iris match --templates templates.bin --intra --out hd_same.csv
```

Inter-class (different subjects):

```sh
./target/release/iris match --templates templates.bin --inter --out hd_diff.csv
```

Both write a CSV: `i,j,subject_i,subject_j,hd`. Counts: 2 268 intra-class pairs, 283 122 inter-class pairs.

### 3. EER — find the equal-error rate

```sh
./target/release/iris eer --same hd_same.csv --diff hd_diff.csv --curve eer_curve.csv
```

Output:

```text
loaded 2268 same, 283122 diff
EER = 0.0172
```

`--curve` is optional and dumps `threshold,fmr,fnmr` rows for plotting in whatever you prefer (Python/matplotlib, Excel, gnuplot…).

## Expected results on CASIA v1.0

| Metric | Value |
|---|---|
| EER | **0.0172** (MATLAB ref: 0.0157) |
| Intra-class HD (mean ± σ) | 0.298 ± 0.053 |
| Inter-class HD (mean ± σ) | 0.472 ± 0.014 |
| Enroll 756 images | ~13 s |
| Match 283 122 inter-class pairs | ~90 s |

If your EER lands outside `[0.012, 0.022]`, something has likely gone wrong upstream — open a few JPGs from `--diagnostics` and check that the circles fit the iris/pupil.

## Project layout

```text
src/
├── lib.rs            module declarations
├── types.rs          Circle, Template, Mask, EnrolledTemplate
├── casia.rs          dataset walker + BMP loader
├── daugman.rs        integro-differential boundary search
├── eyelid.rs         Canny + Radon line detection
├── segment.rs        full segmentation orchestrator
├── normalize.rs      rubber-sheet polar unwrap
├── encode.rs         1-D log-Gabor + phase quantization
├── matching.rs       shifted Hamming distance
├── diagnostics.rs    overlay / polar / noise JPG writers
└── bin/iris.rs       CLI (enroll / match / eer)
```

## Troubleshooting

**`missing CASIA image: …001/1/001_1_1.bmp`**
The folder layout doesn't match the expected `{subject:03}/{session}/{subject:03}_{session}_{k}.bmp`. Check that you pointed `--casia` at the inner `CASIA Iris Image Database (version 1.0)/` directory, not its parent.

**`Pre-existing main.exe / main.pdb` warnings**
Old build artifacts from a previous skeleton. Safe to delete — `cargo build` will not regenerate them.

**Diagnostic JPGs look wrong (circles way off)**
Open the corresponding `-segmented.jpg`. If only one of the two circles is wrong, the inner-/outer-boundary search parameters in `daugman.rs` (`lpupilradius`, `upupilradius`, …) may need tuning for a non-CASIA dataset. The current values are tuned for CASIA v1.0's 320×280 NIR images.

**Build takes forever the first time**
Expected — `rustfft`, `imageproc`, and `ndarray` pull in a fair amount of code. Subsequent builds are incremental.

## License

MIT — see [`LICENSE`](LICENSE).

The original Daugman MATLAB algorithm sources at <https://github.com/Qingbao/iris> are based on Libor Masek's open-source iris pipeline with light modifications; check that repository's license before redistributing derivative work.
