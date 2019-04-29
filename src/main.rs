#[macro_use]
extern crate serde_derive;

mod config;
mod transformer;

use clap::{App, Arg};
use config::Config;
use transformer::Transformer;

fn main() {
    let matches = App::new("csv-transform")
        .version("0.1.0")
        .author("Robert den Harink <robert@robhar.com")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .value_name("FILE")
            .help("file to load")
            .takes_value(true))
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("FILE")
            .help("output file")
            .takes_value(true))
        .get_matches();

    let config = match matches.value_of("config") {
        Some(file_path) => Config::from_file(file_path),
        None => Config::new(),
    };

    let reader: Box<dyn std::io::Read> = match matches.value_of("file") {
        Some(path) => Box::new(std::fs::File::open(path).expect("cannot read file")),
        None => Box::new(std::io::stdin()),
    };

    let writer: Box<dyn std::io::Write> = match matches.value_of("output") {
        Some(path) => Box::new(std::fs::File::open(path).expect("cannot open file for writing")),
        None => Box::new(std::io::stdout()),
    };

    let mut transformer = Transformer::new(config, reader, writer);
    transformer.transform();
}