use core::marker::PhantomData;

use embedded_graphics::prelude::*;

use crate::{raw_pixels::RawPixels, reader::BmpReader, RawPixel};

/// Iterator over the pixels in a BMP image.
///
/// See the [`pixels`] method documentation for more information.
///
/// [`pixels`]: struct.Bmp.html#method.pixels
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Pixels<'a, 'b, C, R: BmpReader> {
    raw: RawPixels<'a, 'b, R>,
    color_type: PhantomData<C>,
}

impl<'a, 'b, C, R> Pixels<'a, 'b, C, R>
where
    R: BmpReader,
{
    pub(crate) fn new(raw: RawPixels<'a, 'b, R>) -> Self {
        Self {
            raw,
            color_type: PhantomData,
        }
    }
}

impl<C, R> Iterator for Pixels<'_, '_, C, R>
where
    C: PixelColor + From<<C as PixelColor>::Raw>,
    R: BmpReader,
{
    type Item = Pixel<C>;

    fn next(&mut self) -> Option<Self::Item> {
        let RawPixel { position, color } = self.raw.next()?;

        let color = if self.raw.raw_bmp.color_bpp().bits() <= 8 {
            // Return an empty iterator if no color table is present.
            let color_table = self.raw.raw_bmp.color_table()?;

            color_table
                .get(color)
                .unwrap_or_else(|| C::Raw::from_u32(0).into()) //TODO: how should invalid color indices be handled
        } else {
            C::Raw::from_u32(color).into()
        };

        Some(Pixel(position, color))
    }
}
