
use std::io::{IsTerminal, Write};
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{error::Error, fs, io, path::Path};

use clap::Parser;
use crossterm::terminal::{self};

use media::VideoDecoder;
use raster::BWRaster;
use raster::Raster;



mod cli;
mod media;
mod raster;

struct TermSetup;
impl TermSetup {
    pub fn run() -> Result<TermSetup, Box<dyn Error>> {
        if io::stdout().is_terminal() {
            terminal::enable_raw_mode()?;
        }
        io::stdout().write(b"\x1B[?47h\x1B[?80h")?;
        io::stdout().flush()?;
        Ok(TermSetup)
    }
}
impl Drop for TermSetup {
    fn drop(&mut self) {
        let _ = (|| -> Result<(), Box<dyn Error>> {
            if io::stdout().is_terminal() {
                terminal::disable_raw_mode()?;
            }
            io::stdout().write(b"\x1B[?47l\x1B[?80l")?;
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
fn decode_test(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut decoder = VideoDecoder::new(path)?;
    let mut stdout = io::stdout().lock();
    let time_base = decoder.time_base();

    let start_time = Instant::now();
    while let Some(frame) = decoder.next_frame()? {
        // check the time, if it's too early, sleep
        let pts = Duration::from_nanos(((frame.pts as u64) * (time_base.num as u64) * 1_000_000_000u64) / (time_base.den as u64));
        let clock = start_time.elapsed();
        if clock < pts {
            sleep(pts - clock);
        }

        BWRaster::present(&frame, &mut stdout)?;
    }

    Ok(())
}
