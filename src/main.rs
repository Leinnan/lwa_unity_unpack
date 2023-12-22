use std::fs::File;
use std::{fs, io};
use std::path::Path;
use flate2::read::GzDecoder;
use tar::Archive;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::prelude::*;
use std::io::BufReader;
use std::process::Command;

pub fn extract_archive(archive_path: &Path, extract_to: &Path) -> io::Result<()> {
    let tar_gz = File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extract_to)?;
    Ok(())
}

fn main() {
    let archive_path = Path::new("C:\\PROJECTS\\lwa_unity_unpack\\POLYGON_Snow_Kit_Unity_2020_3_v1_4.unitypackage");
    let tmp_dir = Path::new("./tmp_dir");
    let output_dir = Path::new("./output");
    if tmp_dir.exists() {
        println!("Temp directory exits, cleaning up first.");
        fs::remove_dir_all(tmp_dir).unwrap();
    }
    if let Err(e) = extract_archive(archive_path,tmp_dir) {
        println!("Failed to extract archive: {}", e);
    }
    if output_dir.exists() {
        println!("Output directory exits, cleaning up first.");
        fs::remove_dir_all(output_dir).unwrap();
    }
    fs::create_dir(output_dir).unwrap();
    let mut mapping: HashMap<String, String> = HashMap::new();

    for entry in fs::read_dir(tmp_dir).unwrap() {
        let entry = entry.unwrap();
        let root_file = entry.path();
        let asset = entry.file_name().into_string().unwrap();

        if root_file.is_dir() {
            let mut real_path = String::new();
            let mut has_asset = false;

            for sub_entry in fs::read_dir(root_file.clone()).unwrap() {
                let sub_entry = sub_entry.unwrap();
                let file_name = sub_entry.file_name().into_string().unwrap();

                if file_name == "pathname" {
                    let path = sub_entry.path();
                    let file = File::open(path).unwrap();
                    let mut buf_reader = BufReader::new(file);
                    let line = buf_reader.lines().next();

                    match line {
                        Some(Ok(path)) => real_path = path,
                        _ => continue,
                    }
                } else if file_name == "asset" {
                    has_asset = true;
                }
            }

            if has_asset {
                mapping.insert(asset, real_path);
            }
        }
    }

    println!("Results:");
    for (asset_hash, asset_path) in &mapping {
        println!("{}: {}", asset_hash, asset_path);
        let path = Path::new(asset_path);
        let source_asset = Path::new(tmp_dir).join(asset_hash).join("asset");
        let result_path = output_dir.join(path);
        let result_dir = result_path.parent().unwrap();
        if !result_dir.exists() {
            fs::create_dir_all(result_dir).unwrap();
        }

        if !source_asset.exists() {
            panic!("SOURCE ASSET DOES NOT EXIST: {}", source_asset.display());
        }
        if let Some("fbx") = path.extension().and_then(OsStr::to_str) {
            let out_path = result_path.with_extension("");
            println!("{:?}",&["--input", source_asset.to_str().unwrap(),"--output",out_path.to_str().unwrap()]);
            let output = Command::new("C:\\tools\\FBX2glTF.exe")
                .args(&["--input", source_asset.to_str().unwrap(),"-b","--output",out_path.to_str().unwrap()])
                .output().unwrap();

            let output_result = String::from_utf8_lossy(&output.stdout);

            println!("output: {}", output_result);
            continue;
        }
        fs::rename(source_asset,result_path).unwrap();
    }

    //fs::remove_dir_all(tmp_dir).unwrap();
}


