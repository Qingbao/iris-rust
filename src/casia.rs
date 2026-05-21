use crate::types::ImageId;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub const SUBJECTS: u16 = 108;
pub const SESSION1_IMAGES: u8 = 3;
pub const SESSION2_IMAGES: u8 = 4;
pub const IMAGES_PER_SUBJECT: usize = (SESSION1_IMAGES + SESSION2_IMAGES) as usize;
pub const TOTAL_IMAGES: usize = SUBJECTS as usize * IMAGES_PER_SUBJECT;

pub struct CasiaPath {
    pub id: ImageId,
    pub path: PathBuf,
}

pub fn walk(root: &Path) -> Result<Vec<CasiaPath>> {
    let mut out = Vec::with_capacity(TOTAL_IMAGES);
    for subject in 1..=SUBJECTS {
        for session in 1..=2u8 {
            let count = if session == 1 { SESSION1_IMAGES } else { SESSION2_IMAGES };
            for index in 1..=count {
                let path = root
                    .join(format!("{subject:03}"))
                    .join(format!("{session}"))
                    .join(format!("{subject:03}_{session}_{index}.bmp"));
                if !path.exists() {
                    anyhow::bail!("missing CASIA image: {}", path.display());
                }
                out.push(CasiaPath {
                    id: ImageId { subject, session, index },
                    path,
                });
            }
        }
    }
    Ok(out)
}

pub fn load_grayscale(path: &Path) -> Result<crate::types::GrayF64> {
    let img = image::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .to_luma8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let buf = img.into_raw();
    let arr = ndarray::Array2::from_shape_vec((h, w), buf.into_iter().map(|p| p as f64).collect())
        .context("reshape grayscale buffer")?;
    Ok(arr)
}
