use std::ffi::c_int;

use rusty_ffmpeg::ffi::*;


#[derive(PartialEq)]
#[derive(Clone, Copy)]
pub struct AVMediaType(c_int);

impl AVMediaType {
    pub const UNKNOWN: AVMediaType = AVMediaType { 0: AVMediaType_AVMEDIA_TYPE_UNKNOWN };
    pub const VIDEO: AVMediaType = AVMediaType {0: AVMediaType_AVMEDIA_TYPE_VIDEO};
    pub const AUDIO: AVMediaType = AVMediaType {0: AVMediaType_AVMEDIA_TYPE_AUDIO};
    pub const DATA: AVMediaType = AVMediaType {0: AVMediaType_AVMEDIA_TYPE_DATA};
    pub const SUBTITLE: AVMediaType = AVMediaType {0: AVMediaType_AVMEDIA_TYPE_SUBTITLE};
    pub const ATTACHMENT: AVMediaType = AVMediaType {0: AVMediaType_AVMEDIA_TYPE_ATTACHMENT};
}

impl Into<i32> for AVMediaType {
    fn into(self) -> i32 {
        self.0
    }
}