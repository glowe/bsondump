use std::error::Error;
use std::io::Read;
use std::io::Write;
use std::result::Result;

use bson::RawArray;
use bson::RawBsonRef;
use bson::RawDocument;
use bson::RawDocumentBuf;

use serde::ser::Serialize;

use serde_json::ser::PrettyFormatter;
use serde_json::value::Value;
use serde_json::Serializer;

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

pub struct RawDocumentBufs<'reader, R: Read> {
    reader: &'reader mut R,
}

fn raw_document_bufs<R: Read>(reader: &mut R) -> RawDocumentBufs<R> {
    RawDocumentBufs { reader }
}

impl<'r, R: Read> std::iter::Iterator for RawDocumentBufs<'r, R> {
    type Item = Result<RawDocumentBuf, Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf: [u8; 4] = [0, 0, 0, 0];
        if let Err(error) = self.reader.read_exact(&mut buf) {
            if let std::io::ErrorKind::UnexpectedEof = error.kind() {
                return None;
            } else {
                return Some(Err(Box::new(error)));
            }
        }
        let length = i32::from_le_bytes(buf) as usize;

        let mut remainder = vec![0u8; length - buf.len()];
        if let Err(error) = self.reader.read_exact(&mut remainder) {
            return Some(Err(Box::new(error)));
        }

        let mut bytes = Vec::from(buf);
        bytes.append(&mut remainder);

        match RawDocumentBuf::from_bytes(bytes) {
            Ok(raw_document_buf) => Some(Ok(raw_document_buf)),
            Err(error) => Some(Err(Box::new(error))),
        }
    }
}

fn get_indent(indent_level: usize) -> String {
    "\t".repeat(indent_level)
}

trait CountBytes {
    fn count_bytes(&self) -> usize;
}

impl CountBytes for &str {
    fn count_bytes(&self) -> usize {
        // i32 size + characters + null terminator
        4 + self.len() + 1
    }
}

impl CountBytes for RawDocument {
    fn count_bytes(&self) -> usize {
        self.as_bytes().len()
    }
}

impl CountBytes for RawArray {
    fn count_bytes(&self) -> usize {
        self.as_bytes().len()
    }
}

impl CountBytes for bson::RawBsonRef<'_> {
    fn count_bytes(&self) -> usize {
        match self {
            RawBsonRef::Double(_) => 8,
            RawBsonRef::String(string) => string.count_bytes(),
            RawBsonRef::Array(raw_array) => raw_array.count_bytes(),
            RawBsonRef::Document(raw_document) => raw_document.count_bytes(),
            RawBsonRef::Boolean(_) => 1,
            RawBsonRef::Null => 0,
            RawBsonRef::RegularExpression(regex) => {
                regex.pattern.count_bytes() + regex.options.count_bytes()
            }
            RawBsonRef::JavaScriptCode(code) => code.count_bytes(),
            RawBsonRef::JavaScriptCodeWithScope(cws) => {
                cws.code.count_bytes() + cws.scope.count_bytes()
            }
            RawBsonRef::Int32(_) => 4,
            RawBsonRef::Int64(_) => 8,
            RawBsonRef::Timestamp(_) => 8,
            RawBsonRef::Binary(raw_binary_ref) => 4 + 1 + raw_binary_ref.bytes.len(),
            RawBsonRef::ObjectId(_) => 12,
            RawBsonRef::DateTime(_) => 8,
            RawBsonRef::Symbol(symbol) => symbol.count_bytes(),
            RawBsonRef::Decimal128(dec) => dec.bytes().len(),
            RawBsonRef::Undefined => 0,
            RawBsonRef::MaxKey => 0,
            RawBsonRef::MinKey => 0,
            RawBsonRef::DbPointer(_) => "".count_bytes() + 12,
        }
    }
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

    pub fn json(mut self) -> Result<u32, BsonDumpError> {
        if let Err(error) = self.print_json(false) {
            return Err(BsonDumpError {
                num_found: self.num_found,
                message: error.to_string(),
            });
        }
        Ok(self.num_found)
    }

    fn print_pretty_json(
        writer: &mut W,
        value: Value,
        indent: &[u8],
    ) -> Result<(), serde_json::Error>
where {
        let formatter = PrettyFormatter::with_indent(indent);
        let mut ser = Serializer::with_formatter(writer, formatter);
        value.serialize(&mut ser)
    }

    fn print_json(&mut self, is_pretty: bool) -> Result<(), Box<dyn Error>> {
        self.num_found = 0;
        for raw_document_buf in raw_document_bufs(&mut self.reader) {
            let value = match bson::to_bson(&raw_document_buf.unwrap()) {
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

    pub fn pretty_json(mut self) -> Result<u32, BsonDumpError> {
        if let Err(error) = self.print_json(true) {
            return Err(BsonDumpError {
                num_found: self.num_found,
                message: error.to_string(),
            });
        }
        Ok(self.num_found)
    }

    pub fn debug(mut self) -> Result<u32, BsonDumpError> {
        if let Err(error) = self.print_debug() {
            return Err(BsonDumpError {
                num_found: self.num_found,
                message: error.to_string(),
            });
        }
        Ok(self.num_found)
    }

    fn print_debug(&mut self) -> Result<(), Box<dyn Error>> {
        self.num_found = 0;
        for raw_document_buf in raw_document_bufs(&mut self.reader) {
            if let Err(error) =
                Self::print_debug_document(&mut self.writer, &raw_document_buf.unwrap(), 0)
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

    fn print_new_object_header(
        writer: &mut W,
        object: &(impl CountBytes + ?Sized),
        indent_level: usize,
    ) -> Result<(), Box<dyn Error>> {
        writeln!(writer, "{}--- new object ---", get_indent(indent_level))?;
        writeln!(
            writer,
            "{indent}size : {size}",
            indent = get_indent(indent_level + 1),
            size = object.count_bytes(),
        )?;
        Ok(())
    }

    fn print_debug_item(
        writer: &mut W,
        name: &str,
        bson_ref: &RawBsonRef,
        indent_level: usize,
    ) -> Result<(), Box<dyn Error>> {
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

    fn print_debug_array(
        writer: &mut W,
        array: &RawArray,
        indent_level: usize,
    ) -> Result<(), Box<dyn Error>> {
        Self::print_new_object_header(writer, array, indent_level)?;
        for (i, element) in array.into_iter().enumerate() {
            let bson_ref = element?;
            let name = format!("{}", i);
            Self::print_debug_item(writer, &name, &bson_ref, indent_level)?;
        }
        Ok(())
    }

    fn print_debug_document(
        writer: &mut W,
        raw_document: &RawDocument,
        indent_level: usize,
    ) -> Result<(), Box<dyn Error>> {
        Self::print_new_object_header(writer, raw_document, indent_level)?;
        for element in raw_document {
            let (name, bson_ref) = element?;
            Self::print_debug_item(writer, name, &bson_ref, indent_level)?;
        }
        Ok(())
    }
}
