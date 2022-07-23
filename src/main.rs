use std::{
    error::Error,
    fs::File,
    io::{stdin, stdout, BufRead, BufReader, BufWriter, Write},
    result::Result,
};

use clap::{ArgEnum, Parser};
use clap_verbosity_flag::Verbosity;
use log::{error, info};

use bsondump::BsonDump;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
#[clap(rename_all = "camelCase")]
enum OutputType {
    Debug,
    Json,
    PrettyJson,
}

#[derive(Parser)]
#[clap(rename_all = "camelCase")]
struct Cli {
    /// Path to BSON file to dump to JSON; default is stdin
    file: Option<String>,

    #[clap(flatten)]
    verbose: Verbosity,

    #[clap(name="type", long="type", arg_enum, default_value_t = OutputType::Json)]
    // type of output: debug, json, prettyJson
    output_type: OutputType,

    #[clap(long)]
    /// Validate BSON during processing
    objcheck: bool,

    #[clap(long = "outFile", name = "outFile")]
    /// Path to output file to dump JSON to; default is stdout
    out_file: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // FIXME: add nicer error messages that don't contain
    //   Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
    // FIXME: add max bson size test

    let args = Cli::parse();

    env_logger::Builder::new().filter_level(args.verbose.log_level_filter()).init();

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

    let dump_result = match args.output_type {
        OutputType::Json => dump.json(),
        OutputType::PrettyJson => dump.pretty_json(),
        OutputType::Debug => dump.debug(),
    };
    match dump_result {
        Err(error) => {
            info!("{num_found} objects found", num_found = error.get_num_found());
            error!("{}", error.get_message());
        }
        Ok(num_found) => {
            info!("{num_found} objects found", num_found = num_found);
        }
    };

    Ok(())
}
