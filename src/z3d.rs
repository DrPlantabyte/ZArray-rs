//! This module is used for storing 3-dimensional data arrays, and internally uses Z-index arrays
//! to improve data localization and alignment to the CPU cache-line fetches. In other words, use
//! this to improve performance for 3D data that is randomly accessed rather than raster scanned
//! or if your data processing makes heavy use of neighbor look-up in the X, Y, and Z directions.
//! # How It Works
//! When you initialize a zarray::z3d::ZArray3D struct, it creates an array of 8x8x8 data patches
//! (512 total elements per patch), using Z-curve indexing within that patch. When you call a
//! getter or setter method, it finds the corresponding data patch and then looks up (or sets) the
//! data from within the patch.
//! # Example Usage
//! The following example could be used as part of an erosion simulation:
//! ```
//! use zarray::z3d::ZArray3D;
//! let width = 100;
//! let length = 200;
//! let depth = 25;
//! let air = 0f32;
//! let soil_hardness = 1f32;
//! let rock_hardness = 8f32;
//! let drip_power = 1.5f32;
//! let iterations = 12;
//! let mut map = ZArray3D::new(width, length, depth, air);
//! map.fill(0,0,5, width,length,depth, soil_hardness).unwrap();
//! map.fill(0,0,15, width,length,depth, rock_hardness).unwrap();
//! for boulder in [(34,88,6), (66,122,9), (11,154,5), (35,93,8), (72,75,12)]{
//!   map.set(boulder.0, boulder.1, boulder.2, rock_hardness).unwrap();
//! }
//! for _ in 0..iterations{
//!   for x in 0..width{for y in 0..length{
//!     let mut drip = drip_power;
//!     let mut z = 0;
//!     while drip > 0f32 {
//!       let h = *map.bounded_get(x as isize, y as isize, z).unwrap_or(&100f32);
//!       if h > drip {
//!         map.bounded_set(x as isize, y as isize, z, h - drip);
//!         drip = 0.;
//!       } else {
//!         map.bounded_set(x as isize, y as isize, z, 0.);
//!         drip -= h;
//!       }
//!       z += 1;
//!     }
//!   }}
//! }
//! ```
// Z-order indexing in 3 dimensions

use core::hash::{Hash, Hasher};
use core::borrow::Borrow;
use core::marker::PhantomData;
use array_init::array_init;
use crate::LookUpError;


/// Private struct for holding an 8x8x8 data patch
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct Patch<T>{
	contents: [T;512]
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
	/// * **z** - z coord (only lowest 3 bits are used, rest of bits are ignored)
	/// # Returns
	/// Returns a reference to the value stored in the patch at location (x, y, z) (lowest 3
	/// bits only)
	fn get(&self, x: usize, y:usize, z:usize) -> &T {
		// 3-bit x 3-bit x 3-bit
		return &self.contents[zorder_4bit_to_12bit(
			x as u8 & 0x07, y as u8 & 0x07, z as u8 & 0x07) as usize];
	}
	/// data patch setter
	/// # Parameters
	/// * **x** - x coord (only lowest 3 bits are used, rest of bits are ignored)
	/// * **y** - y coord (only lowest 3 bits are used, rest of bits are ignored)
	/// * **z** - z coord (only lowest 3 bits are used, rest of bits are ignored)
	/// * **new_val** - value to set at (x,y,z)
	fn set(&mut self, x: usize, y:usize, z:usize, new_val: T) {
		// 3-bit x 3-bit
		let i = zorder_4bit_to_12bit(
			x as u8 & 0x07, y as u8 & 0x07, z as u8 & 0x07) as usize;
		self.contents[i] = new_val;
	}
}

/// function for converting coordinate to index of data patch in the array of patches
fn patch_index(x: usize, y:usize, z:usize, pxsize: usize, pysize: usize) -> usize{
	return (x >> 3) + pxsize * ((y >> 3) + (pysize * (z >> 3)));
}

/// function for getting the coords represented by a patch
fn patch_coords(pxsize: usize, pysize: usize, pindex: usize) -> [(usize, usize, usize); 512] {
	let mut outbuffer = [(0usize, 0usize, 0usize); 512];
	let bx = (pindex % pxsize) << 3;
	let by = ((pindex / pxsize) % pysize) << 3;
	let bz = (pindex / (pxsize * pysize)) << 3;
	for i in 0..512 {
		let bitmask = REVERSE_ZLUT[i];
		let dx = bitmask & 0b00000111u16;
		let dy = (bitmask >> 3u16) & 0b00000111u16;
		let dz = (bitmask >> 6u16) & 0b00000111u16;
		outbuffer[i] = (bx + dx as usize, by + dy as usize, bz + dz as usize);
	}
	return outbuffer;
}

/// This is primary struct for z-indexed 3D arrays. Create new instances with
/// ZArray3D::new(x_size, y_size, z_size, initial_value)
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ZArray3D<T> {
	// for heap allocated data
	xsize: usize,
	ysize: usize,
	zsize: usize,
	pxsize: usize,
	pysize: usize,
	patches: Vec<Patch<T>>,
	_phantomdata: PhantomData<T>,
}

impl<T> Clone for ZArray3D<T> where T: Clone{
	fn clone(&self) -> Self {
			Self{
				xsize: self.xsize,
				ysize: self.ysize,
				zsize: self.zsize,
				pxsize: self.pxsize,
				pysize: self.pysize,
				patches: self.patches.clone(),
				_phantomdata: self._phantomdata
			}
	}
}
impl<T> PartialEq for ZArray3D<T> where T: PartialEq{
	fn eq(&self, other: &Self) -> bool {
		self.xsize == other.xsize
		&& self.ysize == other.ysize
		&& self.zsize == other.zsize
		&& self.pxsize == other.pxsize
		&& self.pysize == other.pysize
		&& self.patches == other.patches
	}
}
impl<T> Eq for ZArray3D<T> where T: Eq{}
impl<T> Hash for ZArray3D<T> where T: Hash{
	fn hash<H: Hasher>(&self, state: &mut H) {
		for patch in &self.patches {
			patch.hash(state);
		}
	}
}

impl<T> ZArray3D<T> where T: Default {
	/// Create a Z-index 3D array of values, initially filled with the default values
	/// # Parameters
	/// * **xsize** - size of this 3D array in the X dimension
	/// * **ysize** - size of this 3D array in the Y dimension
	/// * **zsize** - size of this 3D array in the Z dimension
	/// # Returns
	/// Returns an initialized *ZArray2D* struct filled with default values
	pub fn new_with_default(xsize: usize, ysize: usize, zsize: usize) -> ZArray3D<T> {
		let px = ((xsize-1) >> 3) + 1;
		let py = ((ysize-1) >> 3) + 1;
		let pz = ((zsize-1) >> 3) + 1;
		let patch_count = px * py * pz;
		let mut p = Vec::with_capacity(patch_count);
		for _ in 0..patch_count {
			let default_contents: [T; 512] = array_init(|_|T::default());
			p.push(Patch { contents: default_contents });
		}
		return ZArray3D { xsize, ysize, zsize, pxsize: px, pysize: py,
			patches: p, _phantomdata: PhantomData};
	}
}

impl<T> ZArray3D<T> where T: Copy {
	/// Create a Z-index 3D array of values, initially filled with the provided default value
	/// # Parameters
	/// * **xsize** - size of this 3D array in the X dimension
	/// * **ysize** - size of this 3D array in the Y dimension
	/// * **zsize** - size of this 3D array in the Z dimension
	/// * **default_val** - initial fill value (if a struct type, then it must implement the
	/// Copy trait)
	/// # Returns
	/// Returns an initialized *ZArray3D* struct filled with *default_val*
	pub fn new(xsize: usize, ysize: usize, zsize: usize, default_val: T) -> ZArray3D<T>{
		let px = ((xsize-1) >> 3) + 1;
		let py = ((ysize-1) >> 3) + 1;
		let pz = ((zsize-1) >> 3) + 1;
		let patch_count = px * py * pz;
		let mut p = Vec::with_capacity(patch_count);
		for _ in 0..patch_count{
			p.push(Patch{contents: [default_val; 512]});
		}
		return ZArray3D { xsize, ysize, zsize, pxsize: px, pysize: py,
			patches: p, _phantomdata: PhantomData};
	}
}

impl<T> ZArray3D<T> where T: Clone {
	
	/// Fills a region of this 3D array with a given value, or returns a *LookUpError* if the
	/// provided coordinates go out of bounds. If you just want to ignore any
	/// out-of-bounds coordinates, then you should use the
	/// *bounded_fill(x1, y1, z1, x2, y2, z2)*
	/// method instead. If you want access to wrap-around (eg (-2, 0, 1) equivalent to
	/// (width-2, 0, 1)), then use the *wrapped_fill(x, y, z)* method.
	/// # Parameters
	/// * **x1** - the first x dimension coordinate (inclusive)
	/// * **y1** - the first y dimension coordinate (inclusive)
	/// * **z1** - the first z dimension coordinate (inclusive)
	/// * **x2** - the second x dimension coordinate (exclusive)
	/// * **y2** - the second y dimension coordinate (exclusive)
	/// * **z2** - the second z dimension coordinate (exclusive)
	/// * **new_val** - value to store in the 2D array in the bounding box defined by
	/// (x1, y1, z1) -> (x2, y2, z2)
	/// # Returns
	/// Returns a Result type that is either empty or a *LookUpError* signalling that a
	/// coordinate is out of bounds
	pub fn fill(&mut self, x1: usize, y1: usize, z1: usize, x2: usize, y2: usize, z2: usize, new_val: impl Borrow<T>) -> Result<(), LookUpError> {
		for y in y1..y2{ for x in x1..x2{ for z in z1..z2{
			self.set(x, y, z, new_val.borrow().clone())?;
		} } }
		Ok(())
	}

		/// Fills a region of this 3D array with a given value, wrapping the axese when
		/// coordinates go out of bounds.
		/// # Parameters
		/// * **x1** - the first x dimension coordinate (inclusive)
		/// * **y1** - the first y dimension coordinate (inclusive)
		/// * **z1** - the first z dimension coordinate (inclusive)
		/// * **x2** - the second x dimension coordinate (exclusive)
		/// * **y2** - the second y dimension coordinate (exclusive)
		/// * **z2** - the second z dimension coordinate (exclusive)
		/// * **new_val** - value to store in the 3D array in the bounding box defined by
		/// (x1, y1, z1) -> (x2, y2, z2)
		pub fn wrapped_fill(&mut self, x1: isize, y1: isize, z1: isize,
						x2: isize, y2: isize, z2: isize, new_val: impl Borrow<T>) {
		for y in y1..y2{ for x in x1..x2{ for z in z1..z2{
			self.wrapped_set(x, y, z, new_val.borrow().clone());
		} } }
	}

		/// Fills a region of this 3D array with a given value, ignoring any
		/// coordinates that go out of bounds.
		/// # Parameters
		/// * **x1** - the first x dimension coordinate (inclusive)
		/// * **y1** - the first y dimension coordinate (inclusive)
		/// * **z1** - the first z dimension coordinate (inclusive)
		/// * **x2** - the second x dimension coordinate (exclusive)
		/// * **y2** - the second y dimension coordinate (exclusive)
		/// * **z2** - the second z dimension coordinate (exclusive)
		/// * **new_val** - value to store in the 3D array in the bounding box defined by
		/// (x1, y1, z1) -> (x2, y2, z2)
		pub fn bounded_fill(&mut self, x1: isize, y1: isize, z1: isize,
						x2: isize, y2: isize, z2: isize, new_val: impl Borrow<T>) {
		for y in y1..y2{ for x in x1..x2{ for z in z1..z2{
			self.bounded_set(x, y, z, new_val.borrow().clone());
		} } }
	}
}

impl<T> ZArray3D<T> {
	/// Create a Z-index 3D array of values, initially filled with the provided constructor function.
	/// Note that the constructor function may be called for coordinates that are outside the
	/// requested dimensions in order to initialize memory in 8x8x8 blocks. To avoid this, use only
	/// dimensions that are multiples of 8.
	/// # Parameters
	/// * **xsize** - size of this 3D array in the X dimension
	/// * **ysize** - size of this 3D array in the Y dimension
	/// * **zsize** - size of this 3D array in the Z dimension
	/// * **constructor** - function which takes in the (X,Y,Z) coords as a tuple and returns a value of type T
	/// # Returns
	/// Returns an initialized *ZArray2D* struct filled with *default_val*
	pub fn new_with_constructor(xsize: usize, ysize: usize, zsize: usize, constructor: impl Fn((usize, usize, usize)) -> T) -> ZArray3D<T> {
		let px = ((xsize-1) >> 3) + 1;
		let py = ((ysize-1) >> 3) + 1;
		let pz = ((zsize-1) >> 3) + 1;
		let patch_count = px * py * pz;
		let mut p = Vec::with_capacity(patch_count);
		for pindex in 0..patch_count {
			let lookup_table = patch_coords(px, py, pindex);
			let initial_contents: [T; 512] = array_init(|i| constructor(lookup_table[i]));
			p.push(Patch { contents: initial_contents });
		}
		return ZArray3D { xsize, ysize, zsize, pxsize: px, pysize: py,
			patches: p, _phantomdata: PhantomData};
	}

	/// Gets the (x, y, z) size of this 3D array
	/// # Returns
	/// Returns a tuple of (width, height, depth) for this 2D array
	pub fn dimensions(&self) -> (usize, usize, usize){
		return (self.xsize, self.ysize, self.zsize);
	}

	/// Gets the X-dimension size (aka width) of this 3D array
	/// # Returns
	/// Returns the size in the X dimension
	pub fn xsize(&self) -> usize {
		return self.xsize;
	}


	/// Alias for `xsize()`
	/// # Returns
	/// Returns the size in the X dimension
	pub fn width(&self) -> usize {
		return self.xsize();
	}

	/// Gets the Y-dimension size (aka height) of this 3D array
	/// # Returns
	/// Returns the size in the Y dimension
	pub fn ysize(&self) -> usize {
		return self.ysize;
	}

	/// Alias for `ysize()`
	/// # Returns
	/// Returns the size in the Y dimension
	pub fn height(&self) -> usize {
		return self.ysize();
	}

	/// Gets the Z-dimension size (aka depth) of this 3D array
	/// # Returns
	/// Returns the size in the Z dimension
	pub fn zsize(&self) -> usize {
		return self.zsize;
	}

	/// Alias for `zsize()`
	/// # Returns
	/// Returns the size in the Z dimension
	pub fn depth(&self) -> usize {
		return self.zsize();
	}


	/// Gets a value from the 3D array, or returns a *LookUpError* if the provided coordinate
	/// is out of bounds. If you are using a default value for out-of-bounds coordinates,
	/// then you should use the *bounded_get(x, y, z)* method instead. If you want access to
	/// wrap-around (eg (-2, 0, 1) equivalent to (width-2, 0, 1)), then use the
	/// *wrapped_get(x, y, z)* method.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// # Returns
	/// Returns a Result type that holds either the returned data value (as a reference) from
	/// the 3D array, or a *LookUpError* signalling that the coordinate is out of bounds
	pub fn get(&self, x: usize, y: usize, z: usize) -> Result<&T,LookUpError>{
		if x < self.xsize && y < self.ysize && z < self.zsize {
			Ok(self.patches[patch_index(x, y, z, self.pxsize, self.pysize)].get(x, y, z))
		} else {
			Err(LookUpError{coord: vec![x, y, z],
				bounds: vec![self.xsize, self.ysize, self.zsize]})
		}
	}

	/// Sets a value in the 3D array, or returns a *LookUpError* if the provided coordinate
	/// is out of bounds. If you want out-of-bound coordinates to result in a no-op, then use
	/// the *bounded_set(x, y, z, val)* method instead. If you want access to wrap-around (eg
	/// (-2, 0, 1) equivalent to (width-2, 0, 1)), then use the
	/// *wrapped_set(x, y, z, val)* method.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// * **new_val** - value to store in the 3D array at (x, y, z)
	/// # Returns
	/// Returns a Result type that is either empty or a *LookUpError* signalling that the
	/// coordinate is out of bounds
	pub fn set(&mut self, x: usize, y: usize, z: usize, new_val: T) -> Result<(),LookUpError>{
		if x < self.xsize && y < self.ysize && z < self.zsize {
			Ok(self.patches[patch_index(x, y, z, self.pxsize, self.pysize)]
				.set(x, y, z, new_val))
		} else {
			Err(LookUpError{coord: vec![x, y, z],
				bounds: vec![self.xsize, self.ysize, self.zsize]})
		}
	}

	/// Gets a value from the 3D array without bounds checking
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// # Returns
	/// Returns the data value (as a reference) from the 3D array
	pub fn get_unchecked(&self, x: usize, y: usize, z: usize) -> &T {
		return self.patches[patch_index(x, y, z, self.pxsize, self.pysize)].get(x, y, z);
	}

	/// Sets a value in the 3D array without bounds checking
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// * **new_val** - value to store in the 3D array at (x, y, z)
	pub fn set_unchecked(&mut self, x: usize, y: usize, z: usize, new_val: T) {
		self.patches[patch_index(x, y, z, self.pxsize, self.pysize)]
				.set(x, y, z, new_val);
	}

	/// Gets a value from the 3D array, wrapping around the X and Y axese when the coordinates
	/// are negative or outside the size of this 2D array. Good for when you want tiling
	/// behavior.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// # Returns
	/// Returns a reference to the data stored at the provided coordinate (wrapping both x
	/// and y dimensions)
	pub fn wrapped_get(&self, x: isize, y: isize, z: isize) -> &T{
		let x = (self.xsize as isize + (x % self.xsize as isize)) as usize % self.xsize;
		let y = (self.ysize as isize + (y % self.ysize as isize)) as usize % self.ysize;
		let z = (self.zsize as isize + (z % self.zsize as isize)) as usize % self.zsize;
		return &self.patches[patch_index(x, y, z, self.pxsize, self.pysize)].get(x, y, z);
	}

	/// Sets a value in the 3D array at the provided coordinate, wrapping the X, Y, and Z axese
	/// if the coordinate is negative or out of bounds.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// * **new_val** - value to store in the 3D array at (x, y, z), wrapping around
	/// the x, y, and z dimensions
	pub fn wrapped_set(&mut self, x: isize, y: isize, z: isize, new_val: T) {
		let x = (self.xsize as isize + (x % self.xsize as isize)) as usize % self.xsize;
		let y = (self.ysize as isize + (y % self.ysize as isize)) as usize % self.ysize;
		let z = (self.zsize as isize + (z % self.zsize as isize)) as usize % self.zsize;
		self.patches[patch_index(x, y, z, self.pxsize, self.pysize)].set(x, y, z, new_val);
	}

	/// Gets a value from the 3D array as an Option that is None if the coordinate
	/// is out of bounds.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// # Returns
	/// Returns an Option type that holds either the returned data value (as a reference) from
	/// the 3D array, or *None* signalling that the coordinate is out of bounds (which can be
	/// combined with .unwrap_or(default_value) to implement an out-of-bounds default)
	pub fn bounded_get(&self, x: isize, y: isize, z: isize) -> Option<&T>{
		if x >= 0 && y >= 0 && z >= 0
			&& x < self.xsize as isize && y < self.ysize as isize && z < self.zsize as isize {
			return Some(&self.patches[
				patch_index(x as usize, y as usize, z as usize, self.pxsize, self.pysize)]
				.get(x as usize, y as usize, z as usize));
		} else {
			return None;
		}
	}

	/// Sets a value in the 3D array if and only if the provided coordinate is in bounds.
	/// Otherwise this method does nothing if the coordinate is out of bounds.
	/// # Parameters
	/// * **x** - x dimension coordinate
	/// * **y** - y dimension coordinate
	/// * **z** - z dimension coordinate
	/// * **new_val** - value to store int eh zD array at (x, y, z)
	pub fn bounded_set(&mut self, x: isize, y: isize, z: isize, new_val: T) {
		if x >= 0 && y >= 0 && z >= 0
			&& x < self.xsize as isize && y < self.ysize as isize && z < self.zsize as isize {
			self.patches[
				patch_index(x as usize, y as usize, z as usize, self.pxsize, self.pysize)]
				.set(x as usize, y as usize, z as usize, new_val);
		} else {
			// no-op
		}
	}

	/// Creates an iterator that iterates through the 3D array in Z-order
	/// # Returns
	/// A new ZArray3DIterator instance
	pub fn iter(&self) -> ZArray3DIterator<T> {
		ZArray3DIterator::new(self)
	}
	
	/// Applies a function to the Z-array to mutate it in-place
	/// # Parameters
	/// * **transform_fn** - Function that takes the coordsinate as a tuple and a
	/// reference to the old value and returns the new value
	pub fn transform(&mut self, transform_fn: impl Fn((usize, usize, usize), &T) -> T) {
		for pindex in 0..self.patches.len() {
			let patch_coords = patch_coords(self.pxsize, self.pysize, pindex);
			for coord in patch_coords {
				if coord.0 < self.xsize && coord.1 < self.ysize && coord.2 < self.zsize {
					let old_val = self.get_unchecked(coord.0, coord.1, coord.2);
					self.set_unchecked(coord.0, coord.1, coord.2, transform_fn(coord, old_val));
				}
			}
		}
	}

	/// Returns a vector of all valid (x, y, z) coordinates in this 3D array in Z-order
	pub fn coords(&self) -> Vec<(usize, usize, usize)> {
		let mut out: Vec<(usize, usize, usize)> = Vec::with_capacity(self.xsize * self.ysize * self.zsize);
		for pindex in 0..self.patches.len() {
			let patch_coords = patch_coords(self.pxsize, self.pysize, pindex);
			for coord in patch_coords {
				if coord.0 < self.xsize && coord.1 < self.ysize && coord.2 < self.zsize {
					out.push(coord);
				}
			}
		}
		return out;
	}
}

#[test]
fn check_patch_count_3d() {
	let arr = ZArray3D::new(1, 1, 1, 0u8);
	assert_eq!(arr.patches.len(), 1, "Allocated wrong number of patches for array of size {}x{}x{}", arr.xsize, arr.ysize, arr.zsize);
	let arr = ZArray3D::new(8, 8, 8, 0u8);
	assert_eq!(arr.patches.len(), 1, "Allocated wrong number of patches for array of size {}x{}x{}", arr.xsize, arr.ysize, arr.zsize);
	let arr = ZArray3D::new(9, 8, 8, 0u8);
	assert_eq!(arr.patches.len(), 2, "Allocated wrong number of patches for array of size {}x{}x{}", arr.xsize, arr.ysize, arr.zsize);
	let arr = ZArray3D::new(8, 9, 8, 0u8);
	assert_eq!(arr.patches.len(), 2, "Allocated wrong number of patches for array of size {}x{}x{}", arr.xsize, arr.ysize, arr.zsize);
	let arr = ZArray3D::new(8, 8, 9, 0u8);
	assert_eq!(arr.patches.len(), 2, "Allocated wrong number of patches for array of size {}x{}x{}", arr.xsize, arr.ysize, arr.zsize);
	let arr = ZArray3D::new(9, 9, 9, 0u8);
	assert_eq!(arr.patches.len(), 8, "Allocated wrong number of patches for array of size {}x{}x{}", arr.xsize, arr.ysize, arr.zsize);
}

/// Used for converting 3D coords to linear Z-index
const ZLUT: [u16; 16] = [
	0b0000000000000000,
	0b0000000000000001,
	0b0000000000001000,
	0b0000000000001001,
	0b0000000001000000,
	0b0000000001000001,
	0b0000000001001000,
	0b0000000001001001,
	0b0000001000000000,
	0b0000001000000001,
	0b0000001000001000,
	0b0000001000001001,
	0b0000001001000000,
	0b0000001001000001,
	0b0000001001001000,
	0b0000001001001001
];

/// General purpose Z-index function to convert a three-dimensional coordinate into a localized
/// one-dimensional coordinate
/// # Parameters
/// * **x** - x dimension coordinate *(ONLY THE LOWER 4 BITS WILL BE USED!)*
/// * **y** - y dimension coordinate *(ONLY THE LOWER 4 BITS WILL BE USED!)*
/// * **z** - z dimension coordinate *(ONLY THE LOWER 4 BITS WILL BE USED!)*
/// # Returns
/// Z-curve index for use as an index in a linear array meant to hold 2D data. In other words,
/// given the binary numbers X=0b0000xxxx, Y=0b0000yyyy, and Z=0b0000zzzz, then this method
/// will return 0b0000zyxzyxzyxzyx.
pub fn zorder_4bit_to_12bit(x: u8, y: u8, z: u8) -> u16 {
	let x_bits = ZLUT[(x & 0x0F) as usize];
	let y_bits = ZLUT[(y & 0x0F) as usize] << 1;
	let z_bits = ZLUT[(z & 0x0F) as usize] << 2;
	return z_bits | y_bits | x_bits;
}
/// General purpose Z-index function to convert a three-dimensional coordinate into a localized
/// one-dimensional coordinate
/// # Parameters
/// * **x** - x dimension coordinate (8 bit)
/// * **y** - y dimension coordinate (8 bit)
/// * **z** - z dimension coordinate (8 bit)
/// # Returns
/// Z-curve index for use as an index in a linear array meant to hold 2D data. In other words,
/// given the binary numbers X=0b0000xxxx, Y=0b0000yyyy, and Z=0b0000zzzz, then this method
/// will return 0b0000zyxzyxzyxzyx.
pub fn zorder_8bit_to_24bit(x:u8, y:u8, z: u8) -> u32 {
	return ((zorder_4bit_to_12bit(x >> 4, y >> 4, z >> 4) as u32) << 12)
		| zorder_4bit_to_12bit(x, y, z) as u32
}

/// Used by iterators to back-calculate the XYZ of an index, bit order is 0bzzzyyyxxx
const REVERSE_ZLUT: [u16; 512] = [
	0, 1, 8, 9, 64, 65, 72, 73, 2, 3, 10, 11, 66, 67, 74, 75,
	16, 17, 24, 25, 80, 81, 88, 89, 18, 19, 26, 27, 82, 83, 90, 91,
	128, 129, 136, 137, 192, 193, 200, 201, 130, 131, 138, 139, 194, 195, 202, 203,
	144, 145, 152, 153, 208, 209, 216, 217, 146, 147, 154, 155, 210, 211, 218, 219,
	4, 5, 12, 13, 68, 69, 76, 77, 6, 7, 14, 15, 70, 71, 78, 79,
	20, 21, 28, 29, 84, 85, 92, 93, 22, 23, 30, 31, 86, 87, 94, 95,
	132, 133, 140, 141, 196, 197, 204, 205, 134, 135, 142, 143, 198, 199, 206, 207,
	148, 149, 156, 157, 212, 213, 220, 221, 150, 151, 158, 159, 214, 215, 222, 223,
	32, 33, 40, 41, 96, 97, 104, 105, 34, 35, 42, 43, 98, 99, 106, 107,
	48, 49, 56, 57, 112, 113, 120, 121, 50, 51, 58, 59, 114, 115, 122, 123,
	160, 161, 168, 169, 224, 225, 232, 233, 162, 163, 170, 171, 226, 227, 234, 235,
	176, 177, 184, 185, 240, 241, 248, 249, 178, 179, 186, 187, 242, 243, 250, 251,
	36, 37, 44, 45, 100, 101, 108, 109, 38, 39, 46, 47, 102, 103, 110, 111,
	52, 53, 60, 61, 116, 117, 124, 125, 54, 55, 62, 63, 118, 119, 126, 127,
	164, 165, 172, 173, 228, 229, 236, 237, 166, 167, 174, 175, 230, 231, 238, 239,
	180, 181, 188, 189, 244, 245, 252, 253, 182, 183, 190, 191, 246, 247, 254, 255,
	256, 257, 264, 265, 320, 321, 328, 329, 258, 259, 266, 267, 322, 323, 330, 331,
	272, 273, 280, 281, 336, 337, 344, 345, 274, 275, 282, 283, 338, 339, 346, 347,
	384, 385, 392, 393, 448, 449, 456, 457, 386, 387, 394, 395, 450, 451, 458, 459,
	400, 401, 408, 409, 464, 465, 472, 473, 402, 403, 410, 411, 466, 467, 474, 475,
	260, 261, 268, 269, 324, 325, 332, 333, 262, 263, 270, 271, 326, 327, 334, 335,
	276, 277, 284, 285, 340, 341, 348, 349, 278, 279, 286, 287, 342, 343, 350, 351,
	388, 389, 396, 397, 452, 453, 460, 461, 390, 391, 398, 399, 454, 455, 462, 463,
	404, 405, 412, 413, 468, 469, 476, 477, 406, 407, 414, 415, 470, 471, 478, 479,
	288, 289, 296, 297, 352, 353, 360, 361, 290, 291, 298, 299, 354, 355, 362, 363,
	304, 305, 312, 313, 368, 369, 376, 377, 306, 307, 314, 315, 370, 371, 378, 379,
	416, 417, 424, 425, 480, 481, 488, 489, 418, 419, 426, 427, 482, 483, 490, 491,
	432, 433, 440, 441, 496, 497, 504, 505, 434, 435, 442, 443, 498, 499, 506, 507,
	292, 293, 300, 301, 356, 357, 364, 365, 294, 295, 302, 303, 358, 359, 366, 367,
	308, 309, 316, 317, 372, 373, 380, 381, 310, 311, 318, 319, 374, 375, 382, 383,
	420, 421, 428, 429, 484, 485, 492, 493, 422, 423, 430, 431, 486, 487, 494, 495,
	436, 437, 444, 445, 500, 501, 508, 509, 438, 439, 446, 447, 502, 503, 510, 511,
];


/// This struct is used by `ZArray2DIterator` to present values to the consumer of the
/// iterator
#[derive(Debug)]
pub struct ZArray3DIteratorItem<'a, T> {
	/// x-dimension coordinate
	pub x: usize,
	/// y-dimension coordinate
	pub y: usize,
	/// z-dimension coordinate
	pub z: usize,
	/// reference to value at this coordinate
	pub value: &'a T
}

/// private state management enum
enum IterState {
	Start, Processing, Done
}
/// Iterator that iterates through the array
pub struct ZArray3DIterator<'a, T> {
	/// array to iterate over
	array: &'a ZArray3D<T>,
	patch: usize,
	index: usize,
	state: IterState
}

impl<'a, T> ZArray3DIterator<'a, T> {
	fn new(array: &'a ZArray3D<T>) -> ZArray3DIterator<'a, T> {
		if array.xsize == 0 || array.ysize == 0 || array.zsize == 0 {
			ZArray3DIterator{array, patch: 0, index: 0, state: IterState::Done} // make a "done" iterator for empty arrays
		} else {
			ZArray3DIterator{array, patch: 0, index: 0, state: IterState::Start}
		}
	}
}

impl<'a, T> Iterator for ZArray3DIterator<'a, T> {
	type Item = ZArray3DIteratorItem<'a, T>;

	fn next(&mut self) -> Option<Self::Item> {
		match &self.state {
			IterState::Done=> None,
			IterState::Start=> {
				self.state = IterState::Processing;
				Some(ZArray3DIteratorItem{x: 0, y: 0, z: 0, value: &self.array.patches[0].contents[0]})
			},
			IterState::Processing => {
				let mut x ; let mut y ; let mut z ;
				loop {
					self.index += 1;
					if self.index >= 512 {
						self.index  = 0;
						self.patch += 1;
					}
					let zyx_lower_pits = REVERSE_ZLUT[self.index];
					x = ((self.patch % self.array.pxsize) << 3) |  (zyx_lower_pits & 0x07) as usize;
					y = (((self.patch / self.array.pxsize) % self.array.pysize ) << 3) | ((zyx_lower_pits >> 3) & 0x07) as usize;
					z = ( (self.patch / (self.array.pysize * self.array.pxsize)) << 3) | ((zyx_lower_pits >> 6) & 0x07) as usize;
					if x < self.array.xsize && y < self.array.ysize && z < self.array.zsize {
						break;
					}
					if self.patch >= self.array.patches.len() {
						self.state = IterState::Done;
						return None;
					}
				}
				Some(ZArray3DIteratorItem{x, y, z, value: &self.array.patches[self.patch].contents[self.index]})
			}
		}
	}
}

#[test]
fn print_3d_zorder() {
	let mut array = ZArray3D::new(8, 8, 8, 0usize);
	for z in 0..8usize {
		for y in 0..8usize {
			for x in 0..8usize {
				array.set_unchecked(x, y, z, (z << 6) | (y << 3) | x);
			}
		}
	}
	println!("[");
	for i in 0..array.patches[0].contents.len() {
		print!("{:?}, ", array.patches[0].contents[i]);
		if i % 16 == 15{println!()}
	}
	println!("]");
}
