use std::io;

use bson::RawArray;
use bson::RawBsonRef;
use bson::RawDocument;

pub struct BsonDump<R: io::Read, W: io::Write> {
    reader: R,
    writer: W,
}

fn get_indent(indent_level: usize) -> String {
    " ".repeat(indent_level * 8)
}

trait RawSize {
    fn get_raw_size(&self) -> usize;
}

const I32_BYTES: usize = 4;

impl RawSize for &str {
    fn get_raw_size(&self) -> usize {
        // i32 size + characters + null terminator
        I32_BYTES + self.len() + 1
    }
}

impl RawSize for RawDocument {
    fn get_raw_size(&self) -> usize {
        self.as_bytes().len()
    }
}

impl RawSize for bson::RawBsonRef<'_> {
    fn get_raw_size(&self) -> usize {
        match self {
            RawBsonRef::Double(_) => 8,
            RawBsonRef::String(string) => string.get_raw_size(),
            RawBsonRef::Array(raw_array) => raw_array.as_bytes().len(),
            RawBsonRef::Document(raw_document) => raw_document.get_raw_size(),
            RawBsonRef::Boolean(_) => 1,
            RawBsonRef::Null => 0,
            RawBsonRef::RegularExpression(regex) => {
                regex.pattern.get_raw_size() + regex.options.get_raw_size()
            }
            RawBsonRef::JavaScriptCode(code) => code.get_raw_size(),
            RawBsonRef::JavaScriptCodeWithScope(cws) => {
                cws.code.get_raw_size() + cws.scope.get_raw_size()
            }
            RawBsonRef::Int32(_) => 4,
            RawBsonRef::Int64(_) => 8,
            RawBsonRef::Timestamp(_) => 8,
            RawBsonRef::Binary(_) => todo!(),
            RawBsonRef::ObjectId(_) => 12,
            RawBsonRef::DateTime(_) => 8,
            RawBsonRef::Symbol(symbol) => symbol.get_raw_size(),
            RawBsonRef::Decimal128(dec) => dec.bytes().len(),
            RawBsonRef::Undefined => 0,
            RawBsonRef::MaxKey => 0,
            RawBsonRef::MinKey => 0,
            RawBsonRef::DbPointer(_) => "".get_raw_size() + 12,
        }
    }
}

impl<R, W> BsonDump<R, W>
where
    R: io::Read,
    W: io::Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        BsonDump { reader, writer }
    }

    pub fn json(mut self) -> io::Result<()> {
        while let Ok(deserialized) = bson::Document::from_reader(&mut self.reader) {
            writeln!(&mut self.writer, "{}", deserialized)?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn pretty_json(mut self) -> io::Result<()> {
        while let Ok(deserialized) = bson::Document::from_reader(&mut self.reader) {
            writeln!(
                &mut self.writer,
                "{}",
                serde_json::to_string_pretty(&deserialized).unwrap()
            )?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn debug(mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut num_objects = 0;

        loop {
            let mut buf: [u8; 4] = [0, 0, 0, 0];
            if let Err(error) = self.reader.read_exact(&mut buf) {
                if let std::io::ErrorKind::UnexpectedEof = error.kind() {
                    break;
                } else {
                    return Err(Box::new(error));
                }
            }
            let length = i32::from_le_bytes(buf) as usize;

            let mut remainder = vec![0u8; length - buf.len()];
            self.reader.read_exact(&mut remainder)?;

            let mut bytes = Vec::from(buf);
            bytes.append(&mut remainder);
            let raw_document = RawDocument::from_bytes(&bytes)?;
            self.debug_document(raw_document, 0)?;
            num_objects += 1;
        }

        write!(&mut self.writer, "{} objects found", num_objects)?;
        self.writer.flush()?;
        Ok(())
    }

    fn debug_array(
        &mut self,
        array: &RawArray,
        indent_level: usize,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        writeln!(
            &mut self.writer,
            "{}--- new object ---",
            get_indent(indent_level)
        )?;
        for element in array {
            let element = element?;
            writeln!(
                &mut self.writer,
                "{indent}type: {type}",
                indent=get_indent(indent_level + 2),
                type=element.element_type() as u8,
            )?;
            match element {
                RawBsonRef::Document(embedded) => {
                    self.debug_document(embedded, indent_level + 3)?
                }
                RawBsonRef::Array(array) => self.debug_array(array, indent_level + 3)?,
                _ => (),
            };
        }
        Ok(())
    }

    fn debug_document(
        &mut self,
        raw_document: &RawDocument,
        indent_level: usize,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        writeln!(
            &mut self.writer,
            "{}--- new object ---",
            get_indent(indent_level)
        )?;

        writeln!(
            &mut self.writer,
            "{indent}size : {size}",
            indent = get_indent(indent_level + 1),
            size = raw_document.as_bytes().len()
        )?;

        for element in raw_document {
            let (name, bson_ref) = element?;
            writeln!(
                &mut self.writer,
                "{indent}{name}",
                indent = get_indent(indent_level + 2),
                name = name,
            )?;

            let size_of_type = 1usize;
            let size_of_name = name.len() + 1; // null terminator
            let size = size_of_type + size_of_name + bson_ref.get_raw_size();

            writeln!(
                &mut self.writer,
                "{indent}type: {type} size: {size}",
                indent = get_indent(indent_level + 3),
                type = bson_ref.element_type() as u8,
                size =size
            )?;
            match bson_ref {
                RawBsonRef::Document(embedded) => {
                    self.debug_document(embedded, indent_level + 3)?
                }
                RawBsonRef::Array(embedded) => self.debug_array(embedded, indent_level + 3)?,
                _ => (),
            };
        }
        Ok(())
    }
}
