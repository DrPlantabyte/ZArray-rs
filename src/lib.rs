use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct LookUpError{
	coord: Vec<usize>,
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

pub mod z2d {
	// Z-order indexing in 2 dimensions

	use std::marker::PhantomData;
	use crate::LookUpError;


	struct Patch<T>{
		contents: [T;64]
	}

	impl<T> Patch<T> {
		fn get(&self, x: usize, y:usize) -> &T {
			// 3-bit x 3-bit
			return &self.contents[zorder_4bit_to_8bit(x as u8 & 0x07, y as u8 & 0x07) as usize];
		}
		fn set(&mut self, x: usize, y:usize, new_val: T) {
			// 3-bit x 3-bit
			let i = zorder_4bit_to_8bit(x as u8 & 0x07, y as u8 & 0x07) as usize;
			//let old_val = &self.contents[i];
			self.contents[i] = new_val;
			//return old_val;
		}
	}

	fn patch_index(x: usize, y:usize, pwidth: usize) -> usize{
		return (x >> 3) + ((y >> 3) * (pwidth));
	}

	pub struct ZArray2D<T> {
		// for heap allocated data
		width: usize,
		height: usize,
		pwidth: usize,
		patches: Vec<Patch<T>>,
		_phantomdata: PhantomData<T>,
	}

	impl<T> ZArray2D<T> where T: Copy {
		pub fn new(width: usize, height: usize, default_val: T) -> ZArray2D<T>{
			let pwidth = (width >> 3) + 1;
			let pheight = (height >> 3) + 1;
			let patch_count = pwidth * pheight;
			let mut p = Vec::with_capacity(patch_count);
			for _ in 0..patch_count{
				p.push(Patch{contents: [default_val; 64]});
			}
			println!("width: {}, height: {}, pwidth: {}, patch count: {}", width, height, pwidth, p.len());
			return ZArray2D {width, height, pwidth, patches: p, _phantomdata: PhantomData};
		}


		pub fn get(&self, x: usize, y: usize) -> Result<&T,LookUpError>{
			if x < self.width && y < self.height {
				Ok(self.patches[patch_index(x, y, self.pwidth)].get(x, y))
			} else {
				Err(LookUpError{coord: vec![x, y], bounds: vec![self.width, self.height]})
			}
		}

		pub fn set(&mut self, x: usize, y: usize, new_val: T) -> Result<(),LookUpError>{
			if x < self.width && y < self.height {
				Ok(self.patches[patch_index(x, y, self.pwidth)].set(x, y, new_val))
			} else {
				Err(LookUpError{coord: vec![x, y], bounds: vec![self.width, self.height]})
			}
		}

		pub fn wrapped_get(&self, x: isize, y: isize) -> &T{
			let x = (self.width as isize + (x % self.width as isize)) as usize % self.width;
			let y = (self.height as isize + (y % self.height as isize)) as usize % self.height;
			return &self.patches[patch_index(x, y, self.pwidth)].get(x, y);
		}

		pub fn wrapped_set(&mut self, x: isize, y: isize, new_val: T) {
			let x = (self.width as isize + (x % self.width as isize)) as usize % self.width;
			let y = (self.height as isize + (y % self.height as isize)) as usize % self.height;
			self.patches[patch_index(x, y, self.pwidth)].set(x, y, new_val);
		}

		pub fn bounded_get(&self, x: isize, y: isize) -> Option<&T>{
			if x >= 0 && y >= 0 && x < self.width as isize && y < self.height as isize {
				return Some(&self.patches[patch_index(x as usize, y as usize, self.pwidth)]
					.get(x as usize, y as usize));
			} else {
				return None;
			}
		}

		pub fn bounded_set(&mut self, x: isize, y: isize, new_val: T) {
			if x >= 0 && y >= 0 && x < self.width as isize && y < self.height as isize {
				self.patches[patch_index(x as usize, y as usize, self.pwidth)]
					.set(x as usize, y as usize, new_val);
			} else {
				// no-op
			}
		}
	}


	static ZLUT: [u8; 16] = [
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

	pub fn zorder_4bit_to_8bit(x: u8, y: u8) -> u8 {
		let x_bits = ZLUT[(x & 0x0F) as usize];
		let y_bits = ZLUT[(y & 0x0F) as usize] << 1;
		return y_bits | x_bits;
	}

	pub fn zorder_8bit_to_16bit(x:u8, y:u8) -> u16 {
		return ((zorder_4bit_to_8bit(x >> 4, y >> 4) as u16) << 8) | zorder_4bit_to_8bit(x, y) as u16
	}

	pub fn zorder_16bit_to_32bit(x:u16, y:u16) -> u32 {
		return ((zorder_8bit_to_16bit((x & 0xFF) as u8, (y & 0xFF) as u8) as u32) << 16) | zorder_8bit_to_16bit((x >> 8) as u8, (y >> 8) as u8) as u32
	}



}


#[cfg(test)]
mod tests {
	use crate::z2d::ZArray2D;
	use rand::{rngs::StdRng, Rng, SeedableRng};

	/*
	#[test]
	fn test_test(){
		use std::collections::BTreeMap;
		use std::string::String;
		let mut m: BTreeMap<i32, String> = BTreeMap::new();
		m.insert(2, "two".to_string());
		m.insert(1, "one".to_string());
		m.insert(3, "three".to_string());
		m.insert(100, "hundred".to_string());
		m.insert(-100, "minus hundred".to_string());
		for e in m {
			println!("<{}, {}>", e.0, e.1);
		}
	}
	*/

	fn seed_arrays_u8(w: usize, h: usize) -> (Vec<Vec<u8>>, ZArray2D<u8>){
		let ref_map: Vec<Vec<u8>> = vec![vec![0u8;w];h];
		let map = ZArray2D::new(w, h, 0u8);
		return (ref_map, map);
	}
	#[test]
	fn test_zarray2dmap_get_set(){
		let h: usize = 601;
		let w: usize = 809;
		let (mut ref_map, mut map) = seed_arrays_u8(w, h);
		let mut prng = StdRng::seed_from_u64(20220331u64);
		// set values
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[y][x] = v;
				map.set(x, y, v).unwrap();
			}
		}
		// get values
		for y in 0..h {
			for x in 0..w {
				assert_eq!(ref_map[y][x], *map.get(x, y).unwrap());
			}
		}
	}

	#[test]
	fn test_zarray2dmap_wrapped_get_set(){
		let h: usize = 20;
		let w: usize = 20;
		let (mut ref_map, mut map) = seed_arrays_u8(w, h);
		let mut prng = StdRng::seed_from_u64(20220331u64);
		// set values
		for y in -10..10 as isize{
			for x in -10..10 as isize {
				let v: u8 = prng.gen();
				ref_map[((20+y%20)%20) as usize][((20+x%20)%20) as usize] = v;
				map.wrapped_set(x, y, v);
			}
		}
		let m: isize = 101;
		let v: u8 = prng.gen();
		ref_map[((20+m%20)%20) as usize][((20+(3*m)%20)%20) as usize] = v;
		map.wrapped_set(3*m, m, v);
		// get values
		for y in 0..h {
			for x in 0..w {
				assert_eq!(ref_map[y][x], *map.get(x, y).unwrap());
			}
		}
	}


	#[test]
	fn test_zarray2dmap_bounded_get_set(){
		let h: usize = 20;
		let w: usize = 20;
		let (mut ref_map, mut map) = seed_arrays_u8(w, h);
		let mut prng = StdRng::seed_from_u64(20220331u64);
		// set values
		for y in -10..10 as isize{
			for x in -10..10 as isize {
				let v: u8 = prng.gen();
				if x >= 0 && x < w as isize && y >= 0 && y < h as isize {
					ref_map[y as usize][x as usize] = v;
				}
				map.bounded_set(x, y, v);
			}
		}
		let oob: u8 = 127;
		let m: isize = 101;
		let v: u8 = prng.gen();
		map.bounded_set(3*m, m, v); // should be a no-op
		// get values
		for y in 0..h {
			for x in 0..w {
				assert_eq!(ref_map[y][x], *map.bounded_get(x as isize, y as isize).unwrap_or(&oob));
			}
		}
		assert_eq!(oob, *map.bounded_get(-1, 0).unwrap_or(&oob));
		assert_eq!(oob, *map.bounded_get(0,  -1).unwrap_or(&oob));
		assert_eq!(oob, *map.bounded_get(w as isize,  h as isize).unwrap_or(&oob));
	}

	#[test]
	fn test_zarray2dmap_power_of_8(){
		let h: usize = 64;
		let w: usize = 64;
		let (mut ref_map, mut map) = seed_arrays_u8(w, h);
		let mut prng = StdRng::seed_from_u64(20220331u64);
		// set values
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[y][x] = v;
				map.set(x, y, v).unwrap();
			}
		}
		// get values
		for y in 0..h {
			for x in 0..w {
				assert_eq!(ref_map[y][x], *map.get(x, y).unwrap());
			}
		}
	}

	#[test]
	fn test_zarray2dmap_small(){
		let h: usize = 3;
		let w: usize = 5;
		let (mut ref_map, mut map) = seed_arrays_u8(w, h);
		let mut prng = StdRng::seed_from_u64(20220331u64);
		// set values
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[y][x] = v;
				map.set(x, y, v).unwrap();
			}
		}
		// get values
		for y in 0..h {
			for x in 0..w {
				assert_eq!(ref_map[y][x], *map.get(x, y).unwrap());
			}
		}
	}

	#[test]
	fn test_zarray2dmap_performance_neighborsum(){
		use std::time::Instant;
		let h: usize = 300;
		let w: usize = 300;
		let (mut ref_map, mut map) = seed_arrays_u8(w, h);
		let mut prng = StdRng::seed_from_u64(20220331u64);
		// set values
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[y][x] = v;
				map.set(x, y, v).unwrap();
			}
		}
		// sum neighbors values with benchmark reference (vecs)
		let mut ref_map_sums: Vec<Vec<u16>> = vec![vec![0u16;w];h];
		let radius: usize = 2;
		let rad_plus = radius * 2 + 1;
		let t0 = Instant::now();
		for y in radius..h-radius {
			for x in radius..w-radius {
				let mut sum = 0;
				for ry in 0..rad_plus as i32 {
					let dy = ry - radius as i32;
					for rx in 0..rad_plus as i32 {
						let dx = rx - radius as i32;
						sum += ref_map[(y as i32+dy) as usize][(x as i32+dx) as usize] as u16;
					}
				}
				ref_map_sums[y][x] = sum;
			}
		}
		let t1 = Instant::now();
		let ref_time =  (t1-t0).as_secs_f64()*1e6;
		println!("Vec<Vec<u16>> {}x{} sum of neighbors in radius {} performance: {} micros", w, h, radius, ref_time);

		// sum neighbors values with ZArray
		let mut map_sums = ZArray2D::new(w, h, 0u16);
		let t0 = Instant::now();
		for y in radius..h-radius {
			for x in radius..w-radius {
				let mut sum = 0;
				for ry in 0..rad_plus as i32 {
					let dy = ry - radius as i32;
					for rx in 0..rad_plus as i32 {
						let dx = rx - radius as i32;
						sum += *map.get((x as i32+dx) as usize, (y as i32+dy) as usize).unwrap() as u16;
					}
				}
				map_sums.set(x, y, sum).unwrap();
			}
		}
		let t1 = Instant::now();
		let my_time = (t1-t0).as_secs_f64()*1e6;
		println!("ZArray2D {}x{} sum of neighbors in radius {} performance: {} micros", w, h,
			radius, my_time);
		println!("Performance improved by {}%", (100. * (ref_time / my_time - 1.)) as i32);
	}

	#[test]
	fn test_zarray2dmap_performance_pathfinding(){
		use std::time::Instant;
		use pathfinding::prelude::{absdiff, astar};
		let h: usize = 300;
		let w: usize = 300;
		let (mut ref_map, mut map) = seed_arrays_u8(w, h);
		let mut prng = StdRng::seed_from_u64(20220331u64);
		// set values
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[y][x] = v;
				map.set(x, y, v).unwrap();
			}
		}
		// A* pathfinding with benchmark reference (vecs)
		let oob: u8 = 127;
		let goal: (i32, i32) = (w as i32 - 1, h as i32 - 1);
		let start: (i32, i32) = (1, 1);
		let t0 = Instant::now();
		let result = astar(
			&start,
			|&(x, y)| vec![
							(x+1,y), (x-1,y), (x,y+1), (x,y-1)
					].into_iter().map(|p:(i32, i32)| (p,
						if p.0 >= 0 && p.1 >= 0 && p.0 < w as i32 && p.1 < h as i32 {
							ref_map[p.1 as usize][p.0 as usize] as i32} else {oob as i32})),
			|&(x, y)| absdiff(x, goal.0) + absdiff(y, goal.1),
			|&p| p == goal
		);
		let (ref_path, ref_cost) = result.unwrap();
		let t1 = Instant::now();
		let ref_time =  (t1-t0).as_secs_f64()*1e6;
		println!("Vec<Vec<u16>> {}x{} A* path from ({},{}) to ({},{}) (path length = {}, cost = \
		{}) performance: {} micros",
				 w, h, start.0, start.1, goal.0, goal.1, ref_path.len(), ref_cost, ref_time);

		// A* pathfinding with ZArray
		let t0 = Instant::now();
		let result = astar(
			&start,
			|&(x, y)| vec![
				(x+1,y), (x-1,y), (x,y+1), (x,y-1)
			].into_iter().map(|p:(i32, i32)| (p, *map.bounded_get(p.0 as isize, p.1 as isize )
				.unwrap_or(&oob) as i32)),
			|&(x, y)| absdiff(x, goal.0) + absdiff(y, goal.1),
			|&p| p == goal
		);
		let (my_path, my_cost) = result.unwrap();
		let t1 = Instant::now();
		let my_time =  (t1-t0).as_secs_f64()*1e6;
		println!("ZArray2D {}x{} A* path from ({},{}) to ({},{}) (path length = {}, cost = \
		{}) performance: {} micros",
				 w, h, start.0, start.1, goal.0, goal.1, my_path.len(), my_cost, my_time);
		assert_eq!(ref_path.len(), my_path.len());
		assert_eq!(ref_cost, my_cost);
		println!("Performance improved by {}%", (100. * (ref_time / my_time - 1.)) as i32);

	}

/*
	#[test]
	fn test_zarray3dmap_get_set(){
		use crate::z3d::ZArray3D;
		use rand::{rngs::StdRng, Rng, SeedableRng};
		let mut prng = StdRng::seed_from_u64(20220331u64);
		let h: usize = 29;
		let w: usize = 57;
		let d: usize = 13;
		let mut ref_map: Vec<Vec<Vec<u8>>> = vec![vec![vec![0u8;w];h];d];
		let mut map = ZArray3D::new(w, h, d, 0u8);
		// set values
		for z in 0..d {
			for y in 0..h {
				for x in 0..w {
					let v: u8 = prng.gen();
					ref_map[z][y][x] = v;
					map.set(x, y, z, v);
				}
			}
		}
		// get values
		for z in 0..d {
			for y in 0..h {
				for x in 0..w {
					assert_eq!(ref_map[z][y][x], *map.get(x, y, z).unwrap());
				}
			}
		}
	}

	#[test]
	fn test_zarray3dmap_power_of_8(){
		use crate::z3d::ZArray3D;
		use rand::{rngs::StdRng, Rng, SeedableRng};
		let mut prng = StdRng::seed_from_u64(20220331u64);
		let h: usize = 64;
		let w: usize = 64;
		let d: usize = 64;
		let mut ref_map: Vec<Vec<Vec<u8>>> = vec![vec![vec![0u8;w];h];d];
		let mut map = ZArray3D::new(w, h, d, 0u8);
		// set values
		for z in 0..d {
			for y in 0..h {
				for x in 0..w {
					let v: u8 = prng.gen();
					ref_map[z][y][x] = v;
					map.set(x, y, z, v);
				}
			}
		}
		// get values
		for z in 0..d {
			for y in 0..h {
				for x in 0..w {
					assert_eq!(ref_map[z][y][x], *map.get(x, y, z).unwrap());
				}
			}
		}
	}

	#[test]
	fn test_zarray3dmap_small(){
		use crate::z3d::ZArray3D;
		use rand::{rngs::StdRng, Rng, SeedableRng};
		let mut prng = StdRng::seed_from_u64(20220331u64);
		let h: usize = 3;
		let w: usize = 2;
		let d: usize = 3;
		let mut ref_map: Vec<Vec<Vec<u8>>> = vec![vec![vec![0u8;w];h];d];
		let mut map = ZArray3D::new(w, h, d, 0u8);
		// set values
		for z in 0..d {
			for y in 0..h {
				for x in 0..w {
					let v: u8 = prng.gen();
					ref_map[z][y][x] = v;
					map.set(x, y, z, v);
				}
			}
		}
		// get values
		for z in 0..d {
			for y in 0..h {
				for x in 0..w {
					assert_eq!(ref_map[z][y][x], *map.get(x, y, z).unwrap());
				}
			}
		}
	}
 */
}
