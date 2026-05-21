use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ParallelProgressIterator, ProgressStyle};
use iris_rust::{
    casia,
    diagnostics,
    encode,
    matching,
    normalize,
    segment,
    types::EnrolledTemplate,
};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "iris")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Walk the CASIA folder, segment + encode every image, persist a binary template bag.
    Enroll {
        #[arg(long)]
        casia: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        diagnostics: Option<PathBuf>,
        /// Process only the first N images (for smoke-testing).
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Compute Hamming-distance pair sweep (intra- or inter-class).
    Match {
        #[arg(long)]
        templates: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, conflicts_with = "inter")]
        intra: bool,
        #[arg(long, conflicts_with = "intra")]
        inter: bool,
    },
    /// Compute EER from two HD CSVs.
    Eer {
        #[arg(long)]
        same: PathBuf,
        #[arg(long)]
        diff: PathBuf,
        #[arg(long)]
        curve: Option<PathBuf>,
    },
}

fn style() -> ProgressStyle {
    ProgressStyle::with_template("{bar:40.cyan/blue} {pos:>6}/{len:6} {msg}").unwrap()
}

fn enroll(casia_root: &Path, out: &Path, diag: Option<&Path>, limit: Option<usize>) -> Result<()> {
    let mut paths = casia::walk(casia_root)?;
    if let Some(n) = limit {
        paths.truncate(n);
    }
    if let Some(d) = diag {
        std::fs::create_dir_all(d).context("creating diagnostics dir")?;
    }

    let bincode_cfg = bincode::config::standard();
    let templates: Vec<EnrolledTemplate> = paths
        .par_iter()
        .progress_with_style(style())
        .with_message("enrolling")
        .filter_map(|p| {
            let result = (|| -> Result<EnrolledTemplate> {
                let img = casia::load_grayscale(&p.path)?;
                let seg = segment::segment_iris(&img);
                let polar = normalize::normalize(&seg.masked, seg.iris, seg.pupil);
                if let Some(d) = diag {
                    let stem = format!("{:03}_{}_{}", p.id.subject, p.id.session, p.id.index);
                    diagnostics::write_segmented(&d.join(format!("{stem}-segmented.jpg")), &img, seg.iris, seg.pupil)?;
                    diagnostics::write_polar(&d.join(format!("{stem}-polar.jpg")), &polar)?;
                    diagnostics::write_polar_noise(&d.join(format!("{stem}-noise.jpg")), &polar)?;
                }
                let (template, mask) = encode::encode(&polar);
                Ok(EnrolledTemplate {
                    id: p.id,
                    iris: seg.iris,
                    pupil: seg.pupil,
                    template,
                    mask,
                })
            })();
            match result {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("warn: skipping {}: {e:#}", p.path.display());
                    None
                }
            }
        })
        .collect();

    let mut writer = BufWriter::new(File::create(out)?);
    bincode::serde::encode_into_std_write(&templates, &mut writer, bincode_cfg)
        .context("writing bincode templates")?;
    writer.flush()?;
    println!("enrolled {} templates -> {}", templates.len(), out.display());
    Ok(())
}

fn match_cmd(templates_path: &Path, out: &Path, intra: bool, inter: bool) -> Result<()> {
    if !intra && !inter {
        anyhow::bail!("specify --intra or --inter");
    }
    let bincode_cfg = bincode::config::standard();
    let mut reader = BufReader::new(File::open(templates_path)?);
    let templates: Vec<EnrolledTemplate> =
        bincode::serde::decode_from_std_read(&mut reader, bincode_cfg)?;

    let pairs: Vec<(usize, usize)> = if intra {
        let mut v = Vec::new();
        for i in 0..templates.len() {
            for j in (i + 1)..templates.len() {
                if templates[i].id.subject == templates[j].id.subject {
                    v.push((i, j));
                }
            }
        }
        v
    } else {
        let mut v = Vec::new();
        for i in 0..templates.len() {
            for j in (i + 1)..templates.len() {
                if templates[i].id.subject != templates[j].id.subject {
                    v.push((i, j));
                }
            }
        }
        v
    };

    let results: Vec<(usize, usize, f64)> = pairs
        .par_iter()
        .progress_with_style(style())
        .with_message("matching")
        .map(|&(i, j)| {
            let hd = matching::hamming_distance(
                &templates[i].template,
                &templates[i].mask,
                &templates[j].template,
                &templates[j].mask,
            );
            (i, j, hd)
        })
        .collect();

    let mut w = BufWriter::new(File::create(out)?);
    writeln!(w, "i,j,subject_i,subject_j,hd")?;
    for (i, j, hd) in &results {
        let a = &templates[*i].id;
        let b = &templates[*j].id;
        writeln!(w, "{i},{j},{},{},{hd}", a.subject, b.subject)?;
    }
    println!("wrote {} HD values -> {}", results.len(), out.display());
    Ok(())
}

fn eer(same: &Path, diff: &Path, curve: Option<&Path>) -> Result<()> {
    let load = |p: &Path| -> Result<Vec<f64>> {
        let s = std::fs::read_to_string(p)?;
        let mut out = Vec::new();
        for (i, line) in s.lines().enumerate() {
            if i == 0 {
                continue;
            }
            if let Some(last) = line.split(',').last() {
                if let Ok(v) = last.parse::<f64>() {
                    if v.is_finite() {
                        out.push(v);
                    }
                }
            }
        }
        Ok(out)
    };
    let hd_same = load(same)?;
    let hd_diff = load(diff)?;
    println!("loaded {} same, {} diff", hd_same.len(), hd_diff.len());

    let n_pts = 101;
    let xs: Vec<f64> = (0..n_pts).map(|i| i as f64 / (n_pts - 1) as f64).collect();
    let fmr: Vec<f64> = xs
        .iter()
        .map(|&t| hd_diff.iter().filter(|&&v| v <= t).count() as f64 / hd_diff.len().max(1) as f64)
        .collect();
    let fnmr: Vec<f64> = xs
        .iter()
        .map(|&t| hd_same.iter().filter(|&&v| v > t).count() as f64 / hd_same.len().max(1) as f64)
        .collect();

    let mut eer = 1.0;
    for i in 0..xs.len() - 1 {
        let a = fmr[i] - fnmr[i];
        let b = fmr[i + 1] - fnmr[i + 1];
        if (a == 0.0) || (a.signum() != b.signum() && a != 0.0 && b != 0.0) {
            eer = (fmr[i] + fmr[i + 1] + fnmr[i] + fnmr[i + 1]) / 4.0;
            break;
        }
    }
    println!("EER = {eer:.4}");

    if let Some(c) = curve {
        let mut w = BufWriter::new(File::create(c)?);
        writeln!(w, "threshold,fmr,fnmr")?;
        for i in 0..xs.len() {
            writeln!(w, "{},{},{}", xs[i], fmr[i], fnmr[i])?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Enroll { casia, out, diagnostics, limit } => {
            enroll(&casia, &out, diagnostics.as_deref(), limit)
        }
        Cmd::Match { templates, out, intra, inter } => match_cmd(&templates, &out, intra, inter),
        Cmd::Eer { same, diff, curve } => eer(&same, &diff, curve.as_deref()),
    }
}
