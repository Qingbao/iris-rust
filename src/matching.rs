use crate::types::{Mask, Template, ANGULAR_RES, RADIAL_RES};
use bitvec::prelude::*;

const COLS: usize = ANGULAR_RES * 2;
const SCALES: i32 = 1;
const MAX_SHIFT: i32 = 8;

fn shift_columns(bits: &BitVec<u64, Lsb0>, noshifts: i32) -> BitVec<u64, Lsb0> {
    if noshifts == 0 {
        return bits.clone();
    }
    let shift = (2 * SCALES * noshifts.abs()) as usize;
    let mut out: BitVec<u64, Lsb0> = BitVec::repeat(false, bits.len());
    for row in 0..RADIAL_RES {
        let base = row * COLS;
        for c in 0..COLS {
            let src_c = if noshifts > 0 {
                // shift right: new[c] = old[c - shift]
                (c + COLS - shift) % COLS
            } else {
                // shift left: new[c] = old[c + shift]
                (c + shift) % COLS
            };
            let b = bits[base + src_c];
            out.set(base + c, b);
        }
    }
    out
}

pub fn hamming_distance(t1: &Template, m1: &Mask, t2: &Template, m2: &Mask) -> f64 {
    let mut best = f64::INFINITY;
    let total = t1.bits.len();
    for s in -MAX_SHIFT..=MAX_SHIFT {
        let t1s = shift_columns(&t1.bits, s);
        let m1s = shift_columns(&m1.bits, s);
        let combined = m1s | m2.bits.clone();
        let mask_count = combined.count_ones();
        let valid = total - mask_count;
        if valid == 0 {
            continue;
        }
        let diff = (t1s ^ t2.bits.clone()) & !combined;
        let hd = diff.count_ones() as f64 / valid as f64;
        if hd < best {
            best = hd;
        }
    }
    best
}
