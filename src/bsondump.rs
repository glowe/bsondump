use std::io;

pub struct BsonDump<R: io::Read, W: io::Write> {
    reader: R,
    writer: W,
}

fn get_indent(indent_level: usize) -> String {
    " ".repeat(indent_level * 4)
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
        // bson::RawDocument::from_bytes
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
        for (name, element) in document {
            // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
            // TODO: implement rawbson for this. This may eliminate the need to have separate debug
            // document and array methods.
            writeln!(
                &mut self.writer,
                "{indent}{name}",
                indent = get_indent(indent_level + 1),
                name = name,
            )?;
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
}
