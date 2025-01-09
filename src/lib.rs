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

 #![ doc = include_str!("../README.md")]

pub mod z2d;
pub mod z3d;

use core::error::Error;
use core::fmt::{Debug, Display, Formatter};

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
