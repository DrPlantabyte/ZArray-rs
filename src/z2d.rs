//! This module is used for storing 2-dimensional data arrays, and internally uses Z-index arrays
//! to improve data localization and alignment to the CPU cache-line fetches. In other words, use
//! this to improve performance for 2D data that is randomly accessed rather than raster scanned
//! or if your data processing makes heavy use of neighbor look-up in both the X and Y directions.
//! # How It Works
//! When you initialize a zarray::z2d::ZArray2D struct, it creates an array of 8x8 data patches,
//! using Z-curve indexing within that patch. When you call a getter or setter method, it finds the
//! corresponding data patch and then looks up (or sets) the data from within the patch. Since the
//! cache-line size on most CPUs is 64 bytes (and up to only 128 bytes on more exotic chips), the
//! 8x8 patch is sufficient localization for the majority of applications.
//! # Example Usage
//! An example of a simple blurring operation
//! ```
//! use zarray::z2d::ZArray2D;
//! let w = 800;
//! let h = 600;
//! let mut input = ZArray2D::new(w, h, 0i32);
//! let mut blurred = ZArray2D::new(w, h, 0i32);
//! for y in 0..h {
//!   for x in 0..w {
//!     let random_number = (((x*1009+1031)*y*1013+1051) % 10) as i32;
//!     input.set(x, y, random_number).unwrap();
//!   }
//! }
//! let radius: i32 = 2;
//! for y in radius..h as i32-radius {
//!   for x in radius..w as i32-radius {
//!     let mut sum = 0;
//!     for dy in -radius..radius+1 {
//!       for dx in -radius..radius+1 {
//!         sum += *input.bounded_get((x+dx) as isize, (y+dy) as isize).unwrap_or(&0);
//!       }
//!     }
//!     blurred.set(x as usize, y as usize, sum/((2*radius+1).pow(2))).unwrap();
//!   }
//! }
//! ```
// Z-order indexing in 2 dimensions

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use array_init::array_init;
use crate::LookUpError;

/// Private struct for holding an 8x8 data patch
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct Patch<T> {
	contents: [T; 64]
}

impl<T> Copy for Patch<T> where T: Copy{}
impl<T> Clone for Patch<T> where T: Clone{
	fn clone(&self) -> Self {
			Self{contents: self.contents.clone()}
	}
}
impl<T> PartialEq for Patch<T> where T: PartialEq{
	fn eq(&self, other: &Self) -> bool {
			self.contents.eq(&other.contents)
	}
}
impl<T> Eq for Patch<T> where T: Eq{}
impl<T> Hash for Patch<T> where T: Hash{
	fn hash<H: Hasher>(&self, state: &mut H) {
		for item in &self.contents {
			item.hash(state);
		}
	}
}


impl<T> Patch<T> {
	/// data patch getter
	/// # Parameters
	/// * **x** - x coord (only lowest 3 bits are used, rest of bits are ignored)
	/// * **y** - y coord (only lowest 3 bits are used, rest of bits are ignored)
	/// # Returns
	/// Returns a reference to the value stored in the patch at location (x & 0x07), (y & 0x07)
	fn get(&self, x: usize, y: usize) -> &T {
		// 3-bit x 3-bit
		return &self.contents[zorder_4bit_to_8bit(x as u8 & 0x07, y as u8 & 0x07) as usize];
	}
	/// data patch setter
	/// # Parameters
	/// * **x** - x coord (only lowest 3 bits are used, rest of bits are ignored)
	/// * **y** - y coord (only lowest 3 bits are used, rest of bits are ignored)
	/// * **new_val** - value to set
	fn set(&mut self, x: usize, y: usize, new_val: T) {
		// 3-bit x 3-bit
		let i = zorder_4bit_to_8bit(x as u8 & 0x07, y as u8 & 0x07) as usize;
		//let old_val = &self.contents[i];
		self.contents[i] = new_val;
		//return old_val;
	}
}

/// function for converting coordinate to index of data patch in the array of patches
fn patch_index(x: usize, y: usize, pwidth: usize) -> usize {
	return (x >> 3) + ((y >> 3) * (pwidth));
}

/// function for getting the coords represented by a patch
fn patch_coords(pwidth: usize, pindex: usize) -> [(usize, usize); 64] {
	let mut outbuffer = [(0usize, 0usize); 64];
	let bx = (pindex % pwidth) << 3;
	let by = (pindex / pwidth) << 3;
	for i in 0..64 {
		let bitmask = REVERSE_ZLUT[i];
		let dx = bitmask & 0b00000111u8;
		let dy = (bitmask >> 3u8) & 0b00000111u8;
		outbuffer[i] = (bx + dx as usize, by + dy as usize);
	}
	return outbuffer;
}

/// This is primary struct for z-indexed 2D arrays. Create new instances with
/// ZArray2D::new(x_size, y_size, initial_value)
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ZArray2D<T> {
	// for heap allocated data
	width: usize,
	height: usize,
	pwidth: usize,
	patches: Vec<Patch<T>>,
	_phantomdata: PhantomData<T>,
}

impl<T> Clone for ZArray2D<T> where T: Clone{
	fn clone(&self) -> Self {
			Self{
				width: self.width,
				height: self.height,
				pwidth: self.pwidth,
				patches: self.patches.clone(),
				_phantomdata: self._phantomdata
			}
	}
}
impl<T> PartialEq for ZArray2D<T> where T: PartialEq{
	fn eq(&self, other: &Self) -> bool {
		self.width == other.width
		&& self.height == other.height
		&& self.pwidth == other.pwidth
		&& self.patches == other.patches
	}
}
impl<T> Eq for ZArray2D<T> where T: Eq{}
impl<T> Hash for ZArray2D<T> where T: Hash{
	fn hash<H: Hasher>(&self, state: &mut H) {
		for patch in &self.patches {
			patch.hash(state);
		}
	}
}

impl<T> ZArray2D<T> where T: Default {
	/// Create a Z-index 2D array of values, initially filled with the default values
	/// # Parameters
	/// * **width** - size of this 2D array in the X dimension
	/// * **height** - size of this 2D array in the Y dimension
	/// # Returns
	/// Returns an initialized *ZArray2D* struct filled with default values
	pub fn new_with_default(width: usize, height: usize) -> ZArray2D<T> {
		let pwidth = ((width-1) >> 3) + 1;
		let pheight = ((height-1) >> 3) + 1;
		let patch_count = pwidth * pheight;
		let mut p = Vec::with_capacity(patch_count);
		for _ in 0..patch_count {
			let default_contents: [T; 64] = array_init(|_|T::default());
			p.push(Patch { contents: default_contents });
		}
		return ZArray2D { width, height, pwidth, patches: p, _phantomdata: PhantomData };
	}
}
impl<T> ZArray2D<T> where T: Copy {
	 /// Create a Z-index 2D array of values, initially filled with the provided default value
	/// # Parameters
	/// * **width** - size of this 2D array in the X dimension
	/// * **height** - size of this 2D array in the Y dimension
	/// * **default_val** - initial fill value (it must implement the Copy trait)
	/// # Returns
	/// Returns an initialized *ZArray2D* struct filled with *default_val*
	pub fn new(width: usize, height: usize, default_val: T) -> ZArray2D<T> {
		let pwidth = ((width-1) >> 3) + 1;
		let pheight = ((height-1) >> 3) + 1;
		let patch_count = pwidth * pheight;
		let mut p = Vec::with_capacity(patch_count);
		for _ in 0..patch_count {
			p.push(Patch { contents: [default_val; 64] });
		}
		return ZArray2D { width, height, pwidth, patches: p, _phantomdata: PhantomData };
	}
}
impl<T> ZArray2D<T> {
	/// Create a Z-index 2D array of values, initially filled with the provided constructor function
	/// # Parameters
	/// * **width** - size of this 2D array in the X dimension
	/// * **height** - size of this 2D array in the Y dimension
	/// * **constructor** - function which takes in the (X,Y) coords as a tuple and returns a value of type T
	/// # Returns
	/// Returns an initialized *ZArray2D* struct filled with *default_val*
	pub fn new_with_constructor(width: usize, height: usize, constructor: Fn((usize, usize)) -> T) -> ZArray2D<T> {
		let pwidth = ((width-1) >> 3) + 1;
		let pheight = ((height-1) >> 3) + 1;
		let init_width = pwidth << 3; // 
		let patch_count = pwidth * pheight;
		let mut p = Vec::with_capacity(patch_count);
		for pindex in 0..patch_count {
			let lookup_table = patch_coords(pwidth, pindex);
			let initial_contents: [T; 64] = array_init(|i| constructor(lookup_table[i]));
			p.push(Patch { contents: initial_contents });
		}
		return ZArray2D { width, height, pwidth, patches: p, _phantomdata: PhantomData };
	}

	/// Gets the (x, y) size of this 2D array
	/// # Returns
	/// Returns a tuple of (width, height) for this 2D array
	pub fn dimensions(&self) -> (usize, usize) {
		return (self.width, self.height);
	}

	/// Gets the X-dimension size (aka width) of this 2D array
	/// # Returns
	/// Returns the size in the X dimension
	pub fn xsize(&self) -> usize {
		return self.width;
	}


	/// Alias for `xsize()`
	/// # Returns
	/// Returns the size in the X dimension
	pub fn width(&self) -> usize {
		return self.xsize();
	}

	/// Gets the Y-dimension size (aka height) of this 2D array
	/// # Returns
	/// Returns the size in the Y dimension
	pub fn ysize(&self) -> usize {
		return self.height;
	}

	/// Alias for `ysize()`
	/// # Returns
	/// Returns the size in the Y dimension
	pub fn height(&self) -> usize {
		return self.ysize();
	}

	/// Gets a value from the 2D array, or returns a *LookUpError* if the provided coordinate
	/// is out of bounds. If you are using a default value for out-of-bounds coordinates,
	/// then you should use the *bounded_get(x, y)* method instead. If you want access to
	/// wrap-around (eg (-2, 0) equivalent to (width-2,0)), then use the *wrapped_get(x, y)*
	/// method.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// # Returns
	/// Returns a Result type that holds either the returned data value (as a reference) from
	/// the 2D array, or a *LookUpError* signalling that the coordinate is out of bounds
	pub fn get(&self, x: usize, y: usize) -> Result<&T, LookUpError> {
		if x < self.width && y < self.height {
			Ok(self.patches[patch_index(x, y, self.pwidth)].get(x, y))
		} else {
			Err(LookUpError { coord: vec![x, y], bounds: vec![self.width, self.height] })
		}
	}

	/// Sets a value in the 2D array, or returns a *LookUpError* if the provided coordinate
	/// is out of bounds. If you want out-of-bound coordinates to result in a no-op, then use
	/// the *bounded_set(x, y, val)* method instead. If you want access to wrap-around (eg
	/// (-2, 0) equivalent to (width-2,0)), then use the *wrapped_set(x, y, val)* method.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **new_val** - value to store in the 2D array at (x, y)
	/// # Returns
	/// Returns a Result type that is either empty or a *LookUpError* signalling that the
	/// coordinate is out of bounds
	pub fn set(&mut self, x: usize, y: usize, new_val: T) -> Result<(), LookUpError> {
		if x < self.width && y < self.height {
			Ok(self.patches[patch_index(x, y, self.pwidth)].set(x, y, new_val))
		} else {
			Err(LookUpError { coord: vec![x, y], bounds: vec![self.width, self.height] })
		}
	}

	/// Gets a value from the 2D array without bounds checking
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// # Returns
	/// Returns a data value (as a reference) from the 2D array
	pub fn get_unchecked(&self, x: usize, y: usize) -> &T {
		return self.patches[patch_index(x, y, self.pwidth)].get(x, y);
	}

	/// Sets a value in the 2D array without bounds checking
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **new_val** - value to store in the 2D array at (x, y)
	pub fn set_unchecked(&mut self, x: usize, y: usize, new_val: T) {
		self.patches[patch_index(x, y, self.pwidth)].set(x, y, new_val);
	}

	/// Gets a value from the 2D array, wrapping around the X and Y axese when the coordinates
	/// are negative or outside the size of this 2D array. Good for when you want tiling
	/// behavior.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// # Returns
	/// Returns a reference to the data stored at the provided coordinate (wrapping both x
	/// and y dimensions)
	pub fn wrapped_get(&self, x: isize, y: isize) -> &T {
		let x = (self.width as isize + (x % self.width as isize)) as usize % self.width;
		let y = (self.height as isize + (y % self.height as isize)) as usize % self.height;
		return &self.patches[patch_index(x, y, self.pwidth)].get(x, y);
	}

	/// Sets a value in the 2D array at the provided coordinate, wrapping the X and Y axese
	/// if the coordinate is negative or out of bounds.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **new_val** - value to store in the 2D array at (x, y), wrapping around both the x
	/// and y dimensions
	pub fn wrapped_set(&mut self, x: isize, y: isize, new_val: T) {
		let x = (self.width as isize + (x % self.width as isize)) as usize % self.width;
		let y = (self.height as isize + (y % self.height as isize)) as usize % self.height;
		self.patches[patch_index(x, y, self.pwidth)].set(x, y, new_val);
	}

	/// Gets a value from the 2D array as an Option that is None if the coordinate
	/// is out of bounds.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// # Returns
	/// Returns an Option type that holds either the returned data value (as a reference) from
	/// the 2D array, or *None* signalling that the coordinate is out of bounds (which can be
	/// combined with .unwrap_or(default_value) to implement an out-of-bounds default)
	pub fn bounded_get(&self, x: isize, y: isize) -> Option<&T> {
		if x >= 0 && y >= 0 && x < self.width as isize && y < self.height as isize {
			return Some(&self.patches[patch_index(x as usize, y as usize, self.pwidth)]
				.get(x as usize, y as usize));
		} else {
			return None;
		}
	}

	/// Sets a value in the 2D array if and only if the provided coordinate is in bounds.
	/// Otherwise this method does nothing if the coordiante is out of bounds.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **new_val** - value to store int eh 2D array at (x, y)
	pub fn bounded_set(&mut self, x: isize, y: isize, new_val: T) {
		if x >= 0 && y >= 0 && x < self.width as isize && y < self.height as isize {
			self.patches[patch_index(x as usize, y as usize, self.pwidth)]
				.set(x as usize, y as usize, new_val);
		} else {
			// no-op
		}
	}

	/// Fills a region of this 2D array with a given value, or returns a *LookUpError* if the
	/// provided coordinates go out of bounds. If you just want to ignore any
	/// out-of-bounds coordinates, then you should use the *bounded_fill(x1, y1, x2, y2)*
	/// method instead. If you want access to wrap-around (eg (-2, 0) equivalent to
	/// (width-2,0)), then use the *wrapped_fill(x, y)* method.
	/// # Parameters
	/// * **x1** - the first x dimension coordinate (inclusive)
	/// * **y1** - the first y dimension coordinate (inclusive)
	/// * **x2** - the second x dimension coordinate (exclusive)
	/// * **y2** - the second y dimension coordinate (exclusive)
	/// * **new_val** - value to store in the 2D array in the bounding box defined by
	/// (x1, y1) -> (x2, y2)
	/// # Returns
	/// Returns a Result type that is either empty or a *LookUpError* signalling that a
	/// coordinate is out of bounds
	pub fn fill(&mut self, x1: usize, y1: usize, x2: usize, y2: usize, new_val: T)
				-> Result<(), LookUpError> {
		for y in y1..y2 {
			for x in x1..x2 {
				self.set(x, y, new_val)?;
			}
		}
		Ok(())
	}

	/// Fills a region of this 2D array with a given value, wrapping the axese when
	/// coordinates go out of bounds.
	/// # Parameters
	/// * **x1** - the first x dimension coordinate (inclusive)
	/// * **y1** - the first y dimension coordinate (inclusive)
	/// * **x2** - the second x dimension coordinate (exclusive)
	/// * **y2** - the second y dimension coordinate (exclusive)
	/// * **new_val** - value to store in the 2D array in the bounding box defined by
	/// (x1, y1) -> (x2, y2) with wrapped axese
	pub fn wrapped_fill(&mut self, x1: isize, y1: isize, x2: isize, y2: isize, new_val: T) {
		for y in y1..y2 {
			for x in x1..x2 {
				self.wrapped_set(x, y, new_val);
			}
		}
	}

	/// Fills a region of this 2D array with a given value, ignoring any
	/// coordinates that go out of bounds.
	/// # Parameters
	/// * **x1** - the first x dimension coordinate (inclusive)
	/// * **y1** - the first y dimension coordinate (inclusive)
	/// * **x2** - the second x dimension coordinate (exclusive)
	/// * **y2** - the second y dimension coordinate (exclusive)
	/// * **new_val** - value to store in the 2D array in the bounding box defined by
	/// (x1, y1) -> (x2, y2)
	pub fn bounded_fill(&mut self, x1: isize, y1: isize, x2: isize, y2: isize, new_val: T) {
		for y in y1..y2 {
			for x in x1..x2 {
				self.bounded_set(x, y, new_val);
			}
		}
	}

	/// Creates an iterator that iterates through the 2D array in Z-order
	/// # Returns
	/// A new ZArray2DIterator instance
	pub fn iter(&self) -> ZArray2DIterator<T> {
		ZArray2DIterator::new(self)
	}

}

#[test]
fn check_patch_count_2d() {
	let arr = ZArray2D::new(1, 1, 0u8);
	assert_eq!(arr.patches.len(), 1, "Allocated wrong number of patches for array of size {}x{}", arr.width, arr.height);
	let arr = ZArray2D::new(8, 8, 0u8);
	assert_eq!(arr.patches.len(), 1, "Allocated wrong number of patches for array of size {}x{}", arr.width, arr.height);
	let arr = ZArray2D::new(9, 8, 0u8);
	assert_eq!(arr.patches.len(), 2, "Allocated wrong number of patches for array of size {}x{}", arr.width, arr.height);
	let arr = ZArray2D::new(8, 9, 0u8);
	assert_eq!(arr.patches.len(), 2, "Allocated wrong number of patches for array of size {}x{}", arr.width, arr.height);
	let arr = ZArray2D::new(9, 9, 0u8);
	assert_eq!(arr.patches.len(), 4, "Allocated wrong number of patches for array of size {}x{}", arr.width, arr.height);
}

/// This struct is used by `ZArray2DIterator` to present values to the consumer of the
/// iterator
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ZArray2DIteratorItem <T> {
	/// x-dimension coordinate
	pub x: usize,
	/// y-dimension coordinate
	pub y: usize,
	/// value at this coordinate
	pub value: T
}


/// private state management enum
enum IterState {
	Start, Processing, Done
}
/// Iterator that iterates through the array
pub struct ZArray2DIterator<'a, T: Copy> {
	/// array to iterate over
	array: &'a ZArray2D<T>,
	patch: usize,
	index: usize,
	state: IterState
}

impl<'a, T: Copy> ZArray2DIterator<'a, T> {
	fn new(array: &'a ZArray2D<T>) -> ZArray2DIterator<'a, T> {
		if array.width == 0 || array.height == 0 {
			ZArray2DIterator{array, patch: 0, index: 0, state: IterState::Done} // make a "done" iterator for empty arrays
		} else {
			ZArray2DIterator{array, patch: 0, index: 0, state: IterState::Start}
		}
	}
}

impl<'a, T: Copy> Iterator for ZArray2DIterator<'a, T> {
	type Item = ZArray2DIteratorItem<T>;

	fn next(&mut self) -> Option<Self::Item> {
		match &self.state {
			IterState::Done=> None,
			IterState::Start=> {
				self.state = IterState::Processing;
				Some(ZArray2DIteratorItem{x: 0, y: 0, value: self.array.patches[0].contents[0]})
			},
			IterState::Processing => {
				let mut x ; let mut y ;
				loop {
					self.index += 1;
					if self.index >= 64 {
						self.index  = 0;
						self.patch += 1;
					}
					let yx_lower_pits = REVERSE_ZLUT[self.index];
					x = ((self.patch % self.array.pwidth) << 3) | (yx_lower_pits & 0x07) as usize;
					y = ((self.patch / self.array.pwidth) << 3) | ((yx_lower_pits >> 3) & 0x07) as usize;
					if x < self.array.width && y < self.array.height{
						break;
					}
					if self.patch >= self.array.patches.len() {
						self.state = IterState::Done;
						return None;
					}
				}
				Some(ZArray2DIteratorItem{x, y, value: self.array.patches[self.patch].contents[self.index]})
			}
		}
	}
}

/// Used for Z-index look-up
const ZLUT: [u8; 16] = [
	0b00000000,
	0b00000001,
	0b00000100,
	0b00000101,
	0b00010000,
	0b00010001,
	0b00010100,
	0b00010101,
	0b01000000,
	0b01000001,
	0b01000100,
	0b01000101,
	0b01010000,
	0b01010001,
	0b01010100,
	0b01010101
];

/// used by iterators for fast conversion from internal index to X, Y. Each number is 0byyyxxx
const REVERSE_ZLUT: [u8; 64] = [
	0 ,  1,  8,  9,  2,  3, 10, 11, 16, 17, 24, 25, 18, 19, 26, 27,
	4 ,  5, 12, 13,  6,  7, 14, 15, 20, 21, 28, 29, 22, 23, 30, 31,
	32, 33, 40, 41, 34, 35, 42, 43, 48, 49, 56, 57, 50, 51, 58, 59,
	36, 37, 44, 45, 38, 39, 46, 47, 52, 53, 60, 61, 54, 55, 62, 63
];

/// General purpose Z-index function to convert a two-dimensional coordinate into a localized
/// one-dimensional coordinate
/// # Parameters
/// * **x** - x dimension coordinate *(ONLY THE LOWER 4 BITS WILL BE USED!)*
/// * **y** - y dimension coordinate *(ONLY THE LOWER 4 BITS WILL BE USED!)*
/// # Returns
/// Z-curve index for use as an index in a linear array meant to hold 2D data. In other words,
/// given the binary numbers X=0b0000xxxx and Y=0b0000yyyy, this method will return 0byxyxyxyx.
pub fn zorder_4bit_to_8bit(x: u8, y: u8) -> u8 {
	let x_bits = ZLUT[(x & 0x0F) as usize];
	let y_bits = ZLUT[(y & 0x0F) as usize] << 1;
	return y_bits | x_bits;
}

/// General purpose Z-index function to convert a two-dimensional coordinate into a localized
/// one-dimensional coordinate
/// # Parameters
/// * **x** - x dimension coordinate (8 bits)
/// * **y** - y dimension coordinate (8 bits)
/// # Returns
/// Z-curve index for use as an index in a linear array meant to hold 2D data. In other words,
/// given the binary numbers Y=0b0000xxxx and Y=0b0000yyyy, this method will return 0byxyxyxyx.
pub fn zorder_8bit_to_16bit(x:u8, y:u8) -> u16 {
	return ((zorder_4bit_to_8bit(x >> 4, y >> 4) as u16) << 8) | zorder_4bit_to_8bit(x, y) as u16
}

/// General purpose Z-index function to convert a two-dimensional coordinate into a localized
/// one-dimensional coordinate
/// # Parameters
/// * **x** - x dimension coordinate (16 bits)
/// * **y** - y dimension coordinate (16 bits)
/// # Returns
/// Z-curve index for use as an index in a linear array meant to hold 2D data. In other words,
/// given the binary numbers Y=0b0000xxxx and Y=0b0000yyyy, this method will return 0byxyxyxyx.
pub fn zorder_16bit_to_32bit(x:u16, y:u16) -> u32 {
	return ((zorder_8bit_to_16bit((x & 0xFF) as u8, (y & 0xFF) as u8) as u32) << 16) | zorder_8bit_to_16bit((x >> 8) as u8, (y >> 8) as u8) as u32
}
