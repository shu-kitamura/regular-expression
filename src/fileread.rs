//! ファイルを読み込むための関数

use std::{
    fs::File,
    io::BufReader
};

use crate::error::FileError;

pub fn open_file(path: &str) -> Result<BufReader<File>, FileError> {
    match File::open(path) {
        Ok(file) => Ok(BufReader::new(file)),
        Err(e) => return Err(FileError::FailedOpen(e.to_string()))
    }
}