use crate::types::{Mask, PolarArray, Template, ANGULAR_RES, RADIAL_RES};
use rustfft::{num_complex::Complex64, FftPlanner};

const MIN_WAVELENGTH: f64 = 18.0;
const SIGMA_ON_F: f64 = 0.5;
const AMPLITUDE_FLOOR: f64 = 1e-4;

fn fill_with_average(polar: &PolarArray) -> ndarray::Array2<f64> {
    let (h, w) = polar.dim();
    let mut sum = 0.0;
    let mut count = 0usize;
    for v in polar.iter().flatten() {
        sum += v;
        count += 1;
    }
    let avg = if count == 0 { 0.5 } else { sum / count as f64 };
    let mut filled = ndarray::Array2::<f64>::zeros((h, w));
    for r in 0..h {
        for c in 0..w {
            filled[(r, c)] = polar[(r, c)].unwrap_or(avg);
        }
    }
    filled
}

fn build_log_gabor(n: usize) -> Vec<f64> {
    let half = n / 2;
    let fo = 1.0 / MIN_WAVELENGTH;
    let mut filter = vec![0.0; n];
    let denom = 2.0 * SIGMA_ON_F.ln().powi(2);
    for k in 1..=half {
        let r = k as f64 / half as f64 / 2.0;
        filter[k] = (-((r / fo).ln().powi(2)) / denom).exp();
    }
    // k = 0 stays 0 (DC removed).
    filter
}

pub fn encode(polar: &PolarArray) -> (Template, Mask) {
    debug_assert_eq!(polar.dim(), (RADIAL_RES, ANGULAR_RES));
    let n = ANGULAR_RES; // already even
    let filter = build_log_gabor(n);
    let filled = fill_with_average(polar);

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(n);
    let ifft = planner.plan_fft_inverse(n);

    let mut template = Template::zeros();
    let mut mask = Mask::zeros();

    for row in 0..RADIAL_RES {
        let mut buf: Vec<Complex64> = (0..n).map(|c| Complex64::new(filled[(row, c)], 0.0)).collect();
        fft.process(&mut buf);
        for k in 0..n {
            buf[k] *= filter[k];
        }
        ifft.process(&mut buf);
        // rustfft is unnormalized in both directions; divide by n to match MATLAB ifft scaling.
        let scale = 1.0 / n as f64;
        for c in 0..n {
            let e = buf[c] * scale;
            let small_amp = e.norm() < AMPLITUDE_FLOOR;
            let noise = polar[(row, c)].is_none();
            let bit_real_idx = row * (n * 2) + 2 * c;
            let bit_imag_idx = bit_real_idx + 1;
            template.bits.set(bit_real_idx, e.re > 0.0);
            template.bits.set(bit_imag_idx, e.im > 0.0);
            mask.bits.set(bit_real_idx, noise || small_amp);
            mask.bits.set(bit_imag_idx, noise || small_amp);
        }
    }

    (template, mask)
}
