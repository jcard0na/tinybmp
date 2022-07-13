#![warn(missing_docs)]

pub trait BmpReader {
    fn get(&self, image_offset: usize) -> Option<u8>;
}

#[derive(Debug)]
pub struct NullReader;

impl BmpReader for NullReader {
    fn get(&self, _image_offset: usize) -> Option<u8> {
        None
    }
}
