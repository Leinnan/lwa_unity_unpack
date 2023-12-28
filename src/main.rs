mod args;
pub mod asset;
mod unpacker;

use clap::Parser;

fn main() {
    let args = crate::args::Args::parse();
    let mut unpacker = crate::unpacker::Unpacker { args, assets: vec![] };

    unpacker.prepare_environment();
    unpacker.extract();
    unpacker.process_data();
}
