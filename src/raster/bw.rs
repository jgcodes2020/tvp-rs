use core::slice;
use std::{
    arch::x86_64::{
        __m256i, _mm256_add_epi8, _mm256_alignr_epi8, _mm256_and_si256, _mm256_bsrli_epi128, _mm256_castsi128_si256, _mm256_castsi256_si128, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8, _mm256_extract_epi8, _mm256_insert_epi8, _mm256_inserti128_si256, _mm256_load_si256, _mm256_movemask_epi8, _mm256_or_si256, _mm256_permute2x128_si256, _mm256_set1_epi32, _mm256_set1_epi8, _mm256_setzero_si256, _mm256_srli_epi16, _mm256_store_si256, _mm_alignr_epi8, _mm_cvtsi32_si128, _mm_insert_epi8, _mm_setzero_si128
    },
    array,
    error::Error,
    io::Write,
    ptr::null,
};

use rsmpeg::{avutil::AVFrame, error::RsmpegError};
use rusty_ffmpeg::ffi::{
    AVPixelFormat, AVPixelFormat_AV_PIX_FMT_GRAY8, AVPixelFormat_AV_PIX_FMT_YUV420P,
    AVERROR_INVALIDDATA,
};

use super::Raster;

struct BWRaster {}

impl Raster for BWRaster {
    fn present<T: Write>(src: &AVFrame, dest: &mut T) {
        let dithered = dither_frame(src).unwrap();
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
            let src_line = slice::from_raw_parts(
                src.data[0].add((i * src.linesize[0]) as usize),
                src.linesize[0] as usize,
            );
            let dst_line = slice::from_raw_parts_mut(
                dst.data[0].add((i * dst.linesize[0]) as usize),
                dst.linesize[0] as usize,
            );
            dither_line(src_line, dst_line, i as usize);
        }
    }

    Ok(dst)
}

// DITHERING
// =========

const fn u32_from_u8s(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

const DITHER_SHIFT: isize = 3;
const DITHER_MATRIX: [u32; 4] = [
    u32_from_u8s(0x00, 0x80, 0x20, 0xA0),
    u32_from_u8s(0xC0, 0x40, 0xE0, 0x60),
    u32_from_u8s(0x30, 0xB0, 0x10, 0x90),
    u32_from_u8s(0xF0, 0x70, 0xD0, 0x50),
];
unsafe fn dither_line(src_line: &[u8], dst_line: &mut [u8], row_index: usize) {
    
    // lengths
    assert!(src_line.len() == dst_line.len());
    assert!(src_line.len() % 32 == 0);
    // pointers
    let mut src_ptr = src_line.as_ptr();
    let mut dst_ptr = dst_line.as_mut_ptr();
    assert!((src_ptr as usize) % 32 == 0);
    assert!((dst_ptr as usize) % 32 == 0);
    // end pointers
    let src_end = src_line.as_ptr().add(src_line.len());

    while src_ptr < src_end {
        let mut chunk = _mm256_load_si256(src_ptr as *const __m256i);
        chunk = _mm256_cmpgt_epi8(
            chunk,
            _mm256_set1_epi32(DITHER_MATRIX[row_index % 4] as i32),
        );
        _mm256_store_si256(dst_line.as_mut_ptr() as *mut __m256i, chunk);

        src_ptr = src_ptr.add(32);
        dst_ptr = dst_ptr.add(32);
    }
}

// SIXEL OUTPUT
// ============

fn display_sixel<T: Write>(src: &AVFrame, out: &mut T) -> Result<(), Box<dyn Error>> {
    // sixel header
    write!(out, "\x1B[40m\x1BPq#0;2;100;100;100")?;

    let mut buffer = Vec::<u8>::new();
    buffer.reserve((src.width * src.height) as usize);
    buffer.clear();

    unsafe {
    }

    // sixel terminator
    write!(out, "\x1B\\")?;

    Ok(())
}

unsafe fn prepare_row<T: Write>(out: &mut T, frame: &AVFrame, row_index: i32) -> Result<(), Box<dyn Error>> {
    let line_size = frame.linesize[0];
    let line_spans: [*const u8; 6] = array::from_fn(|i| {
        let index = row_index + i as i32;
        if index >= frame.height {
            null()
        } else {
            frame.data[0].add((index * line_size) as usize)
        }
    });

    assert!(line_size % 32 == 0);
    assert!(line_spans[0] as isize % 32 == 0);

    // byte carried between groups for switch
    let mut carry: u8 = b'\0';
    // information about the last edge
    let mut last_edge: i32 = -1;

    for i in (0..line_size - 32).step_by(32) {
        // read (32 columns x 6 rows) and encode them to raw sixel characters
        let chunk = _mm256_add_epi8({
            line_spans
                .iter()
                .enumerate()
                // map line pointers to corresponding chunks
                .map(|(j, p)| {
                    if p.is_null() {
                        // treat past-the-end rows as empty
                        return _mm256_setzero_si256();
                    } else {
                        // isolate the correct bit for this row
                        let mut chunk = _mm256_load_si256(p.add(i as usize) as *const __m256i);
                        chunk = _mm256_and_si256(chunk, _mm256_set1_epi8(1 << j));
                        return chunk;
                    }
                })
                // combine the bits from all the rows
                .fold(_mm256_setzero_si256(), |chunk, row| _mm256_or_si256(chunk, row))
                // add the 0x3F offset
        }, _mm256_set1_epi8(0x3F));

        // 
        {
            // shift forward by 1 byte, pulling in the carry.
            let shifted = {
                // https://stackoverflow.com/questions/20775005/8-bit-shift-operation-in-avx2-with-shifting-in-zeros
                let align_low = _mm256_castsi128_si256(_mm_insert_epi8::<15>(_mm_setzero_si128(), carry as i32));
                let align_full = _mm256_inserti128_si256::<1>(align_low, _mm256_castsi256_si128(chunk));
                _mm256_alignr_epi8::<15>(align_full, chunk)
            };
            // get a bitmask of edges (where the bits change)
            let mut diff = !(_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, shifted)) as u32);
            // iterate over all set bits in diff
            while diff != 0 {
                let off = diff.leading_zeros();
                

                diff &= diff - 1;
            }
        }

        

        carry = _mm256_extract_epi8::<31>(chunk) as u8;
    }

    Ok(())
}
