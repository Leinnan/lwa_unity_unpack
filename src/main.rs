mod args;
mod unpacker;

use clap::Parser;








fn main() {
    let args = crate::args::Args::parse();
    let unpacker = crate::unpacker::Unpacker { args };

    unpacker.prepare_environment();
    unpacker.process_data();
}
