use std::error::Error;
use std::io::Read;
use std::io::Write;
use std::result::Result;

use bson::RawArray;
use bson::RawBsonRef;
use bson::RawDocument;
use bson::RawDocumentBuf;

use chrono::offset::Local;

pub struct BsonDump<R: Read, W: Write> {
    reader: R,
    writer: W,
}

fn get_indent(indent_level: usize) -> String {
    " ".repeat(indent_level * 8)
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

impl<R, W> BsonDump<R, W>
where
    R: Read,
    W: Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        BsonDump { reader, writer }
    }

    pub fn json(mut self) -> Result<(), Box<dyn Error>> {
        while let Some(raw_document_buf) = self.next_raw_document_buf()? {
            let document = raw_document_buf.to_document()?;
            writeln!(&mut self.writer, "{}", document)?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn pretty_json(mut self) -> Result<(), Box<dyn Error>> {
        while let Some(raw_document_buf) = self.next_raw_document_buf()? {
            let document = raw_document_buf.to_document()?;
            writeln!(
                &mut self.writer,
                "{}",
                serde_json::to_string_pretty(&document)?
            )?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn debug(mut self) -> Result<(), Box<dyn Error>> {
        let start = Local::now();
        let mut num_objects = 0;
        while let Some(raw_document_buf) = self.next_raw_document_buf()? {
            self.debug_document(&raw_document_buf, 0)?;
            num_objects += 1;
        }
        write!(
            &mut self.writer,
            "{start}    {num_objects} objects found",
            start = start.format("%+"),
            num_objects = num_objects
        )?;
        self.writer.flush()?;
        Ok(())
    }

    fn next_raw_document_buf(
        &mut self,
    ) -> std::result::Result<Option<RawDocumentBuf>, Box<dyn Error>> {
        let mut buf: [u8; 4] = [0, 0, 0, 0];
        if let Err(error) = self.reader.read_exact(&mut buf) {
            if let std::io::ErrorKind::UnexpectedEof = error.kind() {
                return Ok(None);
            } else {
                return Err(Box::new(error));
            }
        }
        let length = i32::from_le_bytes(buf) as usize;

        let mut remainder = vec![0u8; length - buf.len()];
        self.reader.read_exact(&mut remainder)?;

        let mut bytes = Vec::from(buf);
        bytes.append(&mut remainder);
        Ok(Some(RawDocumentBuf::from_bytes(bytes)?))
    }

    fn write_new_object_header(
        &mut self,
        object: &(impl CountBytes + ?Sized),
        indent_level: usize,
    ) -> Result<(), Box<dyn Error>> {
        writeln!(
            &mut self.writer,
            "{}--- new object ---",
            get_indent(indent_level)
        )?;
        writeln!(
            &mut self.writer,
            "{indent}size : {size}",
            indent = get_indent(indent_level + 1),
            size = object.count_bytes(),
        )?;
        Ok(())
    }

    fn debug_item(
        &mut self,
        name: &str,
        bson_ref: &RawBsonRef,
        indent_level: usize,
    ) -> Result<(), Box<dyn Error>> {
        writeln!(
            &mut self.writer,
            "{indent}{name}",
            indent = get_indent(indent_level + 2),
            name = name,
        )?;
        let size_of_type = 1usize;
        let size_of_name = name.len() + 1; // null terminator
        let size = size_of_type + size_of_name + bson_ref.count_bytes();
        writeln!(
            &mut self.writer,
            "{indent}type: {type:>4} size: {size}",
            indent = get_indent(indent_level + 3),
            type = bson_ref.element_type() as u8,
            size = size
        )?;
        match bson_ref {
            RawBsonRef::Document(embedded) => self.debug_document(embedded, indent_level + 3)?,
            RawBsonRef::Array(embedded) => self.debug_array(embedded, indent_level + 3)?,
            _ => (),
        };
        Ok(())
    }

    fn debug_array(&mut self, array: &RawArray, indent_level: usize) -> Result<(), Box<dyn Error>> {
        self.write_new_object_header(array, indent_level)?;
        for (i, element) in array.into_iter().enumerate() {
            let bson_ref = element?;
            let name = format!("{}", i);
            self.debug_item(&name, &bson_ref, indent_level)?;
        }
        Ok(())
    }

    fn debug_document(
        &mut self,
        raw_document: &RawDocument,
        indent_level: usize,
    ) -> Result<(), Box<dyn Error>> {
        self.write_new_object_header(raw_document, indent_level)?;
        for element in raw_document.into_iter() {
            let (name, bson_ref) = element?;
            self.debug_item(name, &bson_ref, indent_level)?;
        }
        Ok(())
    }
}
