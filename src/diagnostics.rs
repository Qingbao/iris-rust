use crate::types::{Circle, GrayF64, PolarArray};
use image::{GrayImage, Luma};
use std::path::Path;

fn gray_from_f64(img: &GrayF64) -> GrayImage {
    let (h, w) = img.dim();
    let mut out = GrayImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let v = img[(y, x)].clamp(0.0, 255.0) as u8;
            out.put_pixel(x as u32, y as u32, Luma([v]));
        }
    }
    out
}

fn draw_circle(img: &mut GrayImage, c: Circle, value: u8) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let steps = (c.r * std::f64::consts::TAU).max(60.0) as usize;
    for i in 0..steps {
        let a = i as f64 * std::f64::consts::TAU / steps as f64;
        let x = (c.x + c.r * a.cos()).round() as i32;
        let y = (c.y + c.r * a.sin()).round() as i32;
        if x >= 0 && x < w && y >= 0 && y < h {
            img.put_pixel(x as u32, y as u32, Luma([value]));
        }
    }
}

pub fn write_segmented(out: &Path, img: &GrayF64, iris: Circle, pupil: Circle) -> anyhow::Result<()> {
    let mut g = gray_from_f64(img);
    draw_circle(&mut g, iris, 255);
    draw_circle(&mut g, pupil, 255);
    g.save(out)?;
    Ok(())
}

pub fn write_polar(out: &Path, polar: &PolarArray) -> anyhow::Result<()> {
    let (h, w) = polar.dim();
    let mut g = GrayImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let v = polar[(y, x)].unwrap_or(0.0);
            let v = (v * 255.0).clamp(0.0, 255.0) as u8;
            g.put_pixel(x as u32, y as u32, Luma([v]));
        }
    }
    g.save(out)?;
    Ok(())
}

pub fn write_polar_noise(out: &Path, polar: &PolarArray) -> anyhow::Result<()> {
    let (h, w) = polar.dim();
    let mut g = GrayImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let v = if polar[(y, x)].is_none() { 255 } else { 0 };
            g.put_pixel(x as u32, y as u32, Luma([v]));
        }
    }
    g.save(out)?;
    Ok(())
}
