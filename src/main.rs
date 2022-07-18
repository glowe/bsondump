use clap::App;
use clap::Arg;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::result;
use std::str;

mod bsondump;

const DEBUG: &'static str = "debug";
const JSON: &'static str = "json";
const PRETTY_JSON: &'static str = "prettyJson";
const DEFAULT_OUTPUT_TYPE: &'static str = JSON;

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

fn main() -> io::Result<()> {
    // TODO:
    // - implement debug
    // - if the bson is invalid, spit out error
    // - implement objcheck
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

    let bsonfile = matches.value_of("bsonFile");
    let reader: Box<dyn io::BufRead> = match bsonfile {
        None => Box::new(io::BufReader::new(io::stdin())),
        Some(path) => {
            let file = fs::File::open(path)?;
            Box::new(io::BufReader::new(file))
        }
    };

    let outfile = matches.value_of("outFile");
    let writer: Box<dyn io::Write> = match outfile {
        None => Box::new(std::io::stdout()), // If someone chose stdio, they probably want to see results sooner.
        Some(path) => {
            let file = fs::File::create(path)?;
            Box::new(io::BufWriter::new(file))
        }
    };

    // TODO: These base strings should be static constants
    let output_type_arg = matches.value_of("type").unwrap_or(DEFAULT_OUTPUT_TYPE);
    let output_type =
        str::FromStr::from_str(output_type_arg).expect("output type was already validated by clap");
    let dump = bsondump::BsonDump::new(reader, writer);
    match output_type {
        OutputType::Json => dump.json()?,
        OutputType::PrettyJson => dump.pretty_json()?,
        OutputType::Debug => dump.debug()?,
    }
    Ok(())
}
