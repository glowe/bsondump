use clap::App;
use clap::Arg;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::result;
use std::str;

use chrono::offset::Local;
use chrono::DateTime;
use chrono::TimeZone;

use crate::bsondump::BsonDump;

pub mod bsondump;

const DEBUG: &str = "debug";
const JSON: &str = "json";
const PRETTY_JSON: &str = "prettyJson";
const DEFAULT_OUTPUT_TYPE: &str = JSON;

enum OutputType {
    Debug,
    Json,
    PrettyJson,
}

#[derive(Debug)]
struct ParseOutputTypeError {
    output_type: String,
}

impl fmt::Display for ParseOutputTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unrecogized outputType {}", self.output_type)
    }
}

impl error::Error for ParseOutputTypeError {}

impl str::FromStr for OutputType {
    type Err = ParseOutputTypeError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            DEBUG => Ok(OutputType::Debug),
            JSON => Ok(OutputType::Json),
            PRETTY_JSON => Ok(OutputType::PrettyJson),
            _ => Err(ParseOutputTypeError {
                output_type: s.to_string(),
            }),
        }
    }
}

fn print_num_found<Tz>(start: DateTime<Tz>, num_found: u32)
where
    Tz: TimeZone,
    <Tz as TimeZone>::Offset: std::fmt::Display,
{
    eprintln!(
        "{start}    {num_found} objects found",
        start = start.format("%+"),
        num_found = num_found
    );
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // TODO:
    // - create a struct and use App to populate it properly
    //   See https://www.fpcomplete.com/rust/command-line-parsing-clap/
    // - refactor main into helper functions
    // - add tests
    let matches = App::new("bsondump")
        .version("0.1")
        .about(
            "View and debug .bson files.

See http://docs.mongodb.org/manual/reference/program/bsondump/ for more information.",
        )
        .arg(
            Arg::with_name("type")
                .long("type")
                .possible_values(&[DEBUG, JSON, PRETTY_JSON])
                .takes_value(true)
                .help(&*format!(
                    "type of output (default '{}')",
                    DEFAULT_OUTPUT_TYPE
                )),
        )
        .arg(
            Arg::with_name("objcheck")
                .long("objcheck")
                .takes_value(false)
                .help("validate BSON during processing"),
        )
        .arg(
            Arg::with_name("bsonFile")
                .long("bsonFile")
                .takes_value(true)
                .help("path to BSON file to dump to JSON; default is stdin"),
        )
        .arg(
            Arg::with_name("outFile")
                .long("outFile")
                .takes_value(true)
                .help("path to output file to dump JSON to; default is stdout"),
        )
        .get_matches();

    let reader: Box<dyn io::BufRead> = match matches.get_one::<String>("bsonFile") {
        None => Box::new(io::BufReader::new(io::stdin())),
        Some(path) => {
            let file = fs::File::open(path)?;
            Box::new(io::BufReader::new(file))
        }
    };

    let writer: Box<dyn io::Write> = match matches.get_one::<String>("outFile") {
        None => Box::new(io::BufWriter::new(std::io::stdout())),
        Some(path) => {
            let file = fs::File::create(path)?;
            Box::new(io::BufWriter::new(file))
        }
    };

    let output_type_arg = matches.value_of("type").unwrap_or(DEFAULT_OUTPUT_TYPE);
    let output_type =
        str::FromStr::from_str(output_type_arg).expect("output type was already validated by clap");
    let objcheck = matches.is_present("objcheck");
    let dump = BsonDump::new(reader, writer, objcheck);
    let start = Local::now();
    let debug_result = match output_type {
        OutputType::Json => dump.json(),
        OutputType::PrettyJson => dump.pretty_json(),
        OutputType::Debug => dump.debug(),
    };
    match debug_result {
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
