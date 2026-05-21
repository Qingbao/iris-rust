use crate::daugman::{search_inner_boundary, search_outer_boundary};
use crate::eyelid::{find_line, y_at_x};
use crate::types::{Circle, GrayF64};
use ndarray::{s, Array2};

const EYELASH_THRESHOLD: f64 = 80.0;

#[derive(Debug, Clone)]
pub struct Segmentation {
    pub iris: Circle,
    pub pupil: Circle,
    /// Same shape as input image. `None` marks noise (eyelid / eyelash / out-of-iris).
    pub masked: Array2<Option<f64>>,
}

pub fn segment_iris(img: &GrayF64) -> Segmentation {
    let pupil = search_inner_boundary(img);
    let iris = search_outer_boundary(img, pupil);

    let (h, w) = img.dim();
    let mut masked: Array2<Option<f64>> = img.map(|v| Some(*v));

    // Top eyelid: search the strip above the pupil.
    let top_y_end = (pupil.y - pupil.r).floor().max(1.0) as usize;
    if top_y_end > 2 {
        let strip = img.slice(s![..top_y_end, ..]).to_owned();
        if let Some(line) = find_line(&strip) {
            for x in 0..w {
                if let Some(yl) = y_at_x(line, x as f64) {
                    let yl = yl.round().clamp(0.0, top_y_end as f64) as usize;
                    for y in 0..=yl.min(h - 1) {
                        masked[(y, x)] = None;
                    }
                }
            }
        }
    }
    // Bottom eyelid: search strip below the pupil.
    let bot_y_start = (pupil.y + pupil.r).ceil().max(0.0) as usize;
    if bot_y_start + 2 < h {
        let strip = img.slice(s![bot_y_start.., ..]).to_owned();
        if let Some(line) = find_line(&strip) {
            for x in 0..w {
                if let Some(yl) = y_at_x(line, x as f64) {
                    let yl = (yl + bot_y_start as f64).round().clamp(0.0, h as f64 - 1.0) as usize;
                    for y in yl..h {
                        masked[(y, x)] = None;
                    }
                }
            }
        }
    }

    // Eyelash thresholding (CASIA-specific).
    for y in 0..h {
        for x in 0..w {
            if img[(y, x)] < EYELASH_THRESHOLD {
                masked[(y, x)] = None;
            }
        }
    }

    Segmentation { iris, pupil, masked }
}
