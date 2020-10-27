pub mod bsondump;

use bson;
use std::io;

struct BsonDump {
    reader: Box<dyn io::BufRead>,
    writer: Box<dyn io::Write>,
}

impl BsonDump {
    fn new(reader: io::BufRead, writer: io::Writer) -> Self {
        Self {
            reader: reader,
            writer: writer,
        }
    }

    fn dump_json(&self) {
        while let Ok(deserialized) = bson::Document::from_reader(&mut reader) {
             write!(&mut writer, "{}\n", deserialized)?;
        }
        writer.flush()?;
    }

    fn dump_pretty_json(&self) {
        while let Ok(deserialized) = Document::from_reader(&mut reader) {
              write!(
                    &mut writer,
                    "{}\n",
                    serde_json::to_string_pretty(&deserialized).unwrap()
                )?;
        }
        writer.flush()?;
    }

    fn dump_debug(&self) {

    }
}
