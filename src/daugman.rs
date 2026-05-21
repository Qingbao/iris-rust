use crate::types::{Circle, GrayF64};
use ndarray::Array3;

pub fn contour_integral_circular(img: &GrayF64, y0: f64, x0: f64, r: f64, angs: &[f64]) -> f64 {
    let (h, w) = img.dim();
    let mut sum = 0.0;
    for &a in angs {
        let y = (y0 - a.cos() * r).round() as isize;
        let x = (x0 + a.sin() * r).round() as isize;
        let y = y.clamp(0, h as isize - 1) as usize;
        let x = x.clamp(0, w as isize - 1) as usize;
        sum += img[(y, x)];
    }
    sum
}

fn partial_derivative_r(hs: &Array3<f64>) -> Array3<f64> {
    let (ny, nx, nr) = hs.dim();
    let mut out = Array3::<f64>::zeros((ny, nx, nr));
    for y in 0..ny {
        for x in 0..nx {
            for r in 0..nr {
                let prev = if r == 0 { hs[(y, x, 0)] } else { hs[(y, x, r - 1)] };
                out[(y, x, r)] = hs[(y, x, r)] - prev;
            }
        }
    }
    out
}

fn box_blur_3d(hs: &Array3<f64>, sm: usize) -> Array3<f64> {
    let (ny, nx, nr) = hs.dim();
    let half = (sm / 2) as isize;
    let mut out = Array3::<f64>::zeros((ny, nx, nr));
    for y in 0..ny {
        for x in 0..nx {
            for r in 0..nr {
                let mut acc = 0.0;
                for dy in -half..=half {
                    let yy = y as isize + dy;
                    if yy < 0 || yy >= ny as isize { continue; }
                    for dx in -half..=half {
                        let xx = x as isize + dx;
                        if xx < 0 || xx >= nx as isize { continue; }
                        for dr in -half..=half {
                            let rr = r as isize + dr;
                            if rr < 0 || rr >= nr as isize { continue; }
                            acc += hs[(yy as usize, xx as usize, rr as usize)];
                        }
                    }
                }
                out[(y, x, r)] = acc;
            }
        }
    }
    out
}

fn argmax_3d(a: &Array3<f64>) -> (usize, usize, usize) {
    let (ny, nx, nr) = a.dim();
    let mut best = f64::NEG_INFINITY;
    let mut idx = (0, 0, 0);
    for y in 0..ny {
        for x in 0..nx {
            for r in 0..nr {
                let v = a[(y, x, r)];
                if v > best {
                    best = v;
                    idx = (y, x, r);
                }
            }
        }
    }
    idx
}

fn linspace_step(start: f64, step: f64, end: f64) -> Vec<f64> {
    let n = ((end - start) / step).floor() as usize + 1;
    (0..n).map(|i| start + i as f64 * step).collect()
}

pub fn search_inner_boundary(img: &GrayF64) -> Circle {
    let (h, w) = img.dim();
    let sect = w as f64 / 4.0;
    let minrad = 10.0;
    let maxrad = sect * 0.8;
    let jump = 4.0;

    // Coarse
    let ny = ((h as f64 - 2.0 * sect) / jump).floor() as usize;
    let nx = ((w as f64 - 2.0 * sect) / jump).floor() as usize;
    let nr = ((maxrad - minrad) / jump).floor() as usize;
    let angs: Vec<f64> = linspace_step(0.0, 1.0, std::f64::consts::TAU);
    let mut hs = Array3::<f64>::zeros((ny, nx, nr));
    for x in 0..nx {
        for y in 0..ny {
            for r in 0..nr {
                let cy = sect + (y + 1) as f64 * jump;
                let cx = sect + (x + 1) as f64 * jump;
                let cr = minrad + (r + 1) as f64 * jump;
                hs[(y, x, r)] = contour_integral_circular(img, cy, cx, cr, &angs);
            }
        }
    }
    let hspdr = partial_derivative_r(&hs);
    let hspdrs = box_blur_3d(&hspdr, 3);
    let (yi, xi, ri) = argmax_3d(&hspdrs);
    let mut inner_y = sect + (yi + 1) as f64 * jump;
    let mut inner_x = sect + (xi + 1) as f64 * jump;
    let mut inner_r = minrad + ri as f64 * jump;

    // Fine
    let n = (jump * 2.0) as usize;
    let angs: Vec<f64> = linspace_step(0.0, 0.1, std::f64::consts::TAU);
    let mut hs = Array3::<f64>::zeros((n, n, n));
    for x in 0..n {
        for y in 0..n {
            for r in 0..n {
                let cy = inner_y - jump + (y + 1) as f64;
                let cx = inner_x - jump + (x + 1) as f64;
                let cr = inner_r - jump + (r + 1) as f64;
                hs[(y, x, r)] = contour_integral_circular(img, cy, cx, cr, &angs);
            }
        }
    }
    let hspdr = partial_derivative_r(&hs);
    let hspdrs = box_blur_3d(&hspdr, 3);
    let (yi, xi, ri) = argmax_3d(&hspdrs);
    inner_y = inner_y - jump + (yi + 1) as f64;
    inner_x = inner_x - jump + (xi + 1) as f64;
    inner_r = inner_r - jump + ri as f64;

    Circle { y: inner_y, x: inner_x, r: inner_r }
}

pub fn search_outer_boundary(img: &GrayF64, inner: Circle) -> Circle {
    let maxdispl = (inner.r * 0.15).round() as usize;
    let minrad = (inner.r / 0.8).round() as usize;
    let maxrad = (inner.r / 0.3).round() as usize;

    let intreg = [(2.0 / 6.0, 4.0 / 6.0), (8.0 / 6.0, 10.0 / 6.0)];
    let prec = 0.05;
    let mut angs = Vec::new();
    for (lo, hi) in intreg {
        let mut a = lo * std::f64::consts::PI;
        let hi = hi * std::f64::consts::PI;
        while a <= hi {
            angs.push(a);
            a += prec;
        }
    }

    let ny = 2 * maxdispl;
    let nx = 2 * maxdispl;
    let nr = maxrad - minrad;
    let mut hs = Array3::<f64>::zeros((ny, nx, nr));
    for x in 0..nx {
        for y in 0..ny {
            for r in 0..nr {
                let cy = inner.y - maxdispl as f64 + (y + 1) as f64;
                let cx = inner.x - maxdispl as f64 + (x + 1) as f64;
                let cr = (minrad + r + 1) as f64;
                hs[(y, x, r)] = contour_integral_circular(img, cy, cx, cr, &angs);
            }
        }
    }
    let hspdr = partial_derivative_r(&hs);
    let hspdrs = box_blur_3d(&hspdr, 7);
    let (yi, xi, ri) = argmax_3d(&hspdrs);

    Circle {
        y: inner.y - maxdispl as f64 + (yi + 1) as f64,
        x: inner.x - maxdispl as f64 + (xi + 1) as f64,
        r: (minrad + ri) as f64,
    }
}

