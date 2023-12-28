use crate::asset::{Asset, AssetType};
use flate2::read::GzDecoder;
use rayon::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::{fs, io};
use tar::Archive;

#[derive(Clone)]
pub struct Unpacker {
    pub args: crate::args::Args,
    pub assets: Vec<Asset>,
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
    pub fn extract(&mut self) {
        let archive_path = Path::new(&self.args.input);
        let tmp_path = Path::new("./tmp_dir");
        let output_dir = Path::new(&self.args.output);

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
                let asset = crate::asset::Asset::from_path(&entry, &output_dir.to_path_buf());
                if let Some(asset) = asset {
                    let extension = &asset.extension.clone().unwrap_or_default();
                    if !ignored_extensions.contains(extension) {
                        s.send(asset).unwrap();
                    }
                }
            });
        self.assets = receiver.iter().collect();
    }

    pub fn update_gltf_materials(&self) {
        if self.args.fbx_to_gltf.is_none() {
            return;
        }
        let fbx_models: Vec<Asset> = self
            .assets
            .clone()
            .into_iter()
            .filter(|a| &a.asset_type == &AssetType::FbxModel)
            .collect();
        let prefabs: Vec<Asset> = self
            .assets
            .clone()
            .into_iter()
            .filter(|a| &a.asset_type == &AssetType::Prefab)
            .collect();
        let materials: Vec<Asset> = self
            .assets
            .clone()
            .into_iter()
            .filter(|a| &a.asset_type == &AssetType::Material)
            .collect();
        println!(
            "There are {} models, {} prefabs and {} materials",
            fbx_models.len(),
            prefabs.len(),
            materials.len()
        );
        let mut counter = 0;
        let mut result_models : Vec<Asset> = vec![];
        for prefab in prefabs.iter() {
            let path = Path::new(&prefab.path);
            let prefab_content = fs::read_to_string(&path).unwrap();
            let matching_materials: Vec<Asset> = materials
                .clone()
                .into_iter()
                .filter(|a| prefab_content.contains(&a.guid))
                .collect();
            let matching_models: Vec<Asset> = fbx_models
                .clone()
                .into_iter()
                .filter(|a| prefab_content.contains(&a.guid))
                .collect();
            if matching_materials.len() != 1 || 1 != matching_models.len() {
                continue;
            }
            let material = matching_materials.first().unwrap();
            let model = matching_models.first().unwrap();
            if result_models.iter().any(|a| model.guid.eq(&a.guid)) {
                continue;
            }
            let texture_guid: Option<String> = material.try_get_mat_texture_guid();
            let texture_asset: &Asset;
            match &texture_guid {
                Some(guid) => {
                    texture_asset = self.assets.iter().find(|a| guid.eq(&a.guid)).unwrap()
                }
                None => continue,
            }
            // here we should read gltf file and replace material texture with Uri based on texture_asset

            result_models.push(model.clone());
            counter += 1;
        }
        println!("Updated {} models", counter);
    }

    pub fn process_data(&self) {
        let output_dir = Path::new(&self.args.output);
        let copy_meta_files = self.args.copy_meta_files;
        let tmp_path = Path::new("./tmp_dir");

        let mapping_arc = Arc::new(&self.assets);
        let tmp_dir = Arc::new(tmp_path);
        fs::create_dir(output_dir).unwrap();

        mapping_arc.par_iter().for_each(|asset| {
            let asset_hash = &asset.guid;
            let path = Path::new(&asset.path);
            let source_asset = Path::new(&*tmp_dir).join(asset_hash).join("asset");

            asset.prepare_directory();
            if copy_meta_files && asset.has_meta {
                let source_meta = Path::new(&*tmp_dir).join(asset_hash).join("asset.meta");
                let mut meta_path = asset.path.clone();
                meta_path.push_str(".meta");
                fs::rename(source_meta, meta_path).unwrap();
            }

            if !source_asset.exists() {
                panic!("SOURCE ASSET DOES NOT EXIST: {}", source_asset.display());
            }

            if self.args.fbx_to_gltf.is_some() && &asset.asset_type == &AssetType::FbxModel {
                process_fbx_file(
                    &source_asset,
                    &path,
                    &self.args.fbx_to_gltf.clone().unwrap(),
                );
            } else {
                process_non_fbx_file(&source_asset, &path);
            }
        });

        fs::remove_dir_all(Path::new(&*tmp_dir)).unwrap();

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
