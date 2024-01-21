use std::{error::Error, ffi::CString, path::{Path, PathBuf}};

use rsmpeg::{avcodec::{AVCodec, AVCodecContext}, avformat::AVFormatContextInput, avutil::{AVDictionary, AVFrame}, error::RsmpegError};
use rusty_ffmpeg::ffi::{AVERROR_DECODER_NOT_FOUND, AVERROR_STREAM_NOT_FOUND, AVRational};

use crate::enums::AVMediaType;

pub struct VideoDecoder {
    fmt_ctx: AVFormatContextInput,
    
    video_stream: usize,
    video_decoder: AVCodecContext,
}

impl VideoDecoder {
    /// Creates a new video decoder for the given path.
    /// # Errors
    /// Errors are raised if the underlying FFmpeg functions fail.
    pub fn new(path: &Path) -> Result<Self, RsmpegError> {
        let mut options: Option<AVDictionary> = None;
        let fmt_ctx = AVFormatContextInput::open(path_to_cstring(path).as_c_str(), None, &mut options)?;

        let (video_stream, video_decoder) = open_codec_ctx(&fmt_ctx, AVMediaType::VIDEO)?;

        Ok(VideoDecoder {
            fmt_ctx, video_stream, video_decoder
        })
    }

    pub fn next_frame(&mut self) -> Result<Option<AVFrame>, RsmpegError> {
        loop {
            match self.fmt_ctx.read_packet()? {
                // if there are no frames left, return None
                None => return Ok(None),
                Some(packet) => {
                    // if it's not video don't worry about it
                    if packet.stream_index as usize != self.video_stream {continue}
                    // decode the next frame
                    self.video_decoder.send_packet(Some(&packet))?;
                    match self.video_decoder.receive_frame() {
                        Ok(frame) => return Ok(Some(frame)),
                        Err(RsmpegError::DecoderDrainError) => (),
                        Err(error) => return Err(error)
                    }
                }
            }
        }
    }

    pub fn time_base(&self) -> AVRational {
        let stream = self.fmt_ctx.streams().get(self.video_stream).unwrap();
        stream.time_base.clone()
    }
}

fn path_to_cstring(path: &Path) -> CString {
    CString::new(path.to_str().unwrap()).unwrap()
}

fn open_codec_ctx(fmt_ctx: &AVFormatContextInput, media_type: AVMediaType) -> Result<(usize, AVCodecContext), RsmpegError> {
    let (index, _) = fmt_ctx.find_best_stream(media_type.into())?.ok_or(RsmpegError::AVError(AVERROR_STREAM_NOT_FOUND))?;

    let stream = fmt_ctx.streams().get(index).expect("STREAMS FUCKED");
    eprintln!("time base: {:?}", stream.time_base);
    let codec = AVCodec::find_decoder(stream.codecpar().codec_id).ok_or(RsmpegError::AVError(AVERROR_DECODER_NOT_FOUND))?;

    let mut codec_ctx = AVCodecContext::new(&codec);
    codec_ctx.apply_codecpar(&stream.codecpar())?;
    codec_ctx.open(None)?;

    Ok((index, codec_ctx))
}