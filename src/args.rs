use clap::Parser;
use std::path::PathBuf;

/// Program for unpacking unitypackages files.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// .unitypackage file to extract
    #[arg(short, long)]
    pub input: PathBuf,
    /// target directory
    #[arg(short, long)]
    pub output: PathBuf,

    /// optional- path to the tool that will auto convert fbx files to gltf during unpacking
    #[arg(long)]
    pub fbx_to_gltf: Option<PathBuf>,

    /// checks if material base texture in prefabs differ from the one specified in fbx model
    /// that is converted to GLTF and overrides it with the one from prefab and copy texture to models folder
    #[arg(long, default_value = "false", default_missing_value = "true")]
    pub get_materials_from_prefabs: bool,

    /// optional- extensions that will be ignored during unpacking
    #[arg(long, action = clap::ArgAction::Append)]
    pub ignore_extensions: Option<Vec<String>>,

    /// copy meta files alongside regular files
    #[arg(long, default_value = "false", default_missing_value = "true")]
    pub copy_meta_files: bool,
}

impl Args {
    pub fn check(&self) {
        if let Some(path) = &self.fbx_to_gltf {
            assert!(
                is_executable(&path),
                "fbx_to_gltf require a path to executable"
            )
        }
    }
}

#[cfg(target_os = "windows")]
fn is_executable(path: &PathBuf) -> bool {
    if let Some(extension) = path.extension() {
        extension.to_str().unwrap().ends_with("exe")
    } else {
        false
    }
}

#[cfg(not(target_os = "windows"))]
fn is_executable(path: &PathBuf) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let permissions = metadata.permissions();
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}
