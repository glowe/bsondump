use std::io;

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
        while let Ok(document) = bson::Document::from_reader(&mut self.reader) {
            num_objects += self.debug_document(&document, 0)?;
        }
        write!(&mut self.writer, "{} objects found", num_objects)?;
        self.writer.flush()?;
        Ok(())
    }

    fn debug_array(&mut self, elements: &Vec<bson::Bson>, indent_level: usize) -> io::Result<u32> {
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
                bson::Bson::Document(inner) => self.debug_document(inner, indent_level + 3)?,
                bson::Bson::Array(inner) => self.debug_array(inner, indent_level + 3)?,
                _ => 0,
            }
        }
        Ok(num_objects)
    }

    fn debug_document(
        &mut self,
        document: &bson::Document,
        indent_level: usize,
    ) -> io::Result<u32> {
        let mut num_objects = 0;
        writeln!(
            &mut self.writer,
            "{}--- new object ---",
            get_indent(indent_level)
        )?;
        // FIXME: change this unwrap to an expect?
        let raw_doc_buf = bson::RawDocumentBuf::from_document(document).unwrap();
        writeln!(
            &mut self.writer,
            "{indent}size : {size}",
            indent = get_indent(indent_level + 1),
            size = raw_doc_buf.as_bytes().len()
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

            let size = size_of_type
                + size_of_name
                + match element {
                    bson::Bson::Array(_arr) => 9000, // FIXME
                    bson::Bson::DateTime(_ts) => 8,
                    bson::Bson::String(string) => {
                        // String - The int32 is the number bytes in the (byte*) + 1 (for the trailing '\x00').
                        // The (byte*) is zero or more UTF-8 encoded characters.
                        let num_of_bytes = 4usize; // 4 bytes
                        num_of_bytes + string.len() + 1 // null terminator
                    }
                    _ => {
                        9000 // FIXME
                    }
                };

            writeln!(
                &mut self.writer,
                "{indent}type: {type} size: {size}",
                indent = get_indent(indent_level + 3),
                type = element.element_type() as u8,
                size =size
            )?;
            num_objects += 1 + match element {
                bson::Bson::Document(inner) => self.debug_document(inner, indent_level + 3)?,
                bson::Bson::Array(inner) => self.debug_array(inner, indent_level + 3)?,
                _ => 0,
            }
        }
        Ok(num_objects)
    }
}
