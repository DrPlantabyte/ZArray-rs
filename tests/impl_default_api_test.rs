//! API test for how I want to use the library

use std::string::String;

#[derive(Default, Debug, PartialEq, Eq)]
struct MyDataStruct{
	id: i32,
	value: i64,
	label: String,
	tags: Box<Vec<String>>
}

#[test]
fn test_2d_default_struct(){
	use zarray::z2d::ZArray2D;
	let mut array2d = ZArray2D::<MyDataStruct>::new_with_default(9, 2);
	array2d.set(1, 1, MyDataStruct{id: 2, value: 11, label: "hello".into(), tags: Box::new(vec!["tag1".to_string(), "tag2".to_string()])}).unwrap();
	eprintln!("");
	for y in (0..array2d.height()).rev() {
		for x in 0..array2d.width() {
			let e = array2d.get(x, y).unwrap();
			eprint!("({}, {}): [{} {} {} <{:?}>]     ", x, y, e.id, e.value, e.label, e.tags);
			if x != 1 && y != 1 {
				assert_eq!(*e, MyDataStruct::default());
			}
		}
		eprintln!("");
	}
}

#[test]
fn test_3d_default_struct(){
	use zarray::z3d::ZArray3D;
	let mut array3d = ZArray3D::<MyDataStruct>::new_with_default(9, 3, 2);
	array3d.set(1, 1, 1, MyDataStruct{id: 2, value: 11, label: "hello".into(), tags: Box::new(vec!["tag1".to_string(), "tag2".to_string()])}).unwrap();
	eprintln!("");
	for z in 0..array3d.depth() {
		for y in 0..array3d.height() {
			for x in 0..array3d.width() {
				let e = array3d.get(x, y, z).unwrap();
				eprint!("({}, {}, {}): [{} {} {} <{:?}>]     ", x, y, z, e.id, e.value, e.label, e.tags);
				if x != 1 && y != 1 && z != 1 {
					assert_eq!(*e, MyDataStruct::default());
				}
			}
		eprintln!("");
		}
		eprintln!("");
	}
}
