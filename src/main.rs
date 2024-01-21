
use std::io::{IsTerminal, Write};


use std::thread::{sleep};
use std::time::{Duration, Instant};
use std::{error::Error, fs, io, path::Path};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{self};

use media::VideoDecoder;
use raster::BWRaster;
use raster::Raster;
use rsmpeg::avutil::AVFrame;
use rusty_ffmpeg::ffi::AVRational;





mod cli;
mod media;
mod raster;

struct TermSetup;
impl TermSetup {
    pub fn run() -> Result<TermSetup, Box<dyn Error>> {

        let mut stdout = io::stdout().lock();

        if stdout.is_terminal() {
            terminal::enable_raw_mode()?;

        }
        stdout.write(b"\x1B[?47h\x1B[?80h\x1B[?25l\x1B[2J\x1B[40m")?;
        stdout.flush()?;
        Ok(TermSetup)
    }
}
impl Drop for TermSetup {
    fn drop(&mut self) {
        let _ = (|| -> Result<(), Box<dyn Error>> {
            terminal::disable_raw_mode()?;
            io::stdout().write(b"\x1B[?47l\x1B[?80l\x1B[?25h\x1B[0m")?;
            io::stdout().flush()?;
            Ok(())
        })();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = cli::TvpCli::parse();
    let canon = fs::canonicalize(args.file)?;

    println!("path: {}", canon.display());

    {
        let _term_settings = TermSetup::run()?;
        decode_test(&canon)?;
    }

    Ok(())
}

fn frame_pts(frame: &AVFrame, time_base: AVRational) -> Duration {
    let timestamp: u128 = ((frame.pts as u128) * (time_base.num as u128) * 1_000_000_000u128) / (time_base.den as u128);
    Duration::from_nanos(timestamp as u64)
}

fn decode_test(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut decoder = VideoDecoder::new(path)?;
    let mut stdout = io::stdout().lock();
    let time_base = decoder.time_base();

    let start_time = Instant::now();
    'main_loop:
    while let Some(frame) = decoder.next_frame()? {
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(event) => {
                    if event.code == KeyCode::Esc {
                        break 'main_loop;
                    }
                }
                _ => ()
            }
        }
        // try to stay ahead of the PTS clock
        let pts = frame_pts(&frame, time_base);
        let clock = start_time.elapsed();
        if clock <= pts {
            sleep(pts - clock);
            BWRaster::present(&frame, &mut stdout)?;
        }
    }

    Ok(())
}
