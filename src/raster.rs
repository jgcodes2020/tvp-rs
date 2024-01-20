use std::io::{self, Write};

pub mod bw;
pub use bw::*;
use rsmpeg::avutil::AVFrame;

pub trait Raster {
    fn present<T: Write>(src: &AVFrame, dest: &mut T);
}