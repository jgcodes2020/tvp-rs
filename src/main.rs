use std::{error::Error, fs, path::Path};

use clap::Parser;
use media::VideoDecoder;

mod cli;
mod enums;
mod media;
mod raster;

fn main() -> Result<(), Box<dyn Error>> {
    let args = cli::TvpCli::parse();
    let canon = fs::canonicalize(args.file)?;
    
    println!("path: {}", canon.display());
    
    decode_test(&canon)?;
    
    Ok(())
}

fn decode_test(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut decoder = VideoDecoder::new(path)?;
    
    while let Some(frame) = decoder.next_frame()? {
        
    }


    Ok(())
}
