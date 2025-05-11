use anyhow::{Context, anyhow};
use entities::TzFile;
use std::{
    collections::HashMap,
    env::args,
    fs::read_dir,
    path::{Path, PathBuf},
    str::FromStr,
};

mod entities;

fn main() {
    let mut zoneinfo =
        PathBuf::from_str("/usr/share/zoneinfo").expect("/usr/share/zoneinfo to be valid PathBuf");
    let locale = args().nth(1).unwrap_or("/".to_owned());
    if locale == "/" {
        walk_dir(&zoneinfo);
        return;
    }

    zoneinfo.push(locale);
    match parse_tzif(&zoneinfo) {
        Ok(tzif) => {
            println!("{:#?}", tzif.body);
            println!("----------------------------------------------------------------------");

            let mut table: HashMap<String, Timezone> = HashMap::new();
            for (idx, ttinfo_idx) in tzif.body.ttinfo_indices[..tzif.body.ttinfo_indices.len() - 1] // all but the last indice
                .iter()
                .enumerate()
            {
                let trans = tzif.body.tt_trans[idx];
                let ttinfo = &(tzif.body.ttinfo_entries)[*ttinfo_idx as usize];
                let tz_name = tzif
                    .body
                    .tz_designations
                    .get(ttinfo.tt_desigidx as usize..)
                    .unwrap()
                    .split_once('\0')
                    .unwrap()
                    .0
                    .to_string();
                if let Some(tz) = table.get_mut(&tz_name) {
                    tz.transitions.push(trans);
                } else {
                    table.insert(
                        tz_name.clone(),
                        Timezone {
                            name: tz_name,
                            ut_offset: ttinfo.tt_utoff,
                            is_daylight_savings: ttinfo.tt_isdst,
                            transitions: vec![],
                        },
                    );
                }
            }
            println!("{:#?}", table)
        }
        Err(err) => {
            eprintln!("Error: {}: {}", err, err.root_cause());
        }
    };
}

fn walk_dir(dir_path: &Path) {
    let dir = read_dir(dir_path).expect("zoneinfo dir can be read");
    let mut tzs: Vec<TzFile> = Vec::new();

    for entry in dir {
        match entry {
            Ok(entry) if entry.path().is_dir() => walk_dir(&entry.path()),
            Ok(entry) => {
                let tzif = match parse_tzif(&entry.path()) {
                    Ok(tzif) => tzif,
                    Err(err) => {
                        eprintln!("Error: {}: {}", err, err.root_cause());
                        continue;
                    }
                };

                tzs.push(tzif);
            }
            Err(err) => eprintln!("An error occurred while iterating `{dir_path:?}`: {err}"),
        }
    }

    for tz in tzs {
        println!("{:#?}", tz.body);
    }
}

fn parse_tzif(file: &Path) -> anyhow::Result<TzFile> {
    let content_bytes =
        std::fs::read(file).with_context(|| anyhow!("tried to read file {file:?}"))?;

    match TzFile::from_bytes(&content_bytes) {
        t @ Ok(_) => t,
        Err(err) => Err(err.context(anyhow!("could not parse file '{}' as tzif", file.display()))),
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct Timezone {
    name: String,
    ut_offset: i32,
    is_daylight_savings: bool,
    transitions: Vec<i32>,
}
