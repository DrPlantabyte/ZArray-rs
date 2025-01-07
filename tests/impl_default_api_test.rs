//! API test for how I want to use the library

use std::string::String;

#[derive(Default, Debug)]
struct MyDataStruct{
	id: i32,
	value: i64,
	label: String,
	tags: Box<Vec<String>>
}

#[test]
fn test_2d(){
	use zarray::z2d::ZArray2D;
	let mut array2d = ZArray2D::new_with_default::<MyDataStruct>(11, 2);
	array2d.set(1, 1, MyDataStruct{id: 2, value: 11, label: "hello".into(), Box::new(vec!["tag1".to_string(), "tag2".to_string()])});
	for y in (0..array2d.height()).rev() {
		for x in 0..array2d.width() {
			let e = array2d.get(x, y).unwrap();
			print!("({}, {}): {:?} ", x, y, e)
		}
	}
}
