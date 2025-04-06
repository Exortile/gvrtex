/// Provides the internal implementation for a [`Iterator::next()`] function, catered to the pixel
/// block iterators.
///
/// This macro allows adding a block of statements on each iteration of a full block, which is
/// needed in [`PixelBlockIteratorExt`].
///
/// # Metavariables
///
/// * `$iter` - The iterator data. Should be a binding to [`PixelBlockIterator`]
/// * `$next_point` - The expression to use for returning the next point out of the iterator.
/// * `$each_block` - The block of statements that gets run on each full block iteration.
macro_rules! impl_pixelblockiterator {
    ($iter:ident, $next_point:expr, $each_block:block) => {
        {
            if $iter.y_block >= $iter.height {
                return None;
            }

            let next_point = $next_point;

            $iter.x += 1;
            if $iter.x == $iter.x_block_size {
                $iter.x = 0;
                $iter.y += 1;
            } else {
                return Some(next_point);
            }

            if $iter.y == $iter.y_block_size {
                $iter.y = 0;

                $each_block

                $iter.x_block += $iter.x_block_size;
            } else {
                return Some(next_point);
            }

            if $iter.x_block >= $iter.width {
                $iter.x_block = 0;
                $iter.y_block += $iter.y_block_size;
            }

            Some(next_point)
        }
    };
}

/// Iterates through an image of the given width and height in 4x4 blocks instead of singular
/// pixels. The iterator returns the x and y coordinate as a tuple on each iteration.
///
/// It works by iterating through a block row by row, before moving on to the next block, which it
/// also iterates through row by row until the end of the image.
pub struct PixelBlockIterator {
    width: u32,
    height: u32,
    x_block_size: u32,
    y_block_size: u32,

    x_block: u32,
    y_block: u32,
    x: u32,
    y: u32,
}

impl PixelBlockIterator {
    pub fn new(width: u32, height: u32, x_block_size: u32, y_block_size: u32) -> Self {
        Self {
            width,
            height,
            x_block_size,
            y_block_size,

            x_block: 0,
            y_block: 0,
            x: 0,
            y: 0,
        }
    }
}

impl Iterator for PixelBlockIterator {
    type Item = (u32, u32);

    /// Iterates over each pixel, returning the x and y coordinate of the next pixel as a tuple.
    fn next(&mut self) -> Option<Self::Item> {
        impl_pixelblockiterator!(self, (self.x_block + self.x, self.y_block + self.y), {})
    }
}

/// See [`PixelBlockIterator`] for specifics on how this iterator works.
///
/// This is an extension upon that iterator, that also returns the amount of blocks that have been
/// processed thus far, and the current column index (x coordinate) in the current block,
/// which some encodings need.
pub struct PixelBlockIteratorExt {
    iterator: PixelBlockIterator,
    blocks: u32,
}

impl PixelBlockIteratorExt {
    pub fn new(width: u32, height: u32, x_block_size: u32, y_block_size: u32) -> Self {
        Self {
            iterator: PixelBlockIterator::new(width, height, x_block_size, y_block_size),
            blocks: 0,
        }
    }
}

impl Iterator for PixelBlockIteratorExt {
    type Item = (u32, u32, u32, u32);

    /// Iterates over each pixel, returning the x and y coordinate of the next pixel as a tuple.
    fn next(&mut self) -> Option<Self::Item> {
        let iter = &mut self.iterator;
        impl_pixelblockiterator!(
            iter,
            (
                self.blocks,
                iter.x,
                iter.x_block + iter.x,
                iter.y_block + iter.y
            ),
            {
                self.blocks += 1;
            }
        )
    }
}
