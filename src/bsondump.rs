use std::io;

const INDENT_SPACES: usize = 4;

pub struct BsonDump<R: io::Read, W: io::Write> {
    reader: R,
    writer: W,
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

    fn debug_array(&mut self, elements: &Vec<bson::Bson>, indent: usize) -> io::Result<u32> {
        let mut num_objects = 0;
        write!(
            &mut self.writer,
            "{spacer:indent$}--- new object ---\n",
            spacer = " ",
            indent = indent * INDENT_SPACES
        )?;
        let indent = indent + 1;
        for element in elements {
            // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
            write!(
                &mut self.writer,
                "{spacer:indent$}type: {type}\n",
                spacer=" ",
                indent=indent * INDENT_SPACES,
                type=element.element_type() as u8,
            )?;
            num_objects += 1;
            match element {
                bson::Bson::Document(inner) => {
                    num_objects += self.debug_document(&inner, indent + 1)?;
                }
                bson::Bson::Array(inner) => {
                    num_objects += self.debug_array(&inner, indent + 1)?;
                }
                _ => {}
            }
        }
        Ok(num_objects)
    }

    fn debug_document(&mut self, document: &bson::Document, indent: usize) -> io::Result<u32> {
        let mut num_objects = 0;
        write!(
            &mut self.writer,
            "{spacer:indent$}--- new object ---\n",
            spacer = " ",
            indent = indent * INDENT_SPACES
        )?;
        for (name, element) in document {
            // We can't get size without raw bson, but the bson crate doesn't support raw bson yet.
            write!(
                &mut self.writer,
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
                    num_objects += self.debug_document(inner, indent + 1)?;
                }
                bson::Bson::Array(inner) => {
                    num_objects += self.debug_array(inner, indent + 1)?;
                }
                _ => {}
            }
        }
        Ok(num_objects)
    }
}
