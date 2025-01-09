# ZArray
Z-order indexed 2D and 3D arrays using Morton order (aka Z-order) with a convenient API for common 2D and 3D access patterns. Use of zarray in place of a Vec of Vecs often improves performance, especially for algorithms such as blurring and cellular automata.

[Crates.io page](https://crates.io/crates/zarray)

## About ZArray
The *zarray* crate  is a lightweight Rust library that provides structs for working with 2D and 3D arrays, using internal Z-Order Morton indexing to improve data localization for better cache-line performance.

## Quickstart Guide
Simply import *zarray::z2d::ZArray2D* and/or *zarray::z3d::ZArray3D* and then use one of the
*ZArray_D::new_(...)* constructor functions to initialize a new instance:
 * Use *ZArray_D::new(...)* for types that implement the `Copy` trait (ie primitive types like `i32`)
 * Use *ZArray_D::new_with_default(...)* for types that implement the `Default` trait (eg `#[derive(Default)] struct MyStruct{...}`)
 * Use *ZArray_D::new_with_constructor(...)* for all other types

For example, here's a simple blur operation using ZArray2D, which generally performs better than using a Vec of Vecs by about 10-25%:
```rust
use zarray::z2d::ZArray2D;
let h: isize = 200;
let w: isize = 300;
let radius: isize = 3;
let mut src = ZArray2D::new(w as usize, h as usize, 0u8);
// set values
src.bounded_fill(100, 100, 200, 150, 255u8);
// sum neighbors values with ZArray
let mut blurred = ZArray2D::new(w as usize, h as usize, 0u16);
for y in 0..h { for x in 0..w {
  let mut sum = 0;
  for dy in -radius..radius+1 { for dx in -radius..radius+1 {
    sum += *src.bounded_get(x+dx, y+dy).unwrap_or(&0u8) as u16;
  } }
  blurred.set(x as usize, y as usize, sum/((2*radius as u16+1).pow(2))).unwrap();
} }
```

## How it works
the *ZArray_D* structs store data in 8x8 or 8x8x8 chuncks, using Z-order indexing to access the data within each chunk (as described [here](https://en.wikipedia.org/wiki/Z-order_curve) ). In so doing, the lowest 4 bits of each dimension are interdigitated to significantly improve data locality and cache-line fetch efficiency (though not as much as a Hilbert curve would do).

## Why not just use Vec of Vecs (aka Vec<Vec<T>>)?
Most of the time, using a `Vec<Vec<T>>` would have great performance, so long as you remember to structure your for-loops correctly. However, when the data is not accessed in a linear fashion, such as when implementing a cellular automata or a blurring or ray tracing algorithm, then the performance of a `Vec<Vec<T>>` can be significantly impaired by frequent RAM access and cache-line misses. This is when data locality matters most for performance.

### Why not Z-Order the entire data array?
Two reasons: Firstly, Z-Order indexing only works on square/cube shaped data, so a pure Z-Order index would waste huge amount of memory for 2D and 3D arrays that are long and/or thin. Second, on most CPU architectures (Intel, AMD, and Arm), memory is accessed in 64-byte cache-lines, thus the performance gains from Z-order indexing are less significant above 6 bits of linear addressing space (ie 8x8 or 4x4x4).

## Note
As of version 1.3.0, any type can be used with the *zarray* crate (eariler versions only allowed data types with the *Copy* trait). However, you will not see a performance improvement over a simple `Vec<Vec<T>>` if `T` is not a sized type or contains pointers (`Box`, `Arc`, etc) or other heap-allocated data. Even so, you may find value in *zarray*'s utility functions such as `wrapped_get/set(...)` and `bounded_get/set(...)` which allow for hassle-free out-of-bounds handling when pplying raster operations to the array.

## License
This library is provided under the MIT license. In other words: free to use as you wish.

## Contributing
If you'd like to contribute, go ahead and fork the GitHub repo and/or submit a pull request
