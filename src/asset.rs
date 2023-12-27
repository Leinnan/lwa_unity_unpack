use std::ffi::OsStr;
use std::fs;
use std::fs::DirEntry;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Asset {
    pub extension: Option<String>,
    pub hash: String,
    pub path_name: String,
    pub has_meta: bool,
}

impl Asset {
    pub fn from_path(entry: &DirEntry) -> Option<Asset> {
        let root_file = entry.path();
        if !root_file.is_dir() {
            return None;
        }
        let asset = entry.file_name().into_string().unwrap();
        let mut real_path = String::new();
        let mut extension = None;
        let mut has_asset = false;
        let mut has_meta = false;
        for sub_entry in fs::read_dir(root_file.clone()).unwrap() {
            let sub_entry = sub_entry.unwrap();
            let file_name = sub_entry.file_name().into_string().unwrap();
            match file_name.as_str() {
                "pathname" => {
                    let path = sub_entry.path();
                    let file = File::open(path).unwrap();
                    let buf_reader = BufReader::new(file);
                    let line = buf_reader.lines().next();
                    match line {
                        Some(Ok(path)) => {
                            real_path = path;
                            if let Some(e) =
                                Path::new(&real_path).extension().and_then(OsStr::to_str)
                            {
                                extension = Some(String::from(e));
                            }
                        }
                        _ => continue,
                    }
                }
                "asset" => has_asset = true,
                "asset.meta" => has_meta = true,
                _ => continue,
            }
        }
        if has_asset {
            Some(Asset {
                extension,
                hash: asset,
                path_name: real_path,
                has_meta,
            })
        } else {
            None
        }
    }
}
