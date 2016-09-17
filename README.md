# rust-cfitsio

FFI wrapper around cfitsio in Rust


[![Join the chat at https://gitter.im/mindriot101/rust-cfitsio](https://badges.gitter.im/mindriot101/rust-cfitsio.svg)](https://gitter.im/mindriot101/rust-cfitsio?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)
[![Build Status](https://travis-ci.org/mindriot101/rust-cfitsio.svg?branch=master)](https://travis-ci.org/mindriot101/rust-cfitsio)

## Installation

For the time being, it's best to stick to the development version from github.
The code is tested before being pushed and is relatively stable. Add this to
your `Cargo.toml` file:

```toml
[dependencies]
fitsio = { git = "https://github.com/mindriot101/rust-cfitsio" }
```

If you want the latest release from `crates.io` then add the following:

```toml
[dependencies]
fitsio = "*"
```

Or pin a specific version:

```toml
[dependencies]
fitsio = "0.2.0"
```


## Documentation

`fitsio-sys` [![`fitsio-sys` documentation](https://docs.rs/fitsio-sys/badge.svg)](https://docs.rs/crate/fitsio-sys)<br />
`fitsio` [![`fitsio` documentation](https://docs.rs/fitsio/badge.svg)](https://docs.rs/crate/fitsio/)


## Api roadmap

```
FitsFile
- fn read_key -> returns header value
- if image:
    - fn image_dimensions -> Vec<usize>
    - fn image_type -> DataType
    - fn read_section -> reads image section into either Vec<_> or ndarray
    - fn read_region -> reads a square region into either Vec<_> or ndarray
- if table:
    - fn num_rows -> usize
    - fn rows -> impl Iterator over rows
    - fn row -> get single row by index
    - fn columns -> impl Iterator over columns
    - fn column -> get single column by name or index
```

### Images

* Change HDU
* Read image data
* Get image metadata

### Tables

## Examples

Open a fits file

```rust
let f = fitsio::FitsFile::open("test.fits");
```
