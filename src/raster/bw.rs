use core::slice;
use std::{
    arch::x86_64::*,
    array, cmp,
    error::Error,
    io::Write,
};

use rsmpeg::{avutil::AVFrame, error::RsmpegError};
use rusty_ffmpeg::ffi::{
    AVPixelFormat_AV_PIX_FMT_GRAY8, AVPixelFormat_AV_PIX_FMT_YUV420P, AVERROR_INVALIDDATA,
};

use super::Raster;

pub struct BWRaster {}


static mut ENCODE_BUFFER: Vec<u8> = Vec::new();

impl Raster for BWRaster {

    fn present<T: Write>(src: &AVFrame, dest: &mut T) -> Result<(), Box<dyn Error>> {
        let dithered = dither_frame(src)?;
        unsafe {
            ENCODE_BUFFER.clear();
            ENCODE_BUFFER.clear();
            encode_sixel(&dithered, &mut ENCODE_BUFFER)?;
            dest.write_all(ENCODE_BUFFER.as_slice())?;
        }
        Ok(())
    }
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
// [['18', '80', '32', '9A'], ['B4', '4C', 'CE', '66'], ['3F', 'A7', '25', '8D'], ['DB', '73', 'C1', '59']]
const DITHER_MATRIX: [u32; 4] = [
    0x18_80_32_9Au32,
    0x66_CE_4C_B4u32,
    0x8D_25_A7_3Fu32,
    0xDB_73_C1_59u32,
];
// 0x00, 0x80, 0x20, 0xA0,
// 0xC0, 0x40, 0xE0, 0x60,
// 0x30, 0xB0, 0x10, 0x90,
// 0xF0, 0x70, 0xD0, 0x50
// 0-127 | 128-255

unsafe fn dither_line(src_line: &[u8], dst_line: &mut [u8], row_index: usize) {
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
            let thresh = _mm256_set1_epi32(DITHER_MATRIX[row_index % 4] as i32);
            _mm256_cmpeq_epi8(_mm256_max_epu8(thresh, chunk), chunk)
        };
        _mm256_storeu_si256(dst_ptr as *mut __m256i, chunk);

        src_ptr = src_ptr.add(32);
        dst_ptr = dst_ptr.add(32);
    }
}

// SIXEL ENCODING

fn encode_sixel<T: Write>(src: &AVFrame, dest: &mut T) -> Result<(), Box<dyn Error>> {
    // sixel header
    dest.write_all(b"\x1BPq#0;2;0;0;0#1;2;100;100;100#1")?;

    // rows
    unsafe {
        let height = src.height as u32;
        let line_size = src.linesize[0] as u32;
        for i in (0..height).step_by(6) {
            let lines: [Option<&[u8]>; 6] = array::from_fn(|j| {
                if i + (j as u32) >= height {
                    None
                } else {
                    let ptr = src.data[0].add(((i as usize) + j) * (line_size as usize));
                    return Some(slice::from_raw_parts(ptr, src.width as usize));
                }
            });
            encode_sixel_row(&lines, dest)?;
            // row termintor
            dest.write_all(b"-")?;
        }
    }

    // sixel footer
    dest.write_all(b"\x1B\\")?;
    dest.flush()?;
    Ok(())
}

unsafe fn ymm_to_u8_array(chunk: __m256i) -> [u8; 32] {
    let mut data: [u8; 32] = [0; 32];
    _mm256_storeu_si256(data.as_mut_ptr() as *mut __m256i, chunk);
    data
}

unsafe fn encode_sixel_row<T: Write>(
    src_lines: &[Option<&[u8]>; 6],
    dest: &mut T,
) -> Result<(), Box<dyn Error>> {
    let mut carry: u8 = b'\0';
    let mut edge: (usize, u8) = (0, b'\0');

    let mut i: usize = 0;

    let mut next_edge = |index: usize, value: u8| -> Result<(), Box<dyn Error>> {
        let run_len = index - edge.0;
        if run_len == 0 {
        } else if run_len <= 3 {
            let data: [u8; 3] = [edge.1; 3];
            dest.write_all(&data[0..run_len])?;
        } else {
            write!(dest, "!{}{}", run_len, edge.1 as char)?;
        }

        edge = (index, value);
        Ok(())
    };

    let row_len = src_lines[0].unwrap().len();

    // AVX2 loop
    #[cfg(target_feature = "avx2")]
    while i + 31 < row_len {
        // read 32x6 region; extract and combine corresponding bits; add 0x3F to get sixel chars
        let chunk: __m256i = _mm256_add_epi8(
            src_lines
                .iter()
                .enumerate()
                .map(|(index, row)| match row {
                    Some(slice) => {
                        let chunk = _mm256_loadu_si256((&slice[i] as *const u8) as *const __m256i);
                        _mm256_and_si256(chunk, _mm256_set1_epi8(1 << index))
                    }
                    None => _mm256_setzero_si256(),
                })
                .fold(_mm256_setzero_si256(), |chunk, row| {
                    _mm256_or_si256(chunk, row)
                }),
            _mm256_set1_epi8(0x3Fu8 as i8),
        );
        let chunk_arr = ymm_to_u8_array(chunk);
        // shift forward by 1, shifting in the carry value
        let shifted = {
            let mut align_out = _mm256_permute2x128_si256::<0x08>(chunk, chunk);
            align_out = _mm256_insert_epi8::<15>(align_out, carry as i8);
            _mm256_alignr_epi8::<15>(chunk, align_out)
        };
        // get the shifted out carry byte
        carry = chunk_arr[31];
        // compare shifted with not shifted; generating a bitmask of edges
        let mut edge_mask = !_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, shifted)) as u32;
        // iterate over all found edges
        while edge_mask != 0 {
            let offset = edge_mask.trailing_zeros() as usize;
            let edge_char = chunk_arr[offset];
            next_edge(i + offset, edge_char)?;

            // clear the current lowest set bit
            edge_mask &= edge_mask - 1;
        }

        i += 31;
    }
    // serial loop
    while i < row_len {
        // read 1x6 region; extract and combine corresponding bits; add 0x3F to get a sixel char
        let chunk: u8 = src_lines
            .iter()
            .enumerate()
            .map(|(index, row)| {
                row.and_then(|slice| Some(slice[i] & (1 << index)))
                    .unwrap_or(0)
            })
            .fold(0, |chunk, row| chunk | row)
            + 0x3F;
        
        // check for edge with previous character
        if chunk != carry {
            next_edge(i, chunk)?;
        }
        carry = chunk;

        i += 1;
    }
    next_edge(i, b'\0')?;

    Ok(())
}
