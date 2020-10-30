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
        while let Ok(deserialized) = bson::Document::from_reader(&mut self.reader) {
            write!(&mut self.writer, "{}\n", deserialized)?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn pretty_json(mut self) -> io::Result<()> {
        while let Ok(deserialized) = bson::Document::from_reader(&mut self.reader) {
            write!(
                &mut self.writer,
                "{}\n",
                serde_json::to_string_pretty(&deserialized).unwrap()
            )?;
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn debug(mut self) -> io::Result<()> {
        let mut num_objects = 0;
        while let Ok(document) = bson::Document::from_reader(&mut self.reader) {
            num_objects += self.debug_document(&document, 1)?;
        }
        write!(&mut self.writer, "{} objects found", num_objects)?;
        self.writer.flush()?;
        Ok(())
    }

    fn debug_array(&mut self, elements: &Vec<bson::Bson>, indent_level: usize) -> io::Result<u32> {
        let mut num_objects = 0;
        write!(&mut self.writer, "{}--- new object ---\n", get_indent(indent_level))?;
        for element in elements {
            // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
            write!(
                &mut self.writer,
                "{indent}type: {type}\n",
                indent=get_indent(indent_level + 2),
                type=element.element_type() as u8,
            )?;
            num_objects += 1;
            match element {
                bson::Bson::Document(inner) => {
                    num_objects += self.debug_document(&inner, indent_level + 3)?;
                }
                bson::Bson::Array(inner) => {
                    num_objects += self.debug_array(&inner, indent_level + 3)?;
                }
                _ => {}
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
        write!(&mut self.writer, "{}--- new object ---\n", get_indent(indent_level))?;
        for (name, element) in document {
            // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
            write!(
                &mut self.writer,
                "{indent}{name}\n",
                indent = get_indent(indent_level + 1),
                name = name,
            )?;
            write!(
                &mut self.writer,
                "{indent}type: {type}\n",
                indent=get_indent(indent_level + 2),
                type=element.element_type() as u8,
            )?;
            num_objects += 1;
            match element {
                bson::Bson::Document(inner) => {
                    num_objects += self.debug_document(inner, indent_level + 3)?;
                }
                bson::Bson::Array(inner) => {
                    num_objects += self.debug_array(inner, indent_level + 3)?;
                }
                _ => {}
            }
        }
        Ok(num_objects)
    }
}
