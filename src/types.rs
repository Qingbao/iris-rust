use bitvec::prelude::*;
use ndarray::Array2;
use serde::{Deserialize, Serialize};

pub const RADIAL_RES: usize = 20;
pub const ANGULAR_RES: usize = 240;
pub const TEMPLATE_BITS: usize = RADIAL_RES * ANGULAR_RES * 2;

pub type GrayF64 = Array2<f64>;
pub type PolarArray = Array2<Option<f64>>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Circle {
    pub y: f64,
    pub x: f64,
    pub r: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub bits: BitVec<u64, Lsb0>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
    pub bits: BitVec<u64, Lsb0>,
}

impl Template {
    pub fn zeros() -> Self {
        Self { bits: BitVec::repeat(false, TEMPLATE_BITS) }
    }
}

impl Mask {
    pub fn zeros() -> Self {
        Self { bits: BitVec::repeat(false, TEMPLATE_BITS) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageId {
    pub subject: u16,
    pub session: u8,
    pub index: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrolledTemplate {
    pub id: ImageId,
    pub iris: Circle,
    pub pupil: Circle,
    pub template: Template,
    pub mask: Mask,
}
