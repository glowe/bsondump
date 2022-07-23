use std::error::Error;
use std::io::{Read, Write};
use std::result::Result;

use bson::{RawArray, RawBsonRef, RawDocument};

use serde::ser::Serialize;

use serde_json::ser::PrettyFormatter;
use serde_json::value::Value;
use serde_json::Serializer;

mod iter;
use iter::raw_document_bufs;

mod bytes;
use bytes::CountBytes;

type DynResult<T> = Result<T, Box<dyn Error>>;
type BsonDumpResult<T> = Result<T, BsonDumpError>;

#[derive(Debug)]
pub struct BsonDumpError {
    num_found: u32,
    message: String,
}

impl BsonDumpError {
    pub fn get_num_found(&self) -> u32 {
        self.num_found
    }
    pub fn get_message(&self) -> &str {
        self.message.as_ref()
    }
}

fn get_indent(indent_level: usize) -> String {
    "\t".repeat(indent_level)
}

pub struct BsonDump<R: Read, W: Write> {
    reader: R,
    writer: W,
    objcheck: bool,
    num_found: u32,
}

impl<R: Read, W: Write> BsonDump<R, W> {
    pub fn new(reader: R, writer: W, objcheck: bool) -> Self {
        BsonDump {
            reader,
            writer,
            objcheck,
            num_found: 0,
        }
    }

    pub fn json(mut self) -> BsonDumpResult<u32> {
        self.print_json(false)
            .map_err(|e| self.to_bsondump_error(e))?;
        Ok(self.num_found)
    }

    pub fn pretty_json(mut self) -> BsonDumpResult<u32> {
        self.print_json(true)
            .map_err(|e| self.to_bsondump_error(e))?;
        Ok(self.num_found)
    }

    pub fn debug(mut self) -> BsonDumpResult<u32> {
        self.print_debug().map_err(|e| self.to_bsondump_error(e))?;
        Ok(self.num_found)
    }

    fn print_pretty_json(writer: &mut W, value: Value, indent: &[u8]) -> DynResult<()> {
        let formatter = PrettyFormatter::with_indent(indent);
        let mut ser = Serializer::with_formatter(writer, formatter);
        value
            .serialize(&mut ser)
            .map_err(|err| Box::new(err) as Box<dyn Error>)
    }

    fn print_json(&mut self, is_pretty: bool) -> DynResult<()> {
        self.num_found = 0;
        for raw_document_buf in raw_document_bufs(&mut self.reader) {
            let value = match bson::to_bson(&raw_document_buf?) {
                Err(error) => {
                    if !self.objcheck {
                        continue;
                    }
                    return Err(Box::new(error));
                }
                Ok(value) => value,
            };

            let extjson = value.into_canonical_extjson();

            if is_pretty {
                Self::print_pretty_json(&mut self.writer, extjson, b"\t")?;
            } else {
                writeln!(&mut self.writer, "{}", extjson)?;
            }
            self.num_found += 1;
        }
        self.writer.flush()?;
        Ok(())
    }

    fn print_debug(&mut self) -> DynResult<()> {
        self.num_found = 0;
        for raw_document_buf in raw_document_bufs(&mut self.reader) {
            if let Err(error) = Self::print_debug_document(&mut self.writer, &raw_document_buf?, 0)
            {
                if !self.objcheck {
                    continue;
                }
                return Err(error);
            };
            self.num_found += 1;
        }
        self.writer.flush()?;
        Ok(())
    }

    fn print_new_object_header<O: CountBytes + ?Sized>(
        writer: &mut W,
        object: &O,
        indent_level: usize,
    ) -> DynResult<()> {
        writeln!(writer, "{}--- new object ---", get_indent(indent_level))?;
        writeln!(
            writer,
            "{indent}size : {size}",
            indent = get_indent(indent_level + 1),
            size = object.count_bytes(),
        )?;
        Ok(())
    }

    fn print_debug_array(writer: &mut W, array: &RawArray, indent_level: usize) -> DynResult<()> {
        Self::print_new_object_header(writer, array, indent_level)?;
        for (i, element) in array.into_iter().enumerate() {
            let name = i.to_string();
            let bson_ref = element?;
            Self::print_debug_item(writer, &name, &bson_ref, indent_level)?;
        }
        Ok(())
    }

    fn print_debug_document(
        writer: &mut W,
        raw_document: &RawDocument,
        indent_level: usize,
    ) -> DynResult<()> {
        Self::print_new_object_header(writer, raw_document, indent_level)?;
        for element in raw_document {
            let (name, bson_ref) = element?;
            Self::print_debug_item(writer, name, &bson_ref, indent_level)?;
        }
        Ok(())
    }

    fn print_debug_item(
        writer: &mut W,
        name: &str,
        bson_ref: &RawBsonRef,
        indent_level: usize,
    ) -> DynResult<()> {
        writeln!(
            writer,
            "{indent}{name}",
            indent = get_indent(indent_level + 2),
            name = name,
        )?;
        let size_of_type = 1usize;
        let size_of_name = name.len() + 1; // null terminator
        let size = size_of_type + size_of_name + bson_ref.count_bytes();
        writeln!(
            writer,
            "{indent}type: {type:>4} size: {size}",
            indent = get_indent(indent_level + 3),
            type = bson_ref.element_type() as u8,
            size = size
        )?;
        match bson_ref {
            RawBsonRef::Document(embedded) => {
                Self::print_debug_document(writer, embedded, indent_level + 3)?
            }
            RawBsonRef::Array(embedded) => {
                Self::print_debug_array(writer, embedded, indent_level + 3)?
            }
            _ => (),
        };
        Ok(())
    }

    fn to_bsondump_error(&self, e: Box<dyn Error>) -> BsonDumpError {
        BsonDumpError {
            num_found: self.num_found,
            message: e.to_string(),
        }
    }
}
