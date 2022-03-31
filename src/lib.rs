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

	#[test]
	fn test_zarray2dmap_get_set(){
		use crate::z2d::ZArray2D;
		use rand::{rngs::StdRng, Rng, SeedableRng};
		let mut prng = StdRng::seed_from_u64(20220331u64);
		let h: usize = 601;
		let w: usize = 809;
		let mut ref_map: Vec<Vec<u8>> = vec![vec![0u8;w];h];
		let mut map = ZArray2D::new(w, h, 0u8);
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
	fn test_zarray2dmap_power_of_8(){
		use crate::z2d::ZArray2D;
		use rand::{rngs::StdRng, Rng, SeedableRng};
		let mut prng = StdRng::seed_from_u64(20220331u64);
		let h: usize = 64;
		let w: usize = 64;
		let mut ref_map: Vec<Vec<u8>> = vec![vec![0u8;w];h];
		let mut map = ZArray2D::new(w, h, 0u8);
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
		use crate::z2d::ZArray2D;
		use rand::{rngs::StdRng, Rng, SeedableRng};
		let mut prng = StdRng::seed_from_u64(20220331u64);
		let h: usize = 3;
		let w: usize = 5;
		let mut ref_map: Vec<Vec<u8>> = vec![vec![0u8;w];h];
		let mut map = ZArray2D::new(w, h, 0u8);
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
	fn test_zarray2dmap_performance(){
		use crate::z2d::ZArray2D;
		use rand::{rngs::StdRng, Rng, SeedableRng};
		use std::time::{Duration, Instant};
		let mut prng = StdRng::seed_from_u64(20220331u64);
		let h: usize = 100;
		let w: usize = 100;
		let mut ref_map: Vec<Vec<u8>> = vec![vec![0u8;w];h];
		let mut map = ZArray2D::new(w, h, 0u8);
		// set values
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[y][x] = v;
				map.set(x, y, v);
			}
		}
		// sum neighbors values with benchmark reference (vecs)
		let mut ref_map_sums: Vec<Vec<u16>> = vec![vec![0u16;w];h];
		let radius: usize = 3;
		let t0 = Instant::now();
		for y in radius..h-radius {
			for x in radius..w-radius {
				let mut sum = 0;
				for dy in -radius..radius+1 as i32 {
					for dx in -radius..radius+1 as i32 {
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
				for dy in -radius..radius+1 as i32 {
					for dx in -radius..radius+1 as i32 {
						sum += *map.get((x as i32+dx) as usize, (y as i32+dy) as usize).unwrap() as u16;
					}
				}
				map_sums.set(x, y, sum).unwrap();
			}
		}
		let t1 = Instant::now();
		let my_time = (t1-t0).as_secs_f64()*1e6;
		println!("ZArray2D {}x{} sum of neighbors performance: {} micros", w, h, my_time);
		println!("Performance improved by {}%", (100. * (ref_time / my_time)) as i32);
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
