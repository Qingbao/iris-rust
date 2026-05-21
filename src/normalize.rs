use crate::types::{Circle, ANGULAR_RES, RADIAL_RES};
use ndarray::{Array2, Array2 as Arr2};

/// Daugman rubber-sheet normalization with pupil-iris offset.
/// Output shape: (RADIAL_RES, ANGULAR_RES). `None` cells = noise.
pub fn normalize(masked: &Arr2<Option<f64>>, iris: Circle, pupil: Circle) -> Array2<Option<f64>> {
    let radiuspixels = RADIAL_RES + 2;
    let angledivisions = ANGULAR_RES - 1;

    let ox = pupil.x - iris.x;
    let oy = pupil.y - iris.y;
    let sgn = if ox <= 0.0 && !(ox == 0.0 && oy > 0.0) { -1.0 } else { 1.0 };
    let a = ox * ox + oy * oy;
    let phi = if ox == 0.0 { std::f64::consts::FRAC_PI_2 } else { (oy / ox).atan() };

    let theta: Vec<f64> = (0..ANGULAR_RES)
        .map(|i| 2.0 * std::f64::consts::PI * i as f64 / angledivisions as f64)
        .collect();

    // Per-angle iris radius (distance from pupil center to iris boundary along that angle).
    let r_at_theta: Vec<f64> = theta
        .iter()
        .map(|t| {
            let b = sgn * (std::f64::consts::PI - phi - t).cos();
            let disc = (a * b * b - (a - iris.r * iris.r)).max(0.0);
            (a.sqrt() * b) + disc.sqrt() - pupil.r
        })
        .collect();

    // Interpolate masked image bilinearly.
    let interp = |y: f64, x: f64| -> Option<f64> {
        let (h, w) = masked.dim();
        if y < 0.0 || x < 0.0 || y > (h - 1) as f64 || x > (w - 1) as f64 {
            return None;
        }
        let y0 = y.floor() as usize;
        let x0 = x.floor() as usize;
        let y1 = (y0 + 1).min(h - 1);
        let x1 = (x0 + 1).min(w - 1);
        let fy = y - y0 as f64;
        let fx = x - x0 as f64;
        let v00 = masked[(y0, x0)]?;
        let v01 = masked[(y0, x1)]?;
        let v10 = masked[(y1, x0)]?;
        let v11 = masked[(y1, x1)]?;
        let top = v00 * (1.0 - fx) + v01 * fx;
        let bot = v10 * (1.0 - fx) + v11 * fx;
        Some(top * (1.0 - fy) + bot * fy)
    };

    // Build the full (radiuspixels, ANGULAR_RES) array, then trim outer two rings.
    let mut full = Array2::<Option<f64>>::from_elem((radiuspixels, ANGULAR_RES), None);
    for ri in 0..radiuspixels {
        let rstep = ri as f64 / (radiuspixels - 1) as f64;
        for ai in 0..ANGULAR_RES {
            let r = r_at_theta[ai] * rstep + pupil.r;
            let x = pupil.x + r * theta[ai].cos();
            let y = pupil.y - r * theta[ai].sin();
            full[(ri, ai)] = interp(y, x).map(|v| v / 255.0);
        }
    }

    // Strip the outer rings (rows 0 and radiuspixels-1) — MATLAB's rmat(2:radiuspixels-1, :).
    let mut out = Array2::<Option<f64>>::from_elem((RADIAL_RES, ANGULAR_RES), None);
    for ri in 0..RADIAL_RES {
        for ai in 0..ANGULAR_RES {
            out[(ri, ai)] = full[(ri + 1, ai)];
        }
    }
    out
}
