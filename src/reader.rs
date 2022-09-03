use core::cell::{Ref, RefCell};
use core::ops::Range;
use core::usize::MAX;

/// BMP file reader
///
/// Traits defining read, iterator and error interfaces to allow loading image
/// data from sources other than slices in memory.
///
/// A reader implementation the reads from slices is provided for guidance and
/// testing.

/// Helper trait to load BMP images from files.
pub trait BmpReader<'a>
where
    <Self as BmpReader<'a>>::IntoIter: DoubleEndedIterator<Item = Ref<'a, [u8]>>,
{
    /// Iterator that will be returned by chunks_exact()
    type IntoIter;

    /// Internal buffer used to store a single image row.
    const INTERNAL_BUFFER_SIZE: usize;

    /// Read a chunk from file into the provided buffer.
    fn read(&self, positions: Range<usize>, buffer: &mut [u8]) -> Result<(), BmpReaderError>;

    /// Read a chunk from file into internal buffer
    fn buffered_read(&self, positions: Range<usize>) -> Result<Ref<'_, [u8]>, BmpReaderError>;

    /// Returns a double ended iterator that can iterate in chunks of size
    /// `stride`
    fn chunks_exact(&'a self, stride: usize) -> Result<Self::IntoIter, BmpReaderError>;
}

/// BmpReader errors
#[derive(Copy, Clone, Debug, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub enum BmpReaderError {
    /// Error reading from the data source
    ReadError,
    /// Requested chunk larger than reader internal buffer
    RequestedChunkTooLarge,
    /// Given buffer is too small for requested read operation
    BufferTooSmall,
    /// This instance of the reader is null
    NullReader,
}

pub trait BmpReaderChunkIterator
where
    Self: DoubleEndedIterator,
{
}

// Implementation of the above traits in a reader for memory slices follow

/// An implementation of the BmpReader that reads from a [u8] slice.  This is
/// the default reader.
///
/// Useful to compare implementation of from_reader() with from_slice()
#[derive(Clone, Debug, PartialEq)]
pub struct SliceReader<'a> {
    image_data: &'a [u8],
    buffer: RefCell<[u8; SliceReader::INTERNAL_BUFFER_SIZE]>,
}

impl<'a> BmpReader<'a> for SliceReader<'a> {
    type IntoIter = SliceReaderIterator<'a>;

    const INTERNAL_BUFFER_SIZE: usize = 200;

    fn read(&self, positions: Range<usize>, buffer: &mut [u8]) -> Result<(), BmpReaderError> {
        let read_size = positions.end - positions.start;
        if read_size > buffer.len() {
            return Err(BmpReaderError::BufferTooSmall);
        }

        // Note: Here is where the I/O operation would happen on other implementations
        // of BmpReader
        let _ = &buffer[0..read_size].copy_from_slice(&self.image_data[positions]);
        Ok(())
    }

    fn chunks_exact(&'a self, stride: usize) -> Result<Self::IntoIter, BmpReaderError> {
        if stride > Self::INTERNAL_BUFFER_SIZE {
            return Err(BmpReaderError::RequestedChunkTooLarge);
        }
        Ok(SliceReaderIterator {
            reader: self,
            stride,
            // Note advance() will set these indices correctly before
            // get() is invoked.
            // rindex will start at file_size and end at MAX
            // index will start at  MAX       and end at file_size
            index: MAX,
            rindex: self.image_data.len(),
        })
    }

    fn buffered_read(&self, positions: Range<usize>) -> Result<Ref<'_, [u8]>, BmpReaderError> {
        let mut read_size = positions.end - positions.start;
        // temporary mutability design pattern
        {
            let mut buffer = self.buffer.borrow_mut();
            let mut positions = positions;
            if read_size > buffer.len() {
                read_size -= buffer.len() - read_size;
                positions = positions.start..(positions.end - buffer.len() + read_size);
            }
            let _ = &buffer[0..read_size].copy_from_slice(&self.image_data[positions]);
        }
        Ok(Ref::map(self.buffer.borrow(), |s| &s[0..read_size]))
    }
}

#[derive(Debug)]
pub struct SliceReaderIterator<'a> {
    reader: &'a SliceReader<'a>,
    index: usize,
    stride: usize,
    rindex: usize,
}

impl<'a> Iterator for SliceReaderIterator<'a> {
    type Item = Ref<'a, [u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == MAX {
            self.index = 0;
        } else {
            self.index += self.stride;
        }
        if self.index + self.stride - 1 >= self.rindex {
            return None;
        }
        // Updating the internal buffer from I/O operation will happen here
        // For SliceReaderIterator, we just copy a chunk from the Reader's slice
        let range = self.index..self.index + self.stride;
        self.reader.buffered_read(range).ok()
        // TODO: handle read errors
    }
}

impl<'a> DoubleEndedIterator for SliceReaderIterator<'a> {
    fn next_back(&mut self) -> Option<Ref<'a, [u8]>> {
        if self.rindex == 0 {
            self.rindex = MAX;
        } else {
            self.rindex -= self.stride;
        }
        if (self.index != MAX && self.index >= self.rindex - self.stride + 1) || self.rindex == MAX
        {
            return None;
        }
        let range = self.rindex..self.rindex + self.stride;
        // Updating the internal buffer from I/O operation will happen here
        self.reader.buffered_read(range).ok()
        // TODO: handle read errors
    }
}

impl<'a> SliceReader<'a> {
    /// Creates a new slice reader from a given slice containing a BMP image
    pub fn new(slice: &'a [u8]) -> Self {
        SliceReader {
            image_data: slice,
            buffer: RefCell::new([0u8; SliceReader::INTERNAL_BUFFER_SIZE]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_iter() {
        let mut image_data: [u8; 1000] = [0u8; 1000];
        let _count = image_data
            .iter_mut()
            .enumerate()
            .map(|(i, v)| *v = i as u8)
            .count();
        let reader = SliceReader::new(&image_data[..]);
        let iter = &mut reader.chunks_exact(1).unwrap();
        assert_eq!(iter.next().unwrap()[0], 0u8);
        assert_eq!(iter.next().unwrap()[0], 1u8);
        assert_eq!(iter.next().unwrap()[0], 2u8);
        assert_eq!(iter.next().unwrap()[0], 3u8);
    }

    #[test]
    fn test_stride() {
        let mut image_data: [u8; 1000] = [0u8; 1000];
        let _count = image_data
            .iter_mut()
            .enumerate()
            .map(|(i, v)| *v = i as u8)
            .count();
        let reader = SliceReader::new(&image_data[..]);
        let iter = &mut reader.chunks_exact(2).unwrap();
        assert_eq!(iter.next().unwrap()[..], [0u8, 1u8][..]);
        assert_eq!(iter.next().unwrap()[..], [2u8, 3u8][..]);
        assert_eq!(iter.next().unwrap()[..], [4u8, 5u8][..]);
    }

    #[test]
    fn test_next_back() {
        let mut image_data = [0u8; 256];
        let _count = image_data
            .iter_mut()
            .enumerate()
            .map(|(i, v)| *v = i as u8)
            .count();
        let reader = SliceReader::new(&image_data[..]);
        let iter = &mut reader.chunks_exact(2).unwrap();
        assert_eq!(iter.next_back().unwrap()[..], [254u8, 255u8][..]);
        assert_eq!(iter.next_back().unwrap()[..], [252u8, 253u8][..]);
        assert_eq!(iter.next_back().unwrap()[..], [250u8, 251u8][..]);
    }

    #[test]
    fn test_chunk_reader() {
        let mut image_data: [u8; 1000] = [0u8; 1000];
        let _count = image_data
            .iter_mut()
            .enumerate()
            .map(|(i, v)| *v = i as u8)
            .count();
        let reader = SliceReader::new(&image_data[..]);
        let mut buffer = [0u8; 3];
        assert_eq!(reader.read(2..5, &mut buffer), Ok(()));
        assert_eq!(buffer, [2u8, 3u8, 4u8]);
        assert_eq!(
            reader.read(2..50, &mut buffer),
            Err(BmpReaderError::BufferTooSmall)
        );
    }
}
