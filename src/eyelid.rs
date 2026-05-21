use crate::types::GrayF64;
use image::{GrayImage, Luma};
use imageproc::edges::canny;

#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub theta_deg: i32,
    pub rho: f64,
}

fn to_gray_u8(img: &GrayF64) -> GrayImage {
    let (h, w) = img.dim();
    let mut g = GrayImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let v = img[(y, x)].clamp(0.0, 255.0) as u8;
            g.put_pixel(x as u32, y as u32, Luma([v]));
        }
    }
    g
}

fn edge_map(img: &GrayF64) -> Vec<(usize, usize)> {
    let g = to_gray_u8(img);
    let edges = canny(&g, 20.0, 60.0);
    let mut pts = Vec::new();
    for y in 0..edges.height() {
        for x in 0..edges.width() {
            if edges.get_pixel(x, y).0[0] > 0 {
                pts.push((y as usize, x as usize));
            }
        }
    }
    pts
}

pub fn find_line(img: &GrayF64) -> Option<Line> {
    let (h, w) = img.dim();
    if h < 3 || w < 3 {
        return None;
    }
    let pts = edge_map(img);
    if pts.is_empty() {
        return None;
    }
    let diag = ((h * h + w * w) as f64).sqrt();
    let rho_max = diag.ceil() as i32;
    let rho_bins = 2 * rho_max + 1;
    let theta_bins = 180usize;
    let mut acc = vec![0u32; theta_bins * rho_bins as usize];

    let cos_t: Vec<f64> = (0..theta_bins).map(|t| (t as f64 * std::f64::consts::PI / 180.0).cos()).collect();
    let sin_t: Vec<f64> = (0..theta_bins).map(|t| (t as f64 * std::f64::consts::PI / 180.0).sin()).collect();

    for &(y, x) in &pts {
        for t in 0..theta_bins {
            let r = x as f64 * cos_t[t] + y as f64 * sin_t[t];
            let ri = (r.round() as i32 + rho_max) as usize;
            if ri < rho_bins as usize {
                acc[t * rho_bins as usize + ri] += 1;
            }
        }
    }

    let (mut best, mut best_t, mut best_r) = (0u32, 0usize, 0usize);
    for t in 0..theta_bins {
        for r in 0..rho_bins as usize {
            let v = acc[t * rho_bins as usize + r];
            if v > best {
                best = v;
                best_t = t;
                best_r = r;
            }
        }
    }
    if best < 25 {
        return None;
    }
    let rho = best_r as f64 - rho_max as f64;
    Some(Line { theta_deg: best_t as i32, rho })
}

/// y as a function of x on the detected line, or None if the line is vertical.
pub fn y_at_x(line: Line, x: f64) -> Option<f64> {
    let t = line.theta_deg as f64 * std::f64::consts::PI / 180.0;
    let s = t.sin();
    if s.abs() < 1e-6 {
        None
    } else {
        Some((line.rho - x * t.cos()) / s)
    }
}
