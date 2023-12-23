use std::fs::File;
use std::{fs, io, sync::Arc};
use std::path::{Path, PathBuf};
use flate2::read::GzDecoder;
use tar::Archive;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::hash::Hash;
use std::io::prelude::*;
use std::io::BufReader;
use std::process::Command;
use rayon::prelude::*;
use clap::Parser;

/// Program for unpacking unitypackages files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// .unitypackage file to extract
    #[arg(short, long)]
    input: PathBuf,
    /// target directory
    #[arg(short, long)]
    output: PathBuf,

    /// optional- path to the tool that will auto convert fbx files to gltf during unpacking
    #[arg(short,long)]
    fbx_to_gltf: Option<PathBuf>
}

pub fn extract_archive(archive_path: &Path, extract_to: &Path) -> io::Result<()> {
    let tar_gz = File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extract_to)?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    let archive_path = Path::new(&args.input);
    let tmp_dir = Path::new("./tmp_dir");
    let output_dir = Path::new(&args.output);
    if !archive_path.exists() {
        panic!("Input file does not exits");
    }
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
    let mut mapping_arc = Arc::new(mapping);
    let tmp_dir = Arc::new(tmp_dir.clone());
    let output_dir = Arc::new(output_dir.clone());

    mapping_arc.par_iter().for_each(|(asset_hash, asset_path)| {
        let path = Path::new(asset_path);
        let source_asset = Path::new(&*tmp_dir).join(asset_hash).join("asset");
        let result_path = output_dir.join(&path);

        process_directory(asset_hash, asset_path, &result_path);
        check_source_asset_exists(&source_asset);

        if args.fbx_to_gltf.is_some() {
            if let Some("fbx") = path.extension().and_then(OsStr::to_str) {
                process_fbx_file(&source_asset, &result_path, &args.fbx_to_gltf.clone().unwrap());
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
        println!("{:?}", &["--input", source_asset.to_str().unwrap(), "--output", out_path.to_str().unwrap()]);
        let output = Command::new(tool)
            .args(&["--input", source_asset.to_str().unwrap(), "-b", "--output", out_path.to_str().unwrap()])
            .output().unwrap();
        let output_result = String::from_utf8_lossy(&output.stdout);
        println!("output: {}", output_result);
    }

    fn process_non_fbx_file(source_asset: &Path, result_path: &Path) {
        fs::rename(source_asset, result_path).unwrap();
    }
}