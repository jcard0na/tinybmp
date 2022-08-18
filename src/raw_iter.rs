use core::{iter, marker::PhantomData, slice};

use embedded_graphics::{
    iterator::raw::RawDataSlice,
    pixelcolor::raw::{LittleEndian, RawU1, RawU16, RawU24, RawU32, RawU4, RawU8},
    prelude::*,
    primitives::{rectangle, Rectangle},
};

use crate::{
    header::{Bpp, RowOrder},
    raw_bmp::RawBmp,
};

/// Iterator over raw pixel colors.
#[allow(missing_debug_implementations)]
pub struct RawColors<'a, I, R>
where
    RawDataSlice<'a, I, LittleEndian>: IntoIterator<Item = I>,
{
    rows: slice::ChunksExact<'a, u8>,
    row_order: RowOrder,
    current_row: iter::Take<<RawDataSlice<'a, I, LittleEndian> as IntoIterator>::IntoIter>,
    width: usize,
    reader: PhantomData<R>,
}

impl<'a, I, R> RawColors<'a, I, R>
where
    RawDataSlice<'a, I, LittleEndian>: IntoIterator<Item = I>,
{
    pub(crate) fn new(raw_bmp: &'a RawBmp<'a, R>) -> Self {
        let header = raw_bmp.header();

        let width = header.image_size.width as usize;

        Self {
            rows: raw_bmp.image_data().chunks_exact(header.bytes_per_row()),
            row_order: raw_bmp.header().row_order,
            current_row: RawDataSlice::new(&[]).into_iter().take(0),
            width,
            reader: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for RawColors<'a, I, R>
where
    RawDataSlice<'a, I, LittleEndian>: IntoIterator<Item = I>,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        self.current_row.next().or_else(|| {
            let next_row = match self.row_order {
                RowOrder::TopDown => self.rows.next(),
                RowOrder::BottomUp => self.rows.next_back(),
            }?;

            self.current_row = RawDataSlice::new(next_row).into_iter().take(self.width);

            self.current_row.next()
        })
    }
}

enum DynamicRawColors<'a, R> {
    Bpp1(RawColors<'a, RawU1, R>),
    Bpp4(RawColors<'a, RawU4, R>),
    Bpp8(RawColors<'a, RawU8, R>),
    Bpp16(RawColors<'a, RawU16, R>),
    Bpp24(RawColors<'a, RawU24, R>),
    Bpp32(RawColors<'a, RawU32, R>),
}

/// Iterator over individual BMP pixels.
///
/// Each pixel is returned as a `u32` regardless of the bit depth of the source image.
#[allow(missing_debug_implementations)]
pub struct RawPixels<'a, R> {
    colors: DynamicRawColors<'a, R>,
    points: rectangle::Points,
    reader: PhantomData<R>,
}

impl<'a, R> RawPixels<'a, R> {
    pub(crate) fn new(raw_bmp: &'a RawBmp<'a, R>) -> Self {
        let header = raw_bmp.header();

        let colors = match header.bpp {
            Bpp::Bits1 => DynamicRawColors::Bpp1(RawColors::new(raw_bmp)),
            Bpp::Bits4 => DynamicRawColors::Bpp4(RawColors::new(raw_bmp)),
            Bpp::Bits8 => DynamicRawColors::Bpp8(RawColors::new(raw_bmp)),
            Bpp::Bits16 => DynamicRawColors::Bpp16(RawColors::new(raw_bmp)),
            Bpp::Bits24 => DynamicRawColors::Bpp24(RawColors::new(raw_bmp)),
            Bpp::Bits32 => DynamicRawColors::Bpp32(RawColors::new(raw_bmp)),
        };
        let points = Rectangle::new(Point::zero(), header.image_size).points();

        Self {
            colors,
            points,
            reader: PhantomData,
        }
    }
}

impl<R> Iterator for RawPixels<'_, R> {
    type Item = RawPixel;

    fn next(&mut self) -> Option<Self::Item> {
        let color = match &mut self.colors {
            DynamicRawColors::Bpp1(colors) => colors.next().map(|r| u32::from(r.into_inner())),
            DynamicRawColors::Bpp4(colors) => colors.next().map(|r| u32::from(r.into_inner())),
            DynamicRawColors::Bpp8(colors) => colors.next().map(|r| u32::from(r.into_inner())),
            DynamicRawColors::Bpp16(colors) => colors.next().map(|r| u32::from(r.into_inner())),
            DynamicRawColors::Bpp24(colors) => colors.next().map(|r| u32::from(r.into_inner())),
            DynamicRawColors::Bpp32(colors) => colors.next().map(|r| u32::from(r.into_inner())),
        }?;

        let position = self.points.next()?;

        Some(RawPixel { position, color })
    }
}

/// Pixel with raw pixel color stored as a `u32`.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
pub struct RawPixel {
    /// The position relative to the top left corner of the image.
    pub position: Point,

    /// The raw pixel color.
    pub color: u32,
}

impl RawPixel {
    /// Creates a new raw pixel.
    pub const fn new(position: Point, color: u32) -> Self {
        Self { position, color }
    }
}
