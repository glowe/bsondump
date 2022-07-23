use std::{error::Error, io::Read, result::Result};

use bson::RawDocumentBuf;

pub struct RawDocumentBufs<'reader, R: Read> {
    reader: &'reader mut R,
}

pub fn raw_document_bufs<R: Read>(reader: &mut R) -> RawDocumentBufs<R> {
    RawDocumentBufs { reader }
}


// 16kb + 16mb - This is the maximum size we would get when dumping the
// oplog itself. See https://jira.mongodb.org/browse/TOOLS-3001.
const MAX_BSON_SIZE: usize = (16 * 1024 * 1024) + (16 * 1024);
const MIN_BSON_SIZE: usize = 5;

#[derive(Debug)]
struct BsonSizeError {
    size: usize,
    message: String,
}
impl std::fmt::Display for BsonSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", MAX_BSON_SIZE, self.size, self.message)
    }
}
impl std::error::Error for BsonSizeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }
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
        let bson_size = i32::from_le_bytes(buf) as usize;

        if bson_size < MIN_BSON_SIZE {
            return Some(Err(Box::new(BsonSizeError {
                size: bson_size,
                message: String::from("Too small nelly"),
            })));
        }

        if bson_size > MAX_BSON_SIZE {
            return Some(Err(Box::new(BsonSizeError {
                size: bson_size,
                message: String::from("Woah nelly"),
            })));
        }

        let mut remainder = vec![0u8; bson_size - buf.len()];
        if let Err(error) = self.reader.read_exact(&mut remainder) {
            return Some(Err(Box::new(error)));
        }

        let mut bytes = Vec::from(buf);
        bytes.append(&mut remainder);
        Some(RawDocumentBuf::from_bytes(bytes).map_err(|e| e.into()))
    }
}
