//! black-box unit tests
use zarray::z2d::ZArray2D;
use zarray::z3d::ZArray3D;
use rand::{rngs::StdRng, Rng, SeedableRng};


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
	// assert get sizes
	assert_eq!(map.dimensions().0, w);
	assert_eq!(map.dimensions().1, h);
	assert_eq!(map.xsize(), w);
	assert_eq!(map.width(), w);
	assert_eq!(map.ysize(), h);
	assert_eq!(map.height(), h);
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
	println!("Vec<Vec<u16>> {}x{} sum of neighbors in radius {} performance: {} micros", w, h,
				radius, ref_time as i32);

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
		radius, my_time as i32);
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

fn seed_3darrays_u8(w: usize, h: usize, d: usize) -> (Vec<Vec<Vec<u8>>>, ZArray3D<u8>){
	let ref_map: Vec<Vec<Vec<u8>>> = vec![vec![vec![0u8;w];h];d];
	let map = ZArray3D::new(w, h, d, 0u8);
	return (ref_map, map);
}

#[test]
fn test_zarray3dmap_get_set(){
	let h: usize = 11;
	let w: usize = 39;
	let d: usize = 23;
	let (mut ref_map, mut map) = seed_3darrays_u8(w, h, d);
	let mut prng = StdRng::seed_from_u64(20220331u64);
	// assert get sizes
	assert_eq!(map.dimensions().0, w);
	assert_eq!(map.dimensions().1, h);
	assert_eq!(map.dimensions().2, d);
	assert_eq!(map.xsize(), w);
	assert_eq!(map.width(), w);
	assert_eq!(map.ysize(), h);
	assert_eq!(map.height(), h);
	assert_eq!(map.zsize(), d);
	assert_eq!(map.depth(), d);
	// set values
	for z in 0..d {
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[z][y][x] = v;
				map.set(x, y, z, v).unwrap();
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
fn test_zarray3dmap_wrapped_get_set(){
	let h: usize = 20;
	let w: usize = 20;
	let d: usize = 20;
	let (mut ref_map, mut map) = seed_3darrays_u8(w, h, d);
	let mut prng = StdRng::seed_from_u64(20220331u64);
	// set values
	for z in -10..10 as isize {
		for y in -10..10 as isize {
			for x in -10..10 as isize {
				let v: u8 = prng.gen();
				ref_map[((20 + z % 20) % 20) as usize][((20 + y % 20) % 20) as usize][((20 + x % 20) % 20) as usize]
					= v;
				map.wrapped_set(x, y, z, v);
			}
		}
	}
	let m: isize = 101;
	let v: u8 = prng.gen();
	ref_map[((20+(m/2)%20)%20) as usize][((20+m%20)%20) as usize]
		[((20+(3*m)%20)%20) as usize] = v;
	map.wrapped_set(3*m, m, m/2, v);
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
fn test_zarray3dmap_bounded_get_set(){
	let h: usize = 20;
	let w: usize = 20;
	let d: usize = 20;
	let (mut ref_map, mut map) = seed_3darrays_u8(w, h, d);
	let mut prng = StdRng::seed_from_u64(20220331u64);
	// set values
	for z in -10..10 as isize {
		for y in -10..10 as isize {
			for x in -10..10 as isize {
				let v: u8 = prng.gen();
				if x >= 0 && x < w as isize && y >= 0 && y < h as isize
					&& z >= 0 && z < d as isize{
					ref_map[z as usize][y as usize][x as usize] = v;
				}
				map.bounded_set(x, y, z, v);
			}
		}
	}
	let oob: u8 = 127;
	let m: isize = 101;
	let v: u8 = prng.gen();
	map.bounded_set(3*m, m, m/2, v); // should be a no-op
	// get values
	for z in 0..d {
		for y in 0..h {
			for x in 0..w {
				assert_eq!(ref_map[z][y][x],
								*map.bounded_get(x as isize, y as isize, z as isize)
									.unwrap_or(&oob));
			}
		}
	}
	assert_eq!(oob, *map.bounded_get(-1, 0, 0).unwrap_or(&oob));
	assert_eq!(oob, *map.bounded_get(0,  -1, 0).unwrap_or(&oob));
	assert_eq!(oob, *map.bounded_get(0,  0, -1).unwrap_or(&oob));
	assert_eq!(oob, *map.bounded_get(w as isize,  h as isize, d as isize).unwrap_or(&oob));
}

#[test]
fn test_zarray3dmap_power_of_8(){
	let h: usize = 8;
	let w: usize = 8;
	let d: usize = 8;
	let (mut ref_map, mut map) = seed_3darrays_u8(w, h, d);
	let mut prng = StdRng::seed_from_u64(20220331u64);
	// set values
	for z in 0..d {
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[z][y][x] = v;
				map.set(x, y, z, v).unwrap();
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
	let h: usize = 3;
	let w: usize = 2;
	let d: usize = 2;
	let (mut ref_map, mut map) = seed_3darrays_u8(w, h, d);
	let mut prng = StdRng::seed_from_u64(20220331u64);
	// set values
	for z in 0..d {
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[z][y][x] = v;
				map.set(x, y, z, v).unwrap();
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
fn test_zarray3dmap_performance_neighborsum(){
	use std::time::Instant;
	let h: usize = 20;
	let w: usize = 10;
	let d: usize = 30;
	let (mut ref_map, mut map) = seed_3darrays_u8(w, h, d);
	let mut prng = StdRng::seed_from_u64(20220331u64);
	// set values
	for z in 0..d {
		for y in 0..h {
			for x in 0..w {
				let v: u8 = prng.gen();
				ref_map[z][y][x] = v;
				map.set(x, y, z, v).unwrap();
			}
		}
	}
	// sum neighbors values with benchmark reference (vecs)
	let mut ref_map_sums: Vec<Vec<Vec<u32>>> = vec![vec![vec![0u32;w];h];d];
	let radius: usize = 2;
	let rad_plus = radius * 2 + 1;
	let t0 = Instant::now();
	for z in radius..d - radius {
		for y in radius..h - radius {
			for x in radius..w - radius {
				let mut sum = 0;
				for rz in 0..rad_plus as i32 {
					let dz = rz - radius as i32;
					for ry in 0..rad_plus as i32 {
						let dy = ry - radius as i32;
						for rx in 0..rad_plus as i32 {
							let dx = rx - radius as i32;
							sum += ref_map[(z as i32 + dz) as usize][(y as i32 + dy) as usize]
								[(x as i32 + dx) as usize] as u32;
						}
					}
				}
				ref_map_sums[z][y][x] = sum;
			}
		}
	}
	let t1 = Instant::now();
	let ref_time =  (t1-t0).as_secs_f64()*1e6;
	println!("Vec<Vec<u16>> {}x{}x{} sum of neighbors in radius {} performance: {} micros",
		w, h, d, radius, ref_time as i32);

	// sum neighbors values with ZArray
	let mut map_sums = ZArray3D::new(w, h, d, 0u32);
	let t0 = Instant::now();
	for z in radius..d - radius {
		for y in radius..h - radius {
			for x in radius..w - radius {
				let mut sum = 0;
				for rz in 0..rad_plus as i32 {
					let dz = rz - radius as i32;
					for ry in 0..rad_plus as i32 {
						let dy = ry - radius as i32;
						for rx in 0..rad_plus as i32 {
							let dx = rx - radius as i32;
							sum += *map.get((x as i32+dx) as usize, (y as i32+dy) as usize,
							(z as i32+dz) as usize).unwrap() as u32;
						}
					}
				}
				map_sums.set(x, y, z, sum).unwrap();
			}
		}
	}
	let t1 = Instant::now();
	let my_time = (t1-t0).as_secs_f64()*1e6;
	println!("ZArray2D {}x{}x{} sum of neighbors in radius {} performance: {} micros", w, h, d,
				radius, my_time as i32);
	println!("Performance improved by {}%", (100. * (ref_time / my_time - 1.)) as i32);
}

#[test]
fn test_erosion_sim(){
	let width = 100;
	let length = 200;
	let depth = 25;
	let air = 0f32;
	let soil_hardness = 1f32;
	let rock_hardness = 8f32;
	let drip_power = 1.5f32;
	let iterations = 12;
	let mut map = ZArray3D::new(width, length, depth, air);
	map.fill(0,0,5, width,length,depth, soil_hardness).unwrap();
	map.fill(0,0,15, width,length,depth, rock_hardness).unwrap();
	for boulder in [(34,88,6), (66,122,9), (11,154,5), (35,93,8), (72,75,12)]{
		map.set(boulder.0, boulder.1, boulder.2, rock_hardness).unwrap();
	}
	for _ in 0..iterations{
		for x in 0..width{for y in 0..length{
			let mut drip = drip_power;
			let mut z = 0;
			while drip > 0f32 {
				let h = *map.bounded_get(x as isize, y as isize, z).unwrap_or(&100f32);
				if h > drip {
					map.bounded_set(x as isize, y as isize, z, h - drip);
					drip = 0.;
				} else {
					map.bounded_set(x as isize, y as isize, z, 0.);
					drip -= h;
				}
				z += 1;
			}
		}}
	}
}

#[test]
fn iter_2d_test() {
	test_2d_iter(8,8);
	test_2d_iter(3,5);
	test_2d_iter(11,5);
	test_2d_iter(3,11);
	test_2d_iter(57,101);
	test_2d_iter(111,51);
}
fn test_2d_iter(w: usize, h: usize) {
	let a1 = init_with_count_2d(w, h);
	let a2 = init_with_count_2d(w, h);
	for item in a1.iter() {
		assert_eq!(item.value, *a2.get(item.x, item.y).expect("Out of bounds"));
	}
}
fn init_with_count_2d(w: usize, h: usize) -> ZArray2D<i32> {
	let mut array = ZArray2D::new(w, h, 0i32);
	let mut i: i32 = 0;
	for y in 0..h {
		for x in 0..w {
			array.set_unchecked(x, y, i);
			i += 1;
		}
	}
	array
}
#[test]
fn iter_3d_test() {
	test_3d_iter(8,8,8);
	test_3d_iter(3,5,4);
	test_3d_iter(11,5,3);
	test_3d_iter(3,11,1);
	test_3d_iter(57,101,89);
	test_3d_iter(111,51,101);
}
fn test_3d_iter(w: usize, h: usize, l: usize) {
	let a1 = init_with_count_3d(w, h, l);
	let a2 = init_with_count_3d(w, h, l);
	for item in a1.iter() {
		assert_eq!(item.value, *a2.get(item.x, item.y, item.z).expect("Out of bounds"));
	}
}
fn init_with_count_3d(w: usize, h: usize, l: usize) -> ZArray3D<i32> {
	let mut array = ZArray3D::new(w, h, l, 0i32);
	let mut i: i32 = 0;
	for z in 0..l {
		for y in 0..h {
			for x in 0..w {
				array.set_unchecked(x, y, z, i);
				i += 1;
			}
		}
	}
	array
}