use clap::{App, Arg};
use log::LevelFilter;
use std::process;
use simple_logger::SimpleLogger;

pub fn main () {
    let arg_matches = App::new("candump")
                    .version("0.0.1")
                    .author("Raphael Nissl")
                    .about("Display defined/all frames of a specified socket")
                    .arg(
                        Arg::with_name("socket")
                            .help("Name of CAN socket")
                            .index(1)
                            .required(true),
                    )
                    .arg(
                        Arg::with_name("reception_count")
                        .help("terminate after receiving 'n' frames")
                        .takes_value(true)
                        .required(false),
                    )
                    .arg(
                        Arg::with_name("extra")
                            .help("Prints extra info")
                            .short("x")
                            .takes_value(false)
                            .required(false),
                    )
                    .get_matches();

    SimpleLogger::new()
                    .with_level(LevelFilter::Info)
                    .init()
                    .unwrap();

    log::warn!("Not supported yet");

    process::exit(0);
}