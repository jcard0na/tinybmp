use crate::{
    color_table::ColorTable,
    header::{Bpp, Header},
    raw_iter::RawPixels,
    reader::{BmpReader, SliceReader},
    ChannelMasks, ParseError,
};

const FIXED_PORTION_OF_BMP_HEADER_SIZE: usize = 14;

/// Low-level access to BMP image data.
///
/// This struct can be used to access the image data in a BMP file at a lower level than with the
/// [`Bmp`](crate::Bmp) struct. It doesn't do automatic color conversion and doesn't apply the color
/// table, if it is present in the BMP file. For images with a color table the iterator returned by
/// [`pixels`](Self::pixels) will instead return the color indices, that can be looked up manually
/// using the [`ColorTable`] struct.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RawBmp<'a, R = SliceReader<'a>> {
    /// Image header.
    header: Header,

    /// Color type.
    pub(crate) color_type: ColorType,

    /// Color table for color mapped images.
    color_table: Option<ColorTable<'a>>,

    /// Image data.
    image_data: &'a [u8],

    /// Image reader
    pub image_reader: Option<&'a R>,
}

impl<'a, R> RawBmp<'a, R> {
    /// Create a bitmap object from a byte slice.
    ///
    /// The created object keeps a shared reference to the input and does not dynamically allocate
    /// memory.
    pub fn from_slice(bytes: &'a [u8]) -> Result<Self, ParseError> {
        let (_remaining, (header, color_table)) = Header::parse(bytes)?;

        let color_type = ColorType::from_header(&header)?;

        let data_length = header.bytes_per_row() * header.image_size.height as usize;

        let image_data = &bytes
            .get(header.image_data_start..header.image_data_start + data_length)
            .ok_or(ParseError::UnexpectedEndOfFile)?;

        Ok(Self {
            header,
            color_type,
            color_table,
            image_data,
            image_reader: None,
        })
    }

    /// Create a bitmap object from a reader struct.
    ///
    /// Takes a reader and a buffer large enough to hold the full BMP header.
    ///
    // Implementation Note: I tried to keep the header_buffer inside the RawBmp
    // struct, but failed as rust does not (yet) support fields keeping
    // reference to other fields inside the same struct.
    pub fn from_reader(reader: &'a R, header_buffer: &'a mut [u8]) -> Result<Self, ParseError>
    where
        R: BmpReader<'a>,
    {
        let mut buffer = [0u8; FIXED_PORTION_OF_BMP_HEADER_SIZE];
        let _ = reader.read(0..FIXED_PORTION_OF_BMP_HEADER_SIZE, &mut buffer)?;
        let (_remaining, (_file_size, image_data_start)) = Header::parse_size(&buffer)?;

        if image_data_start > header_buffer.len() {
            return Err(ParseError::UnsupportedHeaderLength(image_data_start as u32));
        }

        let _ = reader.read(0..image_data_start, header_buffer)?;
        // Note: &*header_buffer changes the reference to immutable
        let (_remaining, (header, color_table)) = Header::parse(&*header_buffer)?;

        let color_type = ColorType::from_header(&header)?;

        Ok(Self {
            header,
            color_type,
            color_table,
            image_data: &[],
            image_reader: Some(reader),
        })
    }

    /// Returns the color table associated with the image.
    pub const fn color_table(&self) -> Option<&ColorTable<'a>> {
        self.color_table.as_ref()
    }

    /// Returns a slice containing the raw image data.
    pub const fn image_data(&self) -> &'a [u8] {
        self.image_data
    }

    /// Returns a reference to the BMP header.
    pub const fn header(&self) -> &Header {
        &self.header
    }

    /// Returns an iterator over the raw pixels in the image.
    ///
    /// The iterator returns the raw pixel colors as [`u32`] values.  To automatically convert the
    /// raw values into [`embedded_graphics`] color types use [`Bmp::pixels`](crate::Bmp::pixels)
    /// instead.
    pub fn pixels(&'a self) -> RawPixels<'a, R>
    where
        R: BmpReader<'a>,
    {
        RawPixels::new(self)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ColorType {
    Index1,
    Index4,
    Index8,
    Rgb555,
    Rgb565,
    Rgb888,
    Xrgb8888,
}

impl ColorType {
    pub(crate) fn from_header(header: &Header) -> Result<ColorType, ParseError> {
        Ok(match header.bpp {
            Bpp::Bits1 => ColorType::Index1,
            Bpp::Bits4 => ColorType::Index4,
            Bpp::Bits8 => ColorType::Index8,
            Bpp::Bits16 => {
                if let Some(masks) = header.channel_masks {
                    match masks {
                        ChannelMasks::RGB555 => ColorType::Rgb555,
                        ChannelMasks::RGB565 => ColorType::Rgb565,
                        _ => return Err(ParseError::UnsupportedChannelMasks),
                    }
                } else {
                    // According to the GDI docs the default 16 bpp color format is Rgb555 if no
                    // color masks are defined:
                    // https://docs.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapinfoheader
                    ColorType::Rgb555
                }
            }
            Bpp::Bits24 => ColorType::Rgb888,
            Bpp::Bits32 => {
                if let Some(masks) = header.channel_masks {
                    if masks == ChannelMasks::RGB888 {
                        ColorType::Xrgb8888
                    } else {
                        return Err(ParseError::UnsupportedChannelMasks);
                    }
                } else {
                    ColorType::Xrgb8888
                }
            }
        })
    }
}
