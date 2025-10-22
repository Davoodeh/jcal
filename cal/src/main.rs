use jcal::clap_helper::Parse;

use crate::arg_parser::{Args, ColorMode};

mod arg_parser;
mod layout;
mod string;

fn main() {
    let config = Args::parse();

    match config.color {
        ColorMode::Always => colored::control::set_override(true),
        ColorMode::Never => colored::control::set_override(false),
        ColorMode::Auto => colored::control::unset_override(),
    }

    // TODO fix this, get an iterator and print each line
    config.layout.print()
}
