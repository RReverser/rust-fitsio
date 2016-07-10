#![allow(dead_code, unused_imports)]

extern crate fitsio_sys as sys;
extern crate libc;

use std::ptr;
use std::ffi;

mod stringutils;

/// Error type
#[derive(Debug, PartialEq, Eq)]
pub struct FitsError {
    status: i32,
    message: String,
}

pub type Result<T> = std::result::Result<T, FitsError>;

/// Hdu description type
///
/// Any way of describing a HDU - number or string which either
/// changes the hdu by absolute number, or by name.
pub trait DescribesHdu {
    fn change_hdu(&self, fptr: &FitsFile) -> Result<()>;
}

impl DescribesHdu for usize {
    fn change_hdu(&self, f: &FitsFile) -> Result<()> {
        let mut _hdu_type = 0;
        let mut status = 0;
        unsafe {
            sys::ffmahd(f.fptr, (*self + 1) as i32, &mut _hdu_type, &mut status);
        }
        match status {
            0 => Ok(()),
            _ => {
                Err(FitsError {
                    status: status,
                    message: stringutils::status_to_string(status).unwrap(),
                })
            }
        }
    }
}

impl<'a> DescribesHdu for &'a str {
    fn change_hdu(&self, f: &FitsFile) -> Result<()> {
        let mut _hdu_type = 0;
        let mut status = 0;
        let c_hdu_name = ffi::CString::new(*self).unwrap();

        unsafe {
            sys::ffmnhd(f.fptr,
                        sys::HduType::ANY_HDU as libc::c_int,
                        c_hdu_name.into_raw(),
                        0,
                        &mut status);
        }

        match status {
            0 => Ok(()),
            _ => {
                Err(FitsError {
                    status: status,
                    message: stringutils::status_to_string(status).unwrap(),
                })
            }
        }
    }
}

/// Trait applied to types which can be read from a FITS header
///
/// This is currently:
///
/// * i32
/// * i64
/// * f32
/// * f64
/// * String
pub trait ReadsKey {
    fn read_key(f: &FitsFile, name: &str) -> Result<Self> where Self: std::marker::Sized;
}

macro_rules! reads_key_impl {
    ($t:ty, $func:ident) => (
        impl ReadsKey for $t {
            fn read_key(f: &FitsFile, name: &str) -> Result<Self> {
                let c_name = ffi::CString::new(name).unwrap();
                let mut status = 0;
                let mut value: Self = Self::default();

                unsafe {
                    sys::$func(f.fptr,
                           c_name.into_raw(),
                           &mut value,
                           ptr::null_mut(),
                           &mut status);
                }

                match status {
                    0 => Ok(value),
                    s => {
                        Err(FitsError {
                            status: s,
                            message: stringutils::status_to_string(s).unwrap(),
                        })
                    }
                }
            }
        }
    )
}

reads_key_impl!(i32, ffgkyl);
reads_key_impl!(i64, ffgkyj);
reads_key_impl!(f32, ffgkye);
reads_key_impl!(f64, ffgkyd);

impl ReadsKey for String {
    fn read_key(f: &FitsFile, name: &str) -> Result<Self> {
        let c_name = ffi::CString::new(name).unwrap();
        let mut status = 0;
        let mut value: Vec<libc::c_char> = vec![0; sys::MAX_VALUE_LENGTH];

        unsafe {
            sys::ffgkys(f.fptr,
                        c_name.into_raw(),
                        value.as_mut_ptr(),
                        ptr::null_mut(),
                        &mut status);
        }

        match status {
            0 => {
                let value: Vec<u8> = value.iter()
                    .map(|&x| x as u8)
                    .filter(|&x| x != 0)
                    .collect();
                Ok(String::from_utf8(value).unwrap())
            }
            status => {
                Err(FitsError {
                    status: status,
                    message: stringutils::status_to_string(status).unwrap(),
                })
            }
        }

    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum HduInfo {
    ImageInfo {
        dimensions: usize,
        shape: Vec<usize>,
    },
    TableInfo {
        column_names: Vec<String>,
        num_rows: usize,
    },
}

pub struct FitsFile {
    fptr: *mut sys::fitsfile,
    pub filename: String,
    hdu_info: HduInfo,
}

unsafe fn fetch_hdu_info(fptr: *mut sys::fitsfile) -> Result<HduInfo> {
    let mut status = 0;
    let mut hdu_type = 0;

    sys::ffghdt(fptr, &mut hdu_type, &mut status);
    let hdu_type = match hdu_type {
        0 => {
            let mut dimensions = 0;
            sys::ffgidm(fptr, &mut dimensions, &mut status);

            let mut shape = vec![0; dimensions as usize];
            sys::ffgisz(fptr, dimensions, shape.as_mut_ptr(), &mut status);

            HduInfo::ImageInfo {
                dimensions: dimensions as usize,
                shape: shape.iter().map(|v| *v as usize).collect(),
            }
        }
        1 | 2 => {
            let mut num_rows = 0;
            sys::ffgnrw(fptr, &mut num_rows, &mut status);

            let mut num_cols = 0;
            sys::ffgncl(fptr, &mut num_cols, &mut status);
            let mut column_names = Vec::with_capacity(num_cols as usize);

            for i in 0..num_cols {
                let mut buffer: Vec<libc::c_char> = vec![0; 71];
                sys::ffgbcl(fptr,
                       (i + 1) as i32,
                       buffer.as_mut_ptr(),
                       ptr::null_mut(),
                       ptr::null_mut(),
                       ptr::null_mut(),
                       ptr::null_mut(),
                       ptr::null_mut(),
                       ptr::null_mut(),
                       ptr::null_mut(),
                       &mut status);
                column_names.push(stringutils::buf_to_string(&buffer).unwrap());
            }

            HduInfo::TableInfo {
                column_names: column_names,
                num_rows: num_rows as usize,
            }
        }
        _ => panic!("Invalid hdu type found"),
    };

    match status {
        0 => Ok(hdu_type),
        _ => {
            Err(FitsError {
                status: status,
                message: stringutils::status_to_string(status).unwrap(),
            })
        }
    }
}

impl FitsFile {
    pub fn open(filename: &str) -> Result<Self> {
        let mut fptr = ptr::null_mut();
        let mut status = 0;
        let c_filename = ffi::CString::new(filename).unwrap();

        unsafe {
            sys::ffopen(&mut fptr as *mut *mut sys::fitsfile,
                        c_filename.as_ptr(),
                        sys::FileOpenMode::READONLY as libc::c_int,
                        &mut status);
        }

        match status {
            0 => {
                let hdu_info = unsafe { fetch_hdu_info(fptr).unwrap() };
                Ok(FitsFile {
                    fptr: fptr,
                    hdu_info: hdu_info,
                    filename: filename.to_string(),
                })
            }
            status => {
                Err(FitsError {
                    status: status,
                    message: stringutils::status_to_string(status).unwrap(),
                })
            }
        }

    }

    pub fn create(path: &str) -> Result<Self> {
        let mut fptr = ptr::null_mut();
        let mut status = 0;
        let c_filename = ffi::CString::new(path).unwrap();

        unsafe {
            sys::ffinit(&mut fptr as *mut *mut sys::fitsfile,
                        c_filename.as_ptr(),
                        &mut status);
        }

        match status {
            0 => {
                Ok(FitsFile {
                    fptr: fptr,
                    hdu_info: HduInfo::ImageInfo {
                        dimensions: 0,
                        shape: Vec::new(),
                    },
                    filename: path.to_string(),
                })
            }
            status => {
                Err(FitsError {
                    status: status,
                    message: stringutils::status_to_string(status).unwrap(),
                })
            }
        }
    }

    pub fn hdu<T: DescribesHdu>(&mut self, hdu_description: T) -> Result<&Self> {
        try!(hdu_description.change_hdu(self));
        self.hdu_info = self.fetch_hdu_info().unwrap();
        Ok(self)
    }

    fn hdu_number(&self) -> usize {
        let mut hdu_num = 0;
        unsafe {
            sys::ffghdn(self.fptr, &mut hdu_num);
        }
        (hdu_num - 1) as usize
    }

    pub fn read_key<T: ReadsKey>(&self, name: &str) -> Result<T> {
        T::read_key(self, name)
    }

    fn fetch_hdu_info(&self) -> Result<HduInfo> {
        unsafe { fetch_hdu_info(self.fptr) }
    }
}

impl Drop for FitsFile {
    fn drop(&mut self) {
        let mut status = 0;
        unsafe {
            sys::ffclos(self.fptr, &mut status);
        }
    }
}

#[cfg(test)]
mod test {
    extern crate tempdir;
    use super::*;

    #[test]
    fn opening_an_existing_file() {
        match FitsFile::open("../testdata/full_example.fits") {
            Ok(_) => {}
            Err(e) => panic!("{:?}", e),
        }
    }

    #[test]
    fn creating_a_new_file() {
        let tdir = tempdir::TempDir::new("fitsio-").unwrap();
        let tdir_path = tdir.path();
        let filename = tdir_path.join("test.fits");
        assert!(!filename.exists());

        match FitsFile::create(filename.to_str().unwrap()) {
            Ok(_) => assert!(filename.exists()),
            Err(e) => panic!("Error: {:?}", e),
        }
    }

    #[test]
    fn fetching_a_hdu() {
        let mut f = FitsFile::open("../testdata/full_example.fits").unwrap();
        for i in 0..2 {
            assert_eq!(f.hdu(i).unwrap().hdu_number(), i);
        }
        match f.hdu(2) {
            Err(e) => assert_eq!(e.status, 107),
            _ => panic!("Error checking for failure"),
        }

        let tbl_hdu = f.hdu("TESTEXT").unwrap();
        assert_eq!(tbl_hdu.hdu_number(), 1);
    }

    #[test]
    fn reading_header_keys() {
        let mut f = FitsFile::open("../testdata/full_example.fits").unwrap();
        match f.hdu(0).unwrap().read_key::<i64>("INTTEST") {
            Ok(value) => assert_eq!(value, 42),
            Err(e) => panic!("Error reading key: {:?}", e),
        }

        match f.hdu(0).unwrap().read_key::<f64>("DBLTEST") {
            Ok(value) => assert_eq!(value, 0.09375),
            Err(e) => panic!("Error reading key: {:?}", e),
        }

        match f.hdu(0).unwrap().read_key::<String>("TEST") {
            Ok(value) => assert_eq!(value, "value"),
            Err(e) => panic!("Error reading key: {:?}", e),
        }
    }

    #[test]
    fn reading_hdu_info() {
        let mut f = FitsFile::open("../testdata/full_example.fits").unwrap();

        assert_eq!(f.hdu_info,
                   HduInfo::ImageInfo {
                       dimensions: 2,
                       shape: vec![100, 100],
                   });
        assert_eq!(f.hdu(1).unwrap().hdu_info,
                   HduInfo::TableInfo {
                       num_rows: 50,
                       column_names: vec!["intcol".to_string(),
                                          "floatcol".to_string(),
                                          "doublecol".to_string()],
                   });
    }
}
