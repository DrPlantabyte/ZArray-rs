/*
MIT License

Copyright (c) 2022 Christopher Collin Hall (aka DrPlantabyte)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
 */

//! # ZArray
//! Z-order indexed 2D and 3D arrays using Morton order (aka Z-order) with a convenient API for
//! common 2D and 3D access patterns. Use of zarray in place of a Vec of Vecs often improves
//! performance, especially for algorithms such as blurring and cellular automata.
//! ## About ZArray
//! The *zarray* crate  is a lightweight Rust library that provides structs for working with 2D and
//! 3D arrays, using internal Z-Order Morton indexing to improve data localization for better
//! cache-line performance.
//! ## Quickstart Guide
//! Simply import *zarray::z2d::ZArray2D* and/or *zarray::z3d::ZArray3D* and then use the
//! *ZArray_D::new(...)* function to initialize a new instance. The type will automatically be
//! inferred from the povided default value. Note that only types which implement the *Copy* trait
//! are allowed (ie not Vec or other heap-allocating types).
//!
//! For example, here's a simple blur operation using ZArray2D, which generally performs better
//! than using a Vec of Vecs by about 10-25%:
//! ```rust
//! use zarray::z2d::ZArray2D;
//! let h: isize = 200;
//! let w: isize = 300;
//! let radius: isize = 3;
//! let mut src = ZArray2D::new(w as usize, h as usize, 0u8);
//! // set values
//! src.bounded_fill(100, 100, 200, 150, 255u8);
//! // sum neighbors values with ZArray
//! let mut blurred = ZArray2D::new(w as usize, h as usize, 0u16);
//! for y in 0..h { for x in 0..w {
//!   let mut sum = 0;
//!   for dy in -radius..radius+1 { for dx in -radius..radius+1 {
//!     sum += *src.bounded_get(x+dx, y+dy).unwrap_or(&0u8) as u16;
//!   } }
//!   blurred.set(x as usize, y as usize, sum/((2*radius as u16+1).pow(2))).unwrap();
//! } }
//! ```
//!
//! ## How it works
//! the *ZArray_D* structs store data in 8x8 or 8x8x8 chuncks, using Z-order indexing to access the
//! data within each chunk (as described [here](https://en.wikipedia.org/wiki/Z-order_curve) ). In
//! so doing, the lowest 4 bits of each dimension are interdigitated to significantly improve data
//! locality and cache-line fetch efficiency (though not as much as a Hilbert curve would do)
//!
//! ## Why not just use Vec of Vecs (aka Vec<Vec<T>>)?
//! Most of the time, using a Vec<Vec<T>> would have great performance, so long as you remember to
//! structure your for-loops correctly. However, when the data is not accessed in a linear fashion,
//! such as when implementing a cellular automata or a blurring or ray tracing algorithm, then the
//! performance of a Vec<Vec<T>> can be significantly impaired by frequent RAM access and
//! cache-line misses. This is when data locality matters most for performance.
//!
//! ### Why not Z-Order the entire data array?
//! Two reasons: Firstly, Z-Order indexing only works on square/cube shaped data, so a pure Z-Order
//! index would waste huge amount of memory for 2D and 3D arrays that are long and thin. Second, on
//! most CPU architectures (Intel, AMD, and Arm), memory is accessed in 64-byte cache-lines, thus
//! the performance gains from Z-order indexing are less significant above 6 bits of linear
//! addressing space (ie 8x8 or 4x4x4)
//!
//! ## Note
//! Only types with the *Copy* trait can be stored in *ZArray_D* structs. Thus *zarray* works for
//! all numerical types and most simple data structs, but not for heap-allocating data types such
//! as Vec. This limitation arises from complexities in filling the data patches with initial
//! values. In addition, you would not see a performance improvement over a simple Vec<Vec<T>> if
//! the data resided on the heap.
//!
//! ## License
//! This library is provided under the MIT license. In other words: free to use as you wish.
//!
//! ## Contributing
//! If you'd like to contribute, go ahead and fork the GitHub repo and/or submit a pull request

pub mod z2d;
pub mod z3d;

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

/// This struct is an error type that is returned when attempting to get a value that is outside
/// the range of the data. It implements the Debug and Display traits so that it can be easily
/// printed as an error message.
pub struct LookUpError{
	/// coordinate that was out of bounds
	coord: Vec<usize>,
	/// bounds of the ZArray*D that was violated
	bounds: Vec<usize>,
}

impl Debug for LookUpError {
	// programmer-facing error message
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		return write!(f, "{{ file: {}, line: {}, coord: {}, bounds: {} }}", file!(), line!(), vec_to_string(&self.coord), vec_to_string(&self.bounds));
	}
}

impl Display for LookUpError {
	// user-facing error message
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		return write!(f, "Error: could not access coordinate {} because it is out of range for size {}", vec_to_string(&self.coord), vec_to_string(&self.bounds));
	}
}

impl Error for LookUpError{}

impl LookUpError {

}
/// Utility function for converting Vecs to Strings for the purpose of error reporting and debugging
fn vec_to_string(v: &Vec<usize>) -> String{
	let mut sb = String::from("(");
	let mut not_first = false;
	for n in v {
		if not_first {
			sb += &String::from(", ");
		} else {
			not_first = true;
		}
		sb += &String::from(n.to_string());
	}
	sb += &String::from(")");
	return sb;
}
