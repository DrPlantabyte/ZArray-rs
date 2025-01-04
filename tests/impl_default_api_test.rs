//! API test for how I want to use the library

use std::string::String;

#[derive(Default)]
struct MyDataStruct{
	id: i32,
	value: i64,
	label: String,
	tags: Box<Vec<String>>
}
