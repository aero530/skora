//! Endian Read and Write
//!
//! Provide functions needed to read / write data in various
//! byte sizes in little or big endian order.
//!

use byteorder::{BigEndian, ByteOrder, LittleEndian};

/// Byte order of the data
#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Big,
    Little,
}

/// Read an n byte integer from a buffer based on the endian order specified
pub fn order_read(endian: Endian, buffer: &[u8], size: usize) -> u64 {
    match endian {
        Endian::Big => BigEndian::read_uint(buffer, size),
        Endian::Little => LittleEndian::read_uint(buffer, size),
    }
}

/// Write an 8 byte value to a buffer based on the endian order specified
pub fn order_write_8(_endian: Endian, buffer: &mut Vec<u8>, data: u8) {
    buffer.push(data);
}

/// Write an 16 byte value to a buffer based on the endian order specified
pub fn order_write_16(endian: Endian, buffer: &mut Vec<u8>, data: u16) {
    let mut new_bytes = [0; 2];
    match endian {
        Endian::Big => BigEndian::write_u16(&mut new_bytes, data),
        Endian::Little => LittleEndian::write_u16(&mut new_bytes, data),
    };
    buffer.append(&mut new_bytes.to_vec());
}

/// Write an 32 byte value to a buffer based on the endian order specified
pub fn order_write_32(endian: Endian, buffer: &mut Vec<u8>, data: u32) {
    let mut new_bytes = [0; 4];
    match endian {
        Endian::Big => BigEndian::write_u32(&mut new_bytes, data),
        Endian::Little => LittleEndian::write_u32(&mut new_bytes, data),
    };
    buffer.append(&mut new_bytes.to_vec());
}

/// Write an 64 byte value to a buffer based on the endian order specified
pub fn order_write_64(endian: Endian, buffer: &mut Vec<u8>, data: u64) {
    let mut new_bytes = [0; 8];
    match endian {
        Endian::Big => BigEndian::write_u64(&mut new_bytes, data),
        Endian::Little => LittleEndian::write_u64(&mut new_bytes, data),
    };
    buffer.append(&mut new_bytes.to_vec());
}
