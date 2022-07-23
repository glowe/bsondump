use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Write};
use std::result::Result;

use clap::{ArgEnum, Parser};

use chrono::{offset::Local, DateTime, TimeZone};

use crate::bsondump::BsonDump;

mod bsondump;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
#[clap(rename_all = "camelCase")]
enum OutputType {
    Debug,
    Json,
    PrettyJson,
}

fn print_num_found<Tz>(start: DateTime<Tz>, num_found: u32)
where
    Tz: TimeZone,
    <Tz as TimeZone>::Offset: Display,
{
    eprintln!(
        "{start}    {num_found} objects found",
        start = start.format("%+"),
        num_found = num_found
    );
}

#[derive(Parser)]
#[clap(rename_all = "camelCase")]
struct Args {
    #[clap(value_parser)]
    /// path to BSON file to dump to JSON; default is stdin
    file: Option<String>,

    #[clap(name="type", long="type", arg_enum, value_parser, default_value_t = OutputType::Json)]
    output_type: OutputType,

    #[clap(long, value_parser, default_value_t = false)]
    /// validate BSON during processing
    objcheck: bool,

    #[clap(long = "outFile", name = "outFile", value_parser)]
    /// path to output file to dump JSON to; default is stdout
    out_file: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let reader: Box<dyn BufRead> = match args.file.as_deref() {
        None => Box::new(BufReader::new(stdin())),
        Some(path) => {
            let file = File::open(path)?;
            Box::new(BufReader::new(file))
        }
    };

    let writer: Box<dyn Write> = match args.out_file.as_deref() {
        None => Box::new(BufWriter::new(stdout())),
        Some(path) => {
            let file = File::create(path)?;
            Box::new(BufWriter::new(file))
        }
    };

    let dump = BsonDump::new(reader, writer, args.objcheck);

    let start = Local::now();
    let dump_result = match args.output_type {
        OutputType::Json => dump.json(),
        OutputType::PrettyJson => dump.pretty_json(),
        OutputType::Debug => dump.debug(),
    };
    match dump_result {
        Err(error) => {
            print_num_found(start, error.get_num_found());
            eprintln!("{}", error.get_message());
        }
        Ok(num_found) => {
            print_num_found(start, num_found);
        }
    };

    Ok(())
}
