mod args;
pub mod asset;
pub mod primitives;
mod unpacker;
mod yaml_helpers;
use clap::Parser;

fn main() {
    let args = crate::args::Args::parse();
    let mut unpacker = crate::unpacker::Unpacker {
        args,
        assets: vec![],
    };

    unpacker.prepare_environment();
    unpacker.extract();
    unpacker.process_data();
    unpacker.update_gltf_materials();
}
