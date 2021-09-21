//! Sketchbook Tiff
//!
//! This library provides functions to read and extract data from
//! tiff files as created by Autodesk Sketchbook.
//!
//! The tiff files from Sketchbook include layers as seperate images (ifds)
//! in private tiff tags.  Information about the specific tiff format can
//! be found at https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html.
//! The functionality in this library was inspired
//! by https://github.com/DigitalSlideArchive/tifftools.
//!

use std::collections::BTreeMap;
use std::error::Error;

use crate::endian_rw::{order_read, order_write_16, order_write_32, order_write_64, Endian};

use crate::tiff_types::{Data, DataType, Ifd, Info, Tag};

/// Read the non-image data from a TIFF.
///
/// # Arguments
///
/// * `file` - pointer to a byte slice containing the tiff
///
/// # Returns
///
/// a dictionary of information on the tiff file & a vector of IFDs
///
pub fn read_tiff(file: &[u8]) -> Result<(Info, Vec<Ifd>), String> {
    let file_size = file.len();
    //println!("File size : {:?} bytes", size);

    // read the file header
    let header: Vec<u8> = file[0..4].into();

    // Verify this is a tiff image
    match header[..] {
        [0x49, 0x49, 0x2a, 0x00] => {} //b'II\x2a\x00',
        [0x4D, 0x4D, 0x00, 0x2a] => {} //b'MM\x00\x2a'
        [0x49, 0x49, 0x2b, 0x00] => {} //b'II\x2b\x00'
        [0x4D, 0x4D, 0x00, 0x2b] => {} //b'MM\x00\x2b'
        _ => return Err("The header is not valid".to_string()),
    }

    // Read which endian encoding is used
    let endian = match header[0] == 0x4D && header[1] == 0x4D {
        true => Endian::Big,
        false => Endian::Little,
    };

    // Read if this is a 'big tiff' image
    let big_tiff = header[2] == 0x2B && header[4] == 0x08;

    // Get the first ifd location of the source tiff image
    // Big tiff sets the first offset to 8 then writes the actual offset to the next 4 bytes
    // Regular tiff just puts the offset in the 4 bytes of the header
    let first_ifd = match big_tiff {
        true => {
            let offset_size = order_read(endian, &file[4..6], 2);
            match offset_size == 8 {
                true => order_read(endian, &file[8..12], 4) as usize,
                false => return Err("Unexpected big tiff offset size".to_string()),
            }
        }
        false => order_read(endian, &file[4..8], 4) as usize,
    };

    // Define the file Info struct
    let mut info: Info = Info {
        endian,
        big_tiff,
        first_ifd,
        header,
        size: file_size,
    };

    // Initialize a list of IFDs to store the main ifd and all sub ifds (layers) from the original tiff image
    let mut ifd_list: Vec<Ifd> = [].to_vec();

    // Recursively read all IDFs in the image
    let mut next_ifd = first_ifd;
    while next_ifd > 0 {
        next_ifd = read_ifd(file, &mut info, next_ifd, &mut ifd_list).unwrap();
    }

    // Return info
    Ok((info, ifd_list))
}

/// Read an IFD and any subIFDs.
///
/// # Arguments
///
/// * `file` - Rerference to the bytes of the tiff file
/// * `info` - Info about the tiff file
/// * `ifd_offset` - Offset of the ifd to read (from the start of the file vector)
/// * `ifd_list` - Reference to a Ifd vector used to store the image data
///
pub fn read_ifd(
    file: &[u8],
    info: &mut Info,
    ifd_offset: usize,
    ifd_list: &mut Vec<Ifd>,
) -> Option<usize> {
    let length = match info.big_tiff {
        true => 16,
        false => 6,
    };
    if !check_offset(info.size, ifd_offset, length) {
        return None;
    }
    let mut offset: usize = ifd_offset;

    let mut ifd: Ifd = Ifd {
        offset,
        tags: BTreeMap::new(),
        size: info.size,
        endian: info.endian,
        big_tiff: info.big_tiff,
        tag_count: 0,
    };

    match info.big_tiff {
        true => {
            ifd.tag_count = order_read(info.endian, &file[offset..offset + 8], 8);
            offset += 8;
        }
        false => {
            ifd.tag_count = order_read(info.endian, &file[offset..offset + 2], 2);
            offset += 2;
        }
    }

    for _entry in 0..(ifd.tag_count) {
        let mut tag = 0;
        let mut datatype = DataType::Byte; // initialize data at a byte array.  this gets updated in read_tags
        let mut count = 0;
        let data: Data;
        let mut data_tmp: u64 = 0;
        let mut data_length = 0;

        match info.big_tiff {
            true => {
                tag = order_read(info.endian, &file[offset..offset + 2], 2);
                offset += 2;

                datatype = (order_read(info.endian, &file[offset..offset + 2], 2) as u16).into();
                offset += 2;

                count = order_read(info.endian, &file[offset..offset + 8], 8);
                offset += 8;

                data_tmp = order_read(info.endian, &file[offset..offset + 8], 8);
                data = match info.endian {
                    Endian::Big => Data::Byte(data_tmp.to_be_bytes().into()),
                    Endian::Little => Data::Byte(data_tmp.to_le_bytes().into()),
                };

                offset += 8;

                data_length = 8;
            }
            false => {
                tag = order_read(info.endian, &file[offset..offset + 2], 2);
                offset += 2;

                datatype = (order_read(info.endian, &file[offset..offset + 2], 2) as u16).into();
                offset += 2;

                count = order_read(info.endian, &file[offset..offset + 4], 4);
                offset += 4;

                data_tmp = order_read(info.endian, &file[offset..offset + 4], 4); //always returns 8 bytes (u64)
                data = match info.endian {
                    Endian::Big => Data::Byte(data_tmp.to_be_bytes().into()),
                    Endian::Little => Data::Byte(data_tmp.to_le_bytes().into()),
                };
                offset += 4;

                data_length = 4;
            }
        }

        let data_element_size = datatype.element_size_in_bytes();
        let mut tag_info: Tag = Tag {
            count,
            data,
            datapos: offset - data_length,
            datatype,
            ifds: None,
            offset: None,
        };

        if (count * data_element_size) as usize > data_length {
            tag_info.offset = Some(data_tmp as usize);
        }

        if ifd.tags.contains_key(&tag) {
            println!(
                "Duplicate tag {:?}: data at {:?} and {:?}",
                tag, ifd.tags[&tag].datapos, tag_info.datapos
            );
        }

        ifd.tags.insert(tag, tag_info);
    }

    let next_ifd = match info.big_tiff {
        true => order_read(info.endian, &file[offset..offset + 8], 8) as usize,
        false => order_read(info.endian, &file[offset..offset + 4], 4) as usize,
    };

    read_ifd_tag_data(file, info, &mut ifd, ifd_list);
    ifd_list.push(ifd);

    Some(next_ifd)
}

/// Read all data from the tags of an IFD; read subifds.
///
/// # Arguments
///
/// * `file` - Rerference to the bytes of the tiff file
/// * `info` - Info about the tiff file
/// * `ifd` - Reference to an Idf to read the tag data from
/// * `ifd_list` - Reference to an Ifd vector used to store the image data
///
pub fn read_ifd_tag_data(file: &[u8], info: &mut Info, ifd: &mut Ifd, ifd_list: &mut Vec<Ifd>) {
    for (tag_num, tag_info) in ifd.tags.iter_mut() {
        let tag = *tag_num;
        let type_size = tag_info.datatype.element_size_in_bytes();

        // second param is the default value in case offset doesnt exist
        let pos = tag_info.offset.unwrap_or(tag_info.datapos);

        let offset = pos;

        let byte_count = (tag_info.count * type_size) as usize;

        if !check_offset(info.size, pos, byte_count as usize) {
            println!(
                "OMG Its gone wrong - size {:?} offset {:?} length {:?}",
                info.size, pos, byte_count
            );
        }

        let raw_data = file[offset..(offset + byte_count)].to_vec();

        tag_info.data = Data::new(raw_data, tag_info.datatype, info.endian, tag_info.count);

        if tag == 330 {
            tag_info.ifds = Some(Vec::new());
            if let Data::Long(sub_ifd_offsets) = tag_info.data.clone() {
                for (_sub_idx, sud_ifd_offset) in sub_ifd_offsets.iter().enumerate() {
                    let mut next_ifd = *sud_ifd_offset as usize;
                    while next_ifd > 0 {
                        next_ifd = read_ifd(file, info, next_ifd, ifd_list).unwrap();
                    }
                }
            }
        }
    }
}

/// Check if a specific number of bytes can be read from a file at a given offset.
///
/// # Arguments
///
/// * `source_length` - the length of the source ifd or file
/// * `offset` - an offset from the start of the file
/// * `length` - the number of bytes to read
///
/// # Returns
///  True if the offset and length are possible, false if not.
///
pub fn check_offset(source_length: usize, offset: usize, length: usize) -> bool {
    // def check_offset(filelen, offset, length):
    //     # The minimum offset is the length of the tiff header
    //     allowed = offset >= 8 and length >= 0 and offset + length <= filelen
    let allowed = (offset >= 8) & (offset + length <= source_length);

    if !allowed {
        println!(
            "Cannot read {:?} bytes from desired offset {:?}.",
            length, offset
        );
    }
    //     return allowed
    allowed
}

/// Get the layers embedded in the tiff file based on data in a list of ifds
///
/// # Arguments
///
/// * `ifds` - A list of IFDs
/// * `source` - The bytes of the original tiff image
///
/// # Returns
///
/// Vector of layers where each layer is a vector of bytes describing a tiff file
///
pub fn get_layers(ifds: Vec<Ifd>, source: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    // Initialize and output vector
    let mut layers: Vec<Vec<u8>> = Vec::new();

    // Loop through each IDF in the input and create an image for it
    for ifd in ifds {
        let mut image: Vec<u8> = Vec::new();

        let endian = ifd.endian;
        let big_tiff = ifd.big_tiff;

        // Initialize the image header
        let mut header = match big_tiff {
            true => {
                // header = b'MM'
                let mut hdr = vec![0x4D, 0x4D];
                order_write_16(endian, &mut hdr, 0x2B_u16);
                order_write_16(endian, &mut hdr, 8_u16);
                order_write_16(endian, &mut hdr, 0_u16);
                order_write_64(endian, &mut hdr, 0_u64);
                // Cut off the last 8 bytes which are the pointer to the first IFD.  These will be added back in from copy_ifd
                hdr
            }
            false => {
                // header = b'II'
                let mut hdr = vec![0x49, 0x49];
                order_write_16(endian, &mut hdr, 0x2A_u16);
                order_write_32(endian, &mut hdr, 0_u32);
                // Cut off the last 4 bytes which are the pointer to the first IFD.  These will be added back in from copy_ifd
                hdr
            }
        };

        // Get a pointer to the ifd location in the header
        let ifd_pointer = match big_tiff {
            true => header.len() - 8,
            false => header.len() - 4,
        };

        // Add the header to the image
        image.append(&mut header);

        // Add the ifds to the image
        copy_ifd(&mut image, ifd, ifd_pointer, source);

        // Add this image to the vector of layer images
        layers.push(image);
    }
    Ok(layers)
}

/// Write an IFD to a TIFF file.  This copies image data from other tiff files.
///
/// # Arguments
///
/// `image` - Reference to the byte vector where the image is being constructed
/// `ifd` - The ifd record to add to the image.
/// `ifd_pointer` - a location to write the value of this ifd's start
/// `source` - Reference to the byte slice containing the original tiff
///
pub fn copy_ifd(image: &mut Vec<u8>, ifd: Ifd, ifd_pointer: usize, source: &[u8]) {
    let tag_data_length = match ifd.big_tiff {
        true => 8,
        false => 4,
    };

    let mut ifd_record: Vec<u8> = Vec::new();

    match ifd.big_tiff {
        true => order_write_64(ifd.endian, &mut ifd_record, ifd.tags.len() as u64),
        false => order_write_16(ifd.endian, &mut ifd_record, ifd.tags.len() as u16),
    }

    for tag_num in ifd.tags.keys() {
        // keys returns a sorted (numerically) list of keys
        // because we are only running through the ifd_list we can never have nested ifds here

        let mut tag_info = ifd.tags[tag_num].clone();
        let mut data = tag_info.data.clone();

        // count = len(data)
        let count = tag_info.count;

        // 273: {'name': 'StripOffsets', 'datatype': (Datatype.SHORT, Datatype.LONG, Datatype.LONG8), 'bytecounts': 'StripByteCounts', 'desc': 'The byte offset of each strip with respect to the beginning of the TIFF file'},
        // 279: {'name': 'StripByteCounts', 'datatype': (Datatype.SHORT, Datatype.LONG, Datatype.LONG8), 'desc': 'For each strip, the number of bytes in the strip after compression'},
        // 288: {'name': 'FreeOffsets', 'datatype': (Datatype.LONG, Datatype.LONG8), 'bytecounts': 'FreeByteCounts', 'desc': 'For each string of contiguous unused bytes in a TIFF file, the byte offset of the string'},
        // 289: {'name': 'FreeByteCounts', 'datatype': (Datatype.LONG, Datatype.LONG8), 'desc': 'For each string of contiguous unused bytes in a TIFF file, the number of bytes in the string'},
        // 324: {'name': 'TileOffsets', 'datatype': (Datatype.LONG, Datatype.LONG8), 'bytecounts': 'TileByteCounts', 'desc': 'For each tile, the byte offset of that tile'},
        // 325: {'name': 'TileByteCounts', 'datatype': (Datatype.LONG, Datatype.LONG8), 'desc': 'For each tile, the number of (compressed) bytes in that tile'},
        // 513: {'name': 'JPEGIFOffset', 'datatype': (Datatype.LONG, Datatype.LONG8), 'count': 1, 'bytecounts': 'JPEGIFByteCount'},
        // 514: {'name': 'JPEGIFByteCount', 'datatype': (Datatype.LONG, Datatype.LONG8), 'count': 1},
        // 519: {'name': 'JPEGQTables', 'datatype': (Datatype.LONG, Datatype.LONG8), 'bytecounts': 64},
        // 520: {'name': 'JPEGDCTables', 'datatype': (Datatype.LONG, Datatype.LONG8), 'bytecounts': 16 + 17},
        // 521: {'name': 'JPEGACTables', 'datatype': (Datatype.LONG, Datatype.LONG8), 'bytecounts': 16 + 256},

        match tag_num {
            // if tag.isOffsetData():
            273 | 288 | 324 | 513 | 519 | 520 | 521 => {
                // if isinstance(tag.bytecounts, str):
                let ref_lengths = match tag_num {
                    273 => {
                        if let Data::Long(val) = ifd.tags[&279].data.clone() {
                            val
                        } else {
                            panic!()
                        }
                    }
                    288 => {
                        if let Data::Long(val) = ifd.tags[&289].data.clone() {
                            val
                        } else {
                            panic!()
                        }
                    }
                    324 => {
                        if let Data::Long(val) = ifd.tags[&325].data.clone() {
                            val
                        } else {
                            panic!()
                        }
                    }
                    513 => {
                        if let Data::Long(val) = ifd.tags[&514].data.clone() {
                            val
                        } else {
                            panic!()
                        }
                    }
                    _ => vec![tag_info.datatype.element_size_in_bytes() as u32; count as usize],
                };

                let offsets_list = match data {
                    Data::Long(val) => val, // all the data types in isOffsetData are Long
                    _ => panic!(),
                };

                // data = write_tag_data(dest, src, data, ifd['tags'][int(tagSet[tag.bytecounts])]['data'], ifd['size'])
                // or
                // data = write_tag_data(dest, src, data, [tag.bytecounts] * count, ifd['size'])
                // depending on if bytecounts is a string (278, 288, 324, 513)
                data = Data::Long(copy_tag_data(
                    image,
                    source,
                    offsets_list,
                    ref_lengths,
                    ifd.size,
                ));

                tag_info.datatype = DataType::Long;
            }
            _ => {}
        }

        let mut data_output: Vec<u8> = Vec::new();

        data_output = data.to_vec_u8(ifd.endian);

        let mut tag_record: Vec<u8> = Vec::new();
        order_write_16(ifd.endian, &mut tag_record, *tag_num as u16);
        order_write_16(
            ifd.endian,
            &mut tag_record,
            tag_info.datatype.type_tiff_id() as u16,
        );
        match tag_data_length {
            4 => {
                order_write_32(ifd.endian, &mut tag_record, count as u32);
            }
            8 => {
                order_write_64(ifd.endian, &mut tag_record, count);
            }
            _ => panic!(),
        }

        if data_output.len() <= tag_data_length {
            let data_output_len = data_output.len();
            tag_record.append(&mut data_output); // put data in record
            tag_record.append(&mut vec![0; tag_data_length - data_output_len]); // pad with zeros
        } else {
            // # word alignment
            if image.len() % 2 == 1 {
                image.push(0);
            }

            match tag_data_length {
                4 => {
                    order_write_32(ifd.endian, &mut tag_record, image.len() as u32);
                }
                8 => {
                    order_write_64(ifd.endian, &mut tag_record, image.len() as u64);
                }
                _ => panic!(),
            }

            image.append(&mut data_output);
        }

        ifd_record.append(&mut tag_record);
    }

    let mut pos = image.len();
    // # ifds are expected to be on word boundaries
    if pos % 2 == 1 {
        image.push(0);
        pos = image.len();
    }

    // Go to the ifd pointer location in the destination data and overwrite the value there with our new position value
    let mut pos_bytes: Vec<u8> = Vec::new(); // get position value as vector of bytes
    match tag_data_length {
        4 => {
            order_write_32(ifd.endian, &mut pos_bytes, pos as u32);
        }
        8 => {
            order_write_64(ifd.endian, &mut pos_bytes, pos as u64);
        }
        _ => panic!(),
    }

    // Write the posititon value back to the end of the header
    for (idx, byte) in pos_bytes.iter().enumerate() {
        image[ifd_pointer + idx] = *byte;
    }

    // Add the ifd to the image
    image.append(&mut ifd_record);

    let mut temp: Vec<u8> = Vec::new();
    match tag_data_length {
        4 => {
            order_write_32(ifd.endian, &mut temp, 0);
        }
        8 => {
            order_write_64(ifd.endian, &mut temp, 0);
        }
        _ => panic!(),
    }
    image.append(&mut temp);
}

/// Copy data from a source tiff to a destination tiff, return a list of offsets where data was written.
///
/// # Arguments
///
/// `image` - Reference to the destination image
/// `source` - The source tiff byte slice
/// `offsets` - A vector of offsets where data will be copied from
/// `lengths` - A vector of lengths to copy from each offset
/// `source_length` - The length of the source ifd
///
/// # Returns
///
/// The offsets in the destination file corresponding to the data copied
///
pub fn copy_tag_data(
    image: &mut Vec<u8>,
    source: &[u8],
    offsets: Vec<u32>,
    lengths: Vec<u32>,
    source_length: usize,
) -> Vec<u32> {
    if offsets.len() != lengths.len() {
        println!("Offsets and byte counts do not correspond.");
    }

    let mut dest_offsets = vec![0; offsets.len()];
    // because we are doing things are bit differently we need to init the first location with the current dest length
    dest_offsets[0] = image.len() as u32;

    // # We preserve the order of the chunks from the original file
    let offset_list = offsets.clone();
    let idx_list: Vec<usize> = (0..offsets.len()).collect();

    let mut olidx = 0;

    while olidx < offset_list.len() {
        let offset = offset_list[olidx];
        let idx = idx_list[olidx];

        let length = lengths[idx];

        // if offset and check_offset(srclen, offset, length):
        if check_offset(source_length, offset as usize, length as usize) {
            let start = offset as usize;
            let end = (offset + length) as usize;
            let source_data = &source[start..end];
            image.append(&mut source_data.to_vec());
        }

        olidx += 1;
    }

    for (index, _val) in lengths.iter().enumerate() {
        if index > 0 {
            dest_offsets[index] = dest_offsets[index - 1] + lengths[index - 1]
        }
    }

    dest_offsets
}
