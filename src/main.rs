use std::io::{Read, Write};
use std::thread::sleep;
use std::time::Duration;
use std::{error::Error, fs, io, path::Path};

use clap::Parser;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use media::VideoDecoder;
use raster::BWRaster;
use raster::Raster;

mod cli;
mod enums;
mod media;
mod raster;

struct TermSetup;
impl TermSetup {
    pub fn run() -> Result<TermSetup, Box<dyn Error>> {
        terminal::enable_raw_mode()?;
        io::stdout().write(b"\x1B[?47h\x1B[?80l")?;
        io::stdout().flush()?;
        Ok(TermSetup)
    }
}
impl Drop for TermSetup {
    fn drop(&mut self) {
        let _ = (|| -> Result<(), Box<dyn Error>> {
            terminal::disable_raw_mode()?;
            io::stdout().write(b"\x1B[?47l\x1B[?80h")?;
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
    let mut stdin = io::stdin().lock();
    while let Some(frame) = decoder.next_frame()? {
        stdout.write(b"\x1B[H\x1B[2J\x1B[3J")?;
        BWRaster::present(&frame, &mut stdout)?;
        stdout.flush()?;
        sleep(Duration::from_millis(250));
    }

    Ok(())
}
