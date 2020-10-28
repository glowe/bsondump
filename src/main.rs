use clap::{App, Arg};
use serde_json;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::result;
use std::str;

fn dump_json<R, W>(mut reader: R, mut writer: W) -> io::Result<()>
where
    R: io::Read,
    W: io::Write,
{
    while let Ok(deserialized) = bson::Document::from_reader(&mut reader) {
        write!(&mut writer, "{}\n", deserialized)?;
    }
    Ok(())
}

fn dump_pretty_json<R, W>(mut reader: R, mut writer: W) -> io::Result<()>
where
    R: io::BufRead,
    W: io::Write,
{
    while let Ok(deserialized) = bson::Document::from_reader(&mut reader) {
        write!(
            &mut writer,
            "{}\n",
            serde_json::to_string_pretty(&deserialized).unwrap()
        )?;
    }
    Ok(())
}

const INDENT_SPACES: usize = 4;

fn debug_array<W>(mut writer: &mut W, elements: &Vec<bson::Bson>, indent: usize) -> io::Result<u32>
where
    W: io::Write,
{
    let mut num_objects = 0;
    write!(
        &mut writer,
        "{spacer:indent$}--- new object ---\n",
        spacer = " ",
        indent = indent * INDENT_SPACES
    )?;
    let indent = indent + 1;
    for element in elements {
        // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
        write!(
            &mut writer,
            "{spacer:indent$}type: {type}\n",
            spacer=" ",
            indent=indent * INDENT_SPACES,
            type=element.element_type() as u8,
        )?;
        num_objects += 1;
        match element {
            bson::Bson::Document(inner) => {
                num_objects += debug_document(writer, &inner, indent + 1)?;
            }
            bson::Bson::Array(inner) => {
                num_objects += debug_array(writer, &inner, indent + 1)?;
            }
            _ => {}
        }
    }
    Ok(num_objects)
}

fn debug_document<W>(
    mut writer: &mut W,
    document: &bson::Document,
    indent: usize,
) -> io::Result<u32>
where
    W: io::Write,
{
    let mut num_objects = 0;
    write!(
        &mut writer,
        "{spacer:indent$}--- new object ---\n",
        spacer = " ",
        indent = indent * INDENT_SPACES
    )?;
    for (name, element) in document {
        // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
        write!(
            &mut writer,
            "{spacer:indent$}{name}\n{spacer:double_indent$}type: {type}\n",
            spacer=" ",
            indent=indent * INDENT_SPACES,
            name=name,
            double_indent=(indent + 1) * INDENT_SPACES,
            type=element.element_type() as u8,
        )?;
        num_objects += 1;
        match element {
            bson::Bson::Document(inner) => {
                num_objects += debug_document(writer, inner, indent + 1)?;
            }
            bson::Bson::Array(inner) => {
                num_objects += debug_array(writer, inner, indent + 1)?;
            }
            _ => {}
        }
    }
    Ok(num_objects)
}

fn dump_debug<R, W>(mut reader: R, mut writer: W) -> io::Result<()>
where
    R: io::BufRead,
    W: io::Write,
{
    let mut num_objects = 0;
    while let Ok(document) = bson::Document::from_reader(&mut reader) {
        num_objects += debug_document(&mut writer, &document, 1)?;
    }
    write!(&mut writer, "{} objects found", num_objects)?;
    Ok(())
}

fn dump<R, W>(output_type: OutputType, mut reader: R, mut writer: W) -> io::Result<()>
where
    R: io::BufRead,
    W: io::Write,
{
    match output_type {
        OutputType::Json => dump_json(&mut reader, &mut writer)?,
        OutputType::PrettyJson => dump_pretty_json(&mut reader, &mut writer)?,
        OutputType::Debug => dump_debug(&mut reader, &mut writer)?,
    }
    writer.flush()?;
    Ok(())
}

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
                .help(&format!(
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
    let mut reader: Box<dyn io::BufRead> = match bsonfile {
        None => Box::new(io::BufReader::new(io::stdin())),
        Some(path) => {
            let file = fs::File::open(path)?;
            Box::new(io::BufReader::new(file))
        }
    };

    let outfile = matches.value_of("outFile");
    let mut writer: Box<dyn io::Write> = match outfile {
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
    dump(output_type, &mut reader, &mut writer)?;
    Ok(())
}
