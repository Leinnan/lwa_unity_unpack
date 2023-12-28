use regex::Regex;
use std::ffi::OsStr;
use std::fs;
use std::fs::DirEntry;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Asset {
    pub extension: Option<String>,
    pub guid: String,
    pub path: String,
    pub has_meta: bool,
    pub asset_type: AssetType,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AssetType {
    FbxModel,
    Material,
    Prefab,
    Scene,
    Other(String),
}

impl Asset {
    pub fn try_get_mat_texture_guid(&self) -> Option<String> {
        match &self.asset_type {
            AssetType::Material => {}
            _ => return None,
        }
        let file = File::open(&self.path).unwrap();
        let buf_reader = BufReader::new(file);
        let search = buf_reader.lines().into_iter().find(|s| {
            let ss = s.as_ref().unwrap();
            return ss.contains("m_Texture") && ss.contains("guid: ");
        });
        if let Some(line) = search {
            let line = line.unwrap_or_default();
            return extract_guid(&line);
        }
        None
    }

    pub fn prepare_directory(&self) {
        println!("{}: {:?}", self.guid, self.path);
        let base_path = Path::new(&self.path);
        let result_dir = base_path.parent();
        if result_dir.is_none() {
            eprintln!("{} is none", &self.path);
        }
        let result_dir = result_dir.unwrap();
        if !result_dir.exists() {
            let result = fs::create_dir_all(result_dir);
            if result.is_err() {
                eprintln!(
                    "Error {}: {}",
                    result_dir.to_str().unwrap(),
                    result.err().unwrap()
                );
            }
        }
    }

    pub fn from_path(entry: &DirEntry, output_dir: &PathBuf) -> Option<Asset> {
        let root_file = entry.path();
        if !root_file.is_dir() {
            return None;
        }
        let guid = entry.file_name().into_string().unwrap();
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
                            real_path = output_dir.join(path).to_str().unwrap().to_string();
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
            let asset_type = match &extension {
                Some(str) => match str.as_str() {
                    "fbx" => AssetType::FbxModel,
                    "prefab" => AssetType::Prefab,
                    "unity" => AssetType::Scene,
                    "mat" => AssetType::Material,
                    _ => AssetType::Other(str.clone()),
                },
                _ => AssetType::Other(String::new()),
            };
            Some(Asset {
                extension,
                guid,
                path: real_path,
                has_meta,
                asset_type,
            })
        } else {
            None
        }
    }
}

fn extract_guid(text: &str) -> Option<String> {
    let re = Regex::new(r"guid: (?P<guid>[A-Za-z0-9]{32})").unwrap();
    re.captures(text)
        .and_then(|cap| cap.name("guid").map(|guid| guid.as_str().to_string()))
}
