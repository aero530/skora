//! Rust types to work with Tiff data
//!
//! Object types and functions used to hold the data from a tiff file.
//!

use std::collections::BTreeMap;

use crate::endian_rw::{
    Endian,
    order_read,
    order_write_8,
    order_write_16,
    order_write_32,
};

/// Top level meta data about the Sketchbook tiff image file.
#[derive(Clone, Debug)]
pub struct Info {
    /// Endian format of the image
    pub endian: Endian,
    /// True if this file is in big tiff format, False if classic
    pub big_tiff: bool,
    /// The offset of the first IFD in the file
    pub first_ifd: usize,
    /// The first four bytes of the tiff file
    pub header: Vec<u8>,
    /// The total length of the tiff file in bytes
    pub size: usize,
}

/// Meta data about an individual ifd (image file directory)
#[derive(Clone, Debug)]
pub struct Ifd {
    /// Endian format of the image
    pub endian: Endian,
    /// True if this file is in big tiff format, False if classic
    pub big_tiff: bool,
    /// The offset of the ifd from the beginning of the file
    pub offset: usize,
    /// The size of the ifd's image
    pub size: usize,
    /// Number of tags in this IFD
    pub tag_count: u64,
    /// Map of the tags for this IFD. The keys are the integer tag values.
    pub tags: BTreeMap<u64, Tag>,
}

/// Piece of information / data about the ifd.
///
/// Data in tiff files is organized by tags.  There are a bunch of tags defined in the Tiff standard to encode various bits of info about the image.
/// Sketchbook Tiffs also use private tags to encode application specific information (along with layer data).
#[derive(Clone, Debug)]
pub struct Tag {
    /// The number of elements in the tag.  For most numeric values, this is the total number of entries.  For rational, this is
    /// the number of pairs of entries.  For ascii, this is the length in bytes including a terminating null.  For undefined, this is the length in bytes.
    pub count: u64,
    /// The data this tag refers to
    pub data: Data,
    /// The offset within the file (always within the IFD) that the data or offset to the data is located.
    pub datapos: usize,
    /// The datatype for this tag
    pub datatype: DataType,
    /// A list of sub-ifds
    pub ifds: Option<Vec<Ifd>>,
    /// If the count is large enough that the data cannot be stored in the IFD, this is the offset within the file of the data associated with the tag.
    pub offset: Option<usize>,
}

/// The type of data that is stored in any given tag as defined in the tiff spec
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataType {
    Byte,
    Ascii,
    Short,
    Long,
    Rational
}

impl From<u16> for DataType {
    /// Return a datatype from an integer per the mapping in the tiff spec
    fn from(n: u16) -> DataType {
        match n {
            1 => DataType::Byte,
            2 => DataType::Ascii,
            3 => DataType::Short,
            4 => DataType::Long,
            5 => DataType::Rational,
            _ => panic!()
        }
    }
}


/// Enum to store data from the tag fields.  Fields in general are arrays (vecs) of data
#[derive(Clone, Debug)]
pub enum Data {
    /// Bytes are u8
    Byte(Vec<u8>),
    /// Ascii data is converted to string
    Ascii(String),
    /// Shorts are u16
    Short(Vec<u16>),
    /// Longs are u32
    Long(Vec<u32>),
    /// Rational numbers are stored as two u32 in the raw data.  Here the f64 (rational) value is stored along with the two u32's (numerator then denominator) that define the rational.
    Rational(Vec<(f64, u32, u32)>),
}

impl Data {
    /// Create a new Data object of the specified DataType
    pub fn new(raw_data: Vec<u8>, data_type: DataType, endian: Endian, count: u64) -> Self {
        match data_type {
            DataType::Byte => {
                let mut tag_data = Vec::new();
                for n in 0..(count) {
                    let start = (n) as usize;
                    let end = (n + 1) as usize;
                    tag_data.push(order_read(endian, &raw_data[start..end], 1) as u8);
                }
                Data::Byte(tag_data)
            }
            DataType::Ascii => {
                let tag_data = String::from_utf8_lossy(&raw_data);
                Data::Ascii(tag_data.to_string())
            }
            DataType::Short => {
                let mut tag_data = Vec::new();
                for n in 0..(count) {
                    let start = (n * 2) as usize;
                    let end = (n * 2 + 2) as usize;
                    tag_data.push(order_read(endian, &raw_data[start..end], 2) as u16);
                }
                Data::Short(tag_data)
            }
            DataType::Long => {
                let mut tag_data = Vec::new();
                for n in 0..(count) {
                    let start = (n * 4) as usize;
                    let end = (n * 4 + 4) as usize;
                    tag_data.push(order_read(endian, &raw_data[start..end], 4) as u32);
                }
                Data::Long(tag_data)
            }
            DataType::Rational => {
                let mut tag_data_raw = Vec::new();
                for n in 0..(count * 2) {
                    let start = (n * 4) as usize;
                    let end = (n * 4 + 4) as usize;
                    tag_data_raw.push(order_read(endian, &raw_data[start..end], 4) as u32);
                }

                let mut tag_data = Vec::new();
                for n in 0..(count) {
                    let start = (n * 2) as usize;
                    let end = (n * 2 + 1) as usize;
                    tag_data.push((
                        tag_data_raw[start] as f64 / tag_data_raw[end] as f64,
                        tag_data_raw[start] as u32,
                        tag_data_raw[end] as u32,
                    ))
                }
                Data::Rational(tag_data)
            }
        }
    }
}

impl Data {
    /// Convert the values in the data to a vector of u8 bytes
    pub fn to_vec_u8(&self, endian: Endian) -> Vec<u8> {
        match self {
            Data::Byte(val) => {
                let mut buf: Vec<u8> = Vec::new();
                for number in val {
                    order_write_8(endian, &mut buf, *number);
                }
                buf
            }
            Data::Ascii(val) => val.clone().into_bytes(),
            Data::Short(val) => {
                let mut buf: Vec<u8> = Vec::new();
                for number in val {
                    order_write_16(endian, &mut buf, *number);
                }
                buf
            }
            Data::Long(val) => {
                let mut buf: Vec<u8> = Vec::new();
                for number in val {
                    order_write_32(endian, &mut buf, *number);
                }
                buf
            }
            Data::Rational(val) => {
                let mut buf: Vec<u8> = Vec::new();
                for number in val {
                    order_write_32(endian, &mut buf, number.1);
                    order_write_32(endian, &mut buf, number.2);
                }
                buf
            }
        }
    }
}

impl Data {
    /// Return the number of u8 bytes each item takes in raw data form (as stored in the tiff tag)
    pub fn element_size_in_bytes(&self) -> u64 {
        match self {
            Data::Byte(_) => 1,
            Data::Ascii(_) => 1,
            Data::Short(_) => 2,
            Data::Long(_) => 4,
            Data::Rational(_) => 8,
        }
    }
}

impl Data {
    /// Return the integer used in the tiff spec to represent the type of data stored in this value
    pub fn type_tiff_id(&self) -> u64 {
        match self {
            Data::Byte(_) => 1,
            Data::Ascii(_) => 2,
            Data::Short(_) => 3,
            Data::Long(_) => 4,
            Data::Rational(_) => 5
        }
    }
}


