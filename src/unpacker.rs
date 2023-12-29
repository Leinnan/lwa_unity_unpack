use crate::asset::{Asset, AssetType};
use flate2::read::GzDecoder;
use gltf::{json, Document};
use rayon::prelude::*;
use std::borrow::Cow;

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

fn get_relative_path(path1: &Path, path2: &Path) -> Option<PathBuf> {
    pathdiff::diff_paths(path1, path2)
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
                let asset = crate::asset::Asset::from_path(&entry, output_dir);
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
            .filter(|a| a.asset_type == AssetType::FbxModel)
            .collect();
        let prefabs: Vec<Asset> = self
            .assets
            .clone()
            .into_iter()
            .filter(|a| a.asset_type == AssetType::Prefab)
            .collect();
        let materials: Vec<Asset> = self
            .assets
            .clone()
            .into_iter()
            .filter(|a| a.asset_type == AssetType::Material)
            .collect();
        println!(
            "There are {} models, {} prefabs and {} materials",
            fbx_models.len(),
            prefabs.len(),
            materials.len()
        );

        prefabs.par_iter().for_each(|prefab| {
            let path = Path::new(&prefab.path);
            let prefab_content = fs::read_to_string(path).unwrap();
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
                return;
            }
            let material = matching_materials.first().unwrap();
            let model: &Asset = matching_models.first().unwrap();
            let texture_guid: Option<String> = material.try_get_mat_texture_guid();

            let texture_asset: &Asset = match &texture_guid {
                Some(guid) => self.assets.iter().find(|a| guid.eq(&a.guid)).unwrap(),
                None => return,
            };
            // here we should read gltf file and replace material texture with Uri based on texture_asset
            let model_path = Path::new(&model.path).with_extension("glb");
            Self::modify_material(&model_path, Path::new(&texture_asset.path));
        });
    }

    fn align_to_multiple_of_four(n: &mut usize) {
        *n = (*n + 3) & !3;
    }

    fn modify_material(gltf_path: &Path, texture_asset: &Path) {
        let file = fs::File::open(gltf_path).unwrap();
        let reader = io::BufReader::new(file);
        let mut gltf = gltf::Gltf::from_reader(reader).unwrap();
        let mut json = gltf.document.into_json();
        if let Some(rel_path) = get_relative_path(texture_asset, gltf_path) {
            for image in json.images.iter_mut() {
                let result = rel_path.file_name().unwrap().to_str().unwrap().to_string();
                let required_file = gltf_path.with_file_name(&result);
                if !required_file.exists() {
                    fs::copy(texture_asset, gltf_path.with_file_name(&result)).unwrap();
                }
                println!(
                    "Image{:?}: {:?} to be replaced with: {}",
                    image.name, image.uri, &result
                );
                image.uri = Some(result);
            }
        }

        gltf.document = Document::from_json(json.clone()).unwrap();
        // Save the modified glTF
        let json_string = json::serialize::to_string(&json).expect("Serialization error");
        let mut json_offset = json_string.len();
        Self::align_to_multiple_of_four(&mut json_offset);
        let blob = gltf.blob.clone().unwrap_or_default();
        let buffer_length = blob.len();
        let glb = gltf::binary::Glb {
            header: gltf::binary::Header {
                magic: *b"glTF",
                version: 2,
                // N.B., the size of binary glTF file is limited to range of `u32`.
                length: (json_offset + buffer_length)
                    .try_into()
                    .expect("file size exceeds binary glTF limit"),
            },
            bin: Some(Cow::Owned(gltf.blob.unwrap_or_default())),
            json: Cow::Owned(json_string.into_bytes()),
        };
        let writer = std::fs::File::create(gltf_path).expect("I/O error");
        glb.to_writer(writer).expect("glTF binary output error");
    }

    pub fn process_data(&self) {
        let output_dir = Path::new(&self.args.output);
        let copy_meta_files = self.args.copy_meta_files;
        let tmp_path = Path::new("./tmp_dir");

        let tmp_dir = Arc::new(tmp_path);
        fs::create_dir(output_dir).unwrap();

        self.assets.par_iter().for_each(|asset| {
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

            if self.args.fbx_to_gltf.is_some() && asset.asset_type == AssetType::FbxModel {
                self.process_fbx_file(&source_asset, path);
            } else {
                fs::rename(source_asset, path).unwrap();
            }
        });

        fs::remove_dir_all(Path::new(&*tmp_dir)).unwrap();
    }

    fn process_fbx_file(&self, source_asset: &Path, result_path: &Path) {
        let tool = self.args.fbx_to_gltf.clone().unwrap();
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

    fn extract_archive(archive_path: &Path, extract_to: &Path) -> io::Result<()> {
        let tar_gz = File::open(archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(extract_to)?;
        Ok(())
    }
}
