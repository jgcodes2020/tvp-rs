use core::slice;
use std::{
    arch::x86_64::*,
    array, cmp,
    error::Error,
    fs::File,
    io::Write,
    mem::{self, MaybeUninit},
    path::Path,
    ptr::null,
};

use rsmpeg::{avutil::AVFrame, error::RsmpegError};
use rusty_ffmpeg::ffi::{
    AVPixelFormat_AV_PIX_FMT_GRAY8, AVPixelFormat_AV_PIX_FMT_YUV420P, AVERROR_INVALIDDATA,
};

use super::Raster;

pub struct BWRaster {}

impl Raster for BWRaster {
    fn present<T: Write>(src: &AVFrame, dest: &mut T) -> Result<(), Box<dyn Error>> {
        let dithered = dither_frame(src)?;
        // save_pgm(&dithered, Path::new("/dev/shm/blablablablabla-output.pgm"))?;

        encode_sixel(&dithered, dest);
        Ok(())
    }
}

// DITHERING

#[allow(unused)]
fn save_pgm(src: &AVFrame, dst: &Path) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(dst)?;

    write!(&mut file, "P5 {} {} 255\n", src.width, src.height)?;
    unsafe {
        for i in 0..src.height {
            file.write(slice::from_raw_parts(
                src.data[0].add((src.linesize[0] * i) as usize),
                src.width as usize,
            ))?;
        }
    }

    Ok(())
}

fn dither_frame(src: &AVFrame) -> Result<AVFrame, RsmpegError> {
    if src.format != AVPixelFormat_AV_PIX_FMT_YUV420P {
        return Err(RsmpegError::AVError(AVERROR_INVALIDDATA));
    }

    let mut dst = AVFrame::new();

    dst.set_width(src.width);
    dst.set_height(src.height);
    dst.set_format(AVPixelFormat_AV_PIX_FMT_GRAY8);
    dst.alloc_buffer()?;

    // SIMD dither filter
    unsafe {
        for i in 0..src.height {
            let line_len = cmp::min(src.linesize[0], dst.linesize[0]) as usize;

            let src_line =
                slice::from_raw_parts(src.data[0].add((i * src.linesize[0]) as usize), line_len);
            let dst_line = slice::from_raw_parts_mut(
                dst.data[0].add((i * dst.linesize[0]) as usize),
                line_len,
            );
            dither_line(src_line, dst_line, i as usize);
        }
    }

    Ok(dst)
}

unsafe fn dither_line(src_line: &[u8], dst_line: &mut [u8], _row_index: usize) {
    // lengths
    assert_eq!(src_line.len(), dst_line.len());
    assert!(src_line.len() % 32 == 0);
    // pointers
    let mut src_ptr = src_line.as_ptr();
    let mut dst_ptr = dst_line.as_mut_ptr();
    assert!((src_ptr as usize) % 32 == 0);
    assert!((dst_ptr as usize) % 32 == 0);
    // end pointers
    let src_end = src_line.as_ptr().add(src_line.len());

    while src_ptr < src_end {
        let mut chunk = _mm256_loadu_si256(src_ptr as *const __m256i);
        chunk = {
            // _mm256_set1_epi32(DITHER_MATRIX[row_index % 4] as i32)
            let thresh = _mm256_set1_epi8(0x80u8 as i8);
            _mm256_cmpeq_epi8(_mm256_max_epu8(chunk, thresh), thresh)
        };
        _mm256_storeu_si256(dst_ptr as *mut __m256i, chunk);

        src_ptr = src_ptr.add(32);
        dst_ptr = dst_ptr.add(32);
    }
}

// SIXEL ENCODING

fn encode_sixel<T: Write>(src: &AVFrame, dest: &mut T) -> Result<(), Box<dyn Error>> {


    Ok(())
}