
use std::{path::PathBuf, fs, io};

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct TvpCli {
    #[arg(value_parser = existing_path, help = "The file to play")]
    pub file: PathBuf,
}

fn existing_path(arg: &str) -> Result<PathBuf, io::Error> {
    let path = PathBuf::from(arg);
    // just check if the file is accessible
    fs::metadata(&path).map(|_| ())?;
    Ok(path)
}