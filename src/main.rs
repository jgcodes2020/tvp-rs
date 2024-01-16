use std::{ffi::{CStr, CString}, fs, error::Error};

use clap::Parser;
use rsmpeg::avformat::AVFormatContextInput;
mod cli;

fn main() -> Result<(), Box<dyn Error>> {
    let args = cli::TvpCli::parse();
    let canon = fs::canonicalize(args.file)?;
    
    println!("path: {}", canon.display());
    
    Ok(())
}
