use std::{error::Error, io::{Write}};

pub mod bw;
pub use bw::*;
use rsmpeg::avutil::AVFrame;

pub trait Raster {
    fn present<T: Write>(src: &AVFrame, dest: &mut T) -> Result<(), Box<dyn Error>>;
}