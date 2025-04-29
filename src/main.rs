use anyhow::{Context, anyhow};
use std::{
    fs::{DirEntry, File, ReadDir, read_dir},
    io::{self, Read},
    path::{Path, PathBuf},
    str::{self, FromStr},
};

const ZONEINFO_DIR: &str = "/usr/share/zoneinfo";

fn main() {
    walk_dir(&PathBuf::from_str(ZONEINFO_DIR).expect("ZONEINFO_DIR to be valid PathBuf"));
}

fn walk_dir(dir_path: &Path) {
    let dir = read_dir(dir_path).expect("zoneinfo dir can be read");
    let mut tzs: Vec<()> = Vec::new();

    for entry in dir {
        match entry {
            Ok(entry) if entry.path().is_dir() => walk_dir(&entry.path()),
            Ok(entry) => {
                read_tzinfo(&entry.path());
            }
            Err(err) => eprintln!("An error occurred while iterating `{dir_path:?}`: {err}"),
        }
    }
}

fn read_tzinfo(file: &Path) -> anyhow::Result<()> {
    let content_bytes = std::fs::read(file)
        .with_context(|| anyhow!("Tried to read file {file:?}"))?
        .into_iter();

    let mut header = [0; 44];
    for (idx, byte) in content_bytes.take(44).enumerate() {
        header[idx] = byte;
    }

    let header = TzFileHeader::from(header);
    println!("TzFile version: {}", header.get_version());
    // println!("Filetype: {header}", header = str::from_utf8(&header[..])?);
    Ok(())
}

#[derive(Debug)]
struct TzFileHeader {
    magic: [u8; 4],
    version: [u8; 1],
    reserved: [u8; 15],

    tzh_ttisutcnt: [u8; 4],
    tzh_ttisstdcnt: [u8; 4],
    tzh_leapcnt: [u8; 4],
    tzh_timecnt: [u8; 4],
    tzh_typecnt: [u8; 4],
    tzh_charcnt: [u8; 4],
}

impl From<[u8; 44]> for TzFileHeader {
    fn from(value: [u8; 44]) -> Self {
        Self {
            magic: [value[0], value[1], value[2], value[3]],
            version: [value[4]],
            reserved: [
                value[5], value[6], value[7], value[8], value[9], value[10], value[11], value[12],
                value[13], value[14], value[15], value[16], value[17], value[18], value[19],
            ],

            tzh_ttisutcnt: [value[20], value[21], value[22], value[23]],
            tzh_ttisstdcnt: [value[24], value[25], value[26], value[27]],
            tzh_leapcnt: [value[28], value[29], value[30], value[31]],
            tzh_timecnt: [value[32], value[33], value[34], value[35]],
            tzh_typecnt: [value[36], value[37], value[38], value[39]],
            tzh_charcnt: [value[40], value[41], value[42], value[43]],
        }
    }
}

impl TzFileHeader {
    fn get_version(self) -> String {
        if self.version[0] == b'\0' {
            return String::from("NULL");
        }
        String::from_utf8_lossy(&self.version).to_string()
    }
}
