use std::error::Error;
use std::io::Read;
use std::result::Result;

use bson::RawDocumentBuf;

pub struct RawDocumentBufs<'reader, R: Read> {
    reader: &'reader mut R,
}

pub fn raw_document_bufs<R: Read>(reader: &mut R) -> RawDocumentBufs<R> {
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
        Some(RawDocumentBuf::from_bytes(bytes).map_err(|e| e.into()))
    }
}
