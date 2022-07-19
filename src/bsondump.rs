use std::io;

use bson::Bson;
use bson::Document;
use bson::RawDocumentBuf;

/*
enum DebugBsonValue {
    Atom {
        element_type: bson::spec::ElementType,
        length: u32,
    },
    Composite {
        element_type: bson::spec::ElementType,
        length: u32,
        elements: Vec<DebugBsonValue>,
    },
}
 */
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

impl RawSize for bson::Bson {
    fn get_raw_size(&self) -> usize {
        match self {
            Bson::Array(array) => array.get_raw_size(),
            Bson::Binary(binary) => I32_BYTES + 1 + binary.bytes.len(),
            Bson::Boolean(_) => 1,
            Bson::DateTime(_ts) => 8,
            Bson::DbPointer(_) => String::from("").get_raw_size() + 12,
            Bson::Decimal128(dec) => dec.bytes().len(),
            Bson::Document(doc) => doc.get_raw_size(),
            Bson::Double(_) => 8,
            Bson::Int32(_) => 4,
            Bson::Int64(_) => 8,
            Bson::JavaScriptCode(code) => code.get_raw_size(),
            Bson::String(string) => string.get_raw_size(),
            Bson::Null => 0,
            Bson::RegularExpression(regex) => {
                regex.pattern.get_raw_size() + regex.options.get_raw_size()
            }
            Bson::JavaScriptCodeWithScope(code_with_scope) => {
                I32_BYTES
                    + code_with_scope.code.get_raw_size()
                    + code_with_scope.scope.get_raw_size()
            }
            Bson::Timestamp(_) => 8,
            Bson::ObjectId(_) => 12,
            Bson::Symbol(symbol) => symbol.get_raw_size(),
            Bson::Undefined => 0,
            Bson::MaxKey => 0,
            Bson::MinKey => 0,
        }
    }
}

impl RawSize for &String {
    fn get_raw_size(&self) -> usize {
        // i32 size + characters + null terminator
        I32_BYTES + self.len() + 1
    }
}

impl RawSize for String {
    fn get_raw_size(&self) -> usize {
        // i32 size + characters + null terminator
        I32_BYTES + self.len() + 1
    }
}

impl RawSize for Document {
    fn get_raw_size(&self) -> usize {
        let raw_doc_buf = RawDocumentBuf::from_document(self)
            .expect("Unable to create RawDocumentBuf from Document");
        raw_doc_buf.as_bytes().len()
    }
}

impl RawSize for &Vec<Bson> {
    fn get_raw_size(&self) -> usize {
        self.iter().fold(0, |acc, elem| acc + elem.get_raw_size())
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

    pub fn debug(mut self) -> io::Result<()> {
        let mut num_objects = 0;
        while let Ok(document) = Document::from_reader(&mut self.reader) {
            num_objects += self.debug_document(&document, 0)?;
        }
        write!(&mut self.writer, "{} objects found", num_objects)?;
        self.writer.flush()?;
        Ok(())
    }

    fn debug_array(&mut self, elements: &Vec<Bson>, indent_level: usize) -> io::Result<u32> {
        let mut num_objects = 0;
        writeln!(
            &mut self.writer,
            "{}--- new object ---",
            get_indent(indent_level)
        )?;
        for element in elements {
            // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
            writeln!(
                &mut self.writer,
                "{indent}type: {type}",
                indent=get_indent(indent_level + 2),
                type=element.element_type() as u8,
            )?;
            num_objects += 1 + match element {
                Bson::Document(inner) => self.debug_document(inner, indent_level + 3)?,
                Bson::Array(inner) => self.debug_array(inner, indent_level + 3)?,
                _ => 0,
            }
        }
        Ok(num_objects)
    }

    fn debug_document(&mut self, document: &Document, indent_level: usize) -> io::Result<u32> {
        let mut num_objects = 0;

        writeln!(
            &mut self.writer,
            "{}--- new object ---",
            get_indent(indent_level)
        )?;

        writeln!(
            &mut self.writer,
            "{indent}size : {size}",
            indent = get_indent(indent_level + 1),
            size = document.get_raw_size()
        )?;

        for (name, element) in document {
            writeln!(
                &mut self.writer,
                "{indent}{name}",
                indent = get_indent(indent_level + 2),
                name = name,
            )?;

            let size_of_type = 1usize;
            let size_of_name = name.len() + 1; // null terminator
            let size = size_of_type + size_of_name + element.get_raw_size();

            writeln!(
                &mut self.writer,
                "{indent}type: {type} size: {size}",
                indent = get_indent(indent_level + 3),
                type = element.element_type() as u8,
                size =size
            )?;
            num_objects += 1 + match element {
                Bson::Document(inner) => self.debug_document(inner, indent_level + 3)?,
                Bson::Array(inner) => self.debug_array(inner, indent_level + 3)?,
                _ => 0,
            }
        }
        Ok(num_objects)
    }
}
