use flate2::read::GzDecoder;
use hashbrown::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::{fs, io};

use rayon::prelude::*;
use std::sync::mpsc::channel;
use tar::Archive;

#[derive(Clone)]
pub struct Unpacker {
    pub args: crate::args::Args,
}

impl Unpacker {
    pub fn prepare_environment(&self) {
        let archive_path = Path::new(&self.args.input);
        let output_dir = Path::new(&self.args.output);
        let tmp_path = Path::new("./tmp_dir");
        if !archive_path.exists() {
            panic!("Input file does not exits");
        }
        if tmp_path.exists() {
            println!("Temp directory exits, cleaning up first.");
            fs::remove_dir_all(tmp_path).unwrap();
        }
        if output_dir.exists() {
            println!("Output directory exits, cleaning up first.");
            fs::remove_dir_all(output_dir).unwrap();
        }
    }

    pub fn process_data(&self) {
        let archive_path = Path::new(&self.args.input);
        let output_dir = Path::new(&self.args.output);
        let tmp_path = Path::new("./tmp_dir");
        if let Err(e) = Unpacker::extract_archive(archive_path, tmp_path) {
            println!("Failed to extract archive: {}", e);
        }

        let (sender, receiver) = channel();
        let ignored_extensions = self.args.clone().ignore_extensions.unwrap_or_default();

        fs::read_dir(tmp_path)
            .unwrap()
            .par_bridge()
            .for_each_with(sender, |s, entry| {
                let entry = entry.unwrap();
                let root_file = entry.path();
                let asset = entry.file_name().into_string().unwrap();
                if !root_file.is_dir() {
                    return;
                }
                let mut real_path = String::new();
                let mut extension = None;
                let mut has_asset = false;
                for sub_entry in fs::read_dir(root_file.clone()).unwrap() {
                    let sub_entry = sub_entry.unwrap();
                    let file_name = sub_entry.file_name().into_string().unwrap();
                    if file_name == "pathname" {
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
                    } else if file_name == "asset" {
                        has_asset = true;
                    }
                }
                if has_asset && !ignored_extensions.contains(&extension.unwrap_or_default()) {
                    s.send((asset, real_path)).unwrap();
                }
            });

        let tmp_dir = Arc::new(tmp_path);
        fs::create_dir(output_dir).unwrap();
        let output_dir = Arc::new(output_dir);
        let mapping: HashMap<String, String> = receiver.iter().collect();
        let mapping_arc = Arc::new(mapping);

        mapping_arc.par_iter().for_each(|(asset_hash, asset_path)| {
            let path = Path::new(asset_path);
            let source_asset = Path::new(&*tmp_dir).join(asset_hash).join("asset");
            let result_path = output_dir.join(path);

            process_directory(asset_hash, asset_path, &result_path);
            check_source_asset_exists(&source_asset);

            if self.args.fbx_to_gltf.is_some() {
                if let Some("fbx") = path.extension().and_then(OsStr::to_str) {
                    process_fbx_file(
                        &source_asset,
                        &result_path,
                        &self.args.fbx_to_gltf.clone().unwrap(),
                    );
                    return;
                }
            }

            process_non_fbx_file(&source_asset, &result_path);
        });

        fs::remove_dir_all(Path::new(&*tmp_dir)).unwrap();

        fn process_directory(asset_hash: &str, asset_path: &str, result_path: &Path) {
            println!("{}: {:?}", asset_hash, asset_path);
            let result_dir = result_path.parent().unwrap();
            if !result_dir.exists() {
                fs::create_dir_all(result_dir).unwrap();
            }
        }

        fn check_source_asset_exists(source_asset: &Path) {
            if !source_asset.exists() {
                panic!("SOURCE ASSET DOES NOT EXIST: {}", source_asset.display());
            }
        }

        fn process_fbx_file(source_asset: &Path, result_path: &Path, tool: &PathBuf) {
            let out_path = result_path.with_extension("");
            println!(
                "{:?}",
                &[
                    "--input",
                    source_asset.to_str().unwrap(),
                    "--output",
                    out_path.to_str().unwrap()
                ]
            );
            let output = Command::new(tool)
                .args([
                    "--input",
                    source_asset.to_str().unwrap(),
                    "-b",
                    "--output",
                    out_path.to_str().unwrap(),
                ])
                .output()
                .unwrap();
            let output_result = String::from_utf8_lossy(&output.stdout);
            println!("output: {}", output_result);
        }

        fn process_non_fbx_file(source_asset: &Path, result_path: &Path) {
            fs::rename(source_asset, result_path).unwrap();
        }
    }

    fn extract_archive(archive_path: &Path, extract_to: &Path) -> io::Result<()> {
        let tar_gz = File::open(archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(extract_to)?;
        Ok(())
    }
}
