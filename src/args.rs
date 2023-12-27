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
    #[arg(short, long)]
    pub fbx_to_gltf: Option<PathBuf>,

    /// optional- extensions that will be ignored during unpacking
    #[arg(long, action = clap::ArgAction::Append)]
    pub ignore_extensions: Option<Vec<String>>,

    /// copy meta files alongside regular files
    #[arg(long, default_value = "false", default_missing_value = "true")]
    pub copy_meta_files: bool,
}
