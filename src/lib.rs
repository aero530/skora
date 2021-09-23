//! # Sketchbook Open Raster Library #
//!
//! This library provides functions to read and extract data from tiff files as created by 
//! [Autodesk Sketchbook](https://www.sketchbook.com) and export them as [Open raster](https://www.openraster.org) 
//! files that can be opened / modified in Gimp or Krita.  Notably thie is done while retaining layer information.
//! 
//! The tiff files from Sketchbook include layers as seperate images (ifds) in private tiff tags.  Information about 
//! the specific tiff format can be found at [Aware Systems](https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html). 
//! The tiff parsing functionality of [TiffTools](https://github.com/DigitalSlideArchive/tifftools) was 
//! referenced when creating parts of this library.
//! 
//! ## What's special about Sketchbook Tiffs? ##
//! 
//! Tiff files are used as a storage mechanism for Autodesk Sketchbook images.  Normally tiff files do not include layer information 
//! (ie they are single layer) but they do allow somewhat arbitrary data to be stored in them by including multiple IFDs 
//! (image file directory) in a single image file or by including additional data in Tags (which are stored inside IFDs).  
//! Sketchbook takes advantage of this by storing a composite version of the image (all the layers merged) as the main image 
//! in the tiff file and putting all the layers (and thumbnail) in different IFDs inside the IFD of the main composite image.  
//! This way, any program can open the tiff file and get the correct image, but if it doesn't support Sketchbook's specific way 
//! of manipulating tiffs for layers, then only the composite image shows up (ie the layers are lost).
//! 
//! As best I could find, there are no applications (other than Sketchbook) that support this tiff format.  While this isn't 
//! a 'normal' way to store layers, if what you are doing is documented its just as valid as anything else.  As it turns out 
//! there is limited documentation about this format (noteably none from Autodesk directly).  The tag used to specify this 
//! proprietary format is called `Alias Layer Metadata` and there is a bit of documentation for the image format at 
//! [Aware Systems](https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html).
//! 
//! ## Usage ##
//! 
//! This library was primarily made in service of a small command line app to convert files.  If you just want to convert some files
//! then [SketchbookTiffConverter](https://github.com/aero530/SketchbookTiffConverter) is probably what you actually want.  If you want
//! to make your own app to convert files then the library might help you out.  As the library was made in service of SketchbookTiffConverter,
//! the main functionality of this library is wrapped up into the `convert_file` function.  That is the best place to start if you are
//! hoping to convert some files in your app.  All the relevant sub-functions are exposed so you can go to any level you want to in processing 
//! your own files.


use std::error::Error;

use hex::FromHex;
use image::load_from_memory;
use image::{DynamicImage, GenericImageView, ImageBuffer};
use pretty_hex::simple_hex;
use log::{info,debug,trace,error};

use std::fs;
use std::path::Path;
use std::io::prelude::*;

pub mod ora;
use crate::ora::{Element, Ora};

pub mod tiff;
pub mod tiff_types;
use crate::tiff_types::{Data, Ifd};

pub mod endian_rw;

/// Convert a Sketchbook Tiff file to an Open Raster file
///
/// # Arguments
///
/// * `file_path_string` - String filepath.  Can be either a tiff file or a directory.
///   If it is a tiff file, just that file is converted.  If it is a directory, then all
///   tiff files in the directory will be converted.
///
/// # Returns
///
/// * Ok or error
///
/// # Example
///
/// ```rust
/// use std::error::Error;
/// use skora;
/// fn main() -> Result<(), Box<dyn Error>> {
///     let path_string = "test.tiff";
///     println!("Processing file {}", path_string);
///     skora::convert_file(path_string)?;
/// }
/// ```
pub fn convert_file(file_path_string: String, export_tiff: bool) -> Result<String, Box<dyn Error>> {
    let file_path = Path::new(&file_path_string);
    let file = fs::read(file_path).unwrap();

    let (info, ifds) = tiff::read_tiff(&file).unwrap();

    info!("File size : {}", info.size);
    debug!("Header : {:?}", simple_hex(&info.header));
    trace!("tiff info : {:#?}",info);
    trace!("tiff ifds : {:#?}",ifds);

    let images: Vec<Vec<u8>> = tiff::get_layers(ifds.clone(), &file)?;

    let mut ora = Ora::default();

    // iterate through the list of images backwards as sketchbook saved the layers with the bottom most layer first in the image.
    // this is so we end up with the right order in the ora file.
    for (idx, image_file) in images.iter().rev().enumerate() {

        if export_tiff {
            // get file name without path info
            let layer_stem = file_path.file_stem().unwrap().to_str().unwrap();

            // get file path and add 'layers' directory to it
            let layer_parent = file_path.parent().unwrap().clone().join("layers");

            // create the layers directory if it doesn't exist
            fs::create_dir_all(layer_parent.clone())?;

            // create the file path for this layer
            let layer_path = layer_parent.join(format!("{}_layer_{}.tiff",layer_stem,idx));

            debug!("Writing tiff layer to {:?}",layer_path);
            let mut layer_file = std::fs::File::create(layer_path).unwrap();
            layer_file.write_all(image_file)?;
        }

        let ifd = &ifds[ifds.len() - 1 - idx];

        match ifd_to_ora_element(idx, ifd, image_file)? {
            Element::Thumbnail(val) => {
                ora.thumbnail = val;
            }
            Element::Composite(val) => {
                ora.width = val.1.width;
                ora.height = val.1.height;
                ora.merged_image = val.0;
                ora.add_layer(val.1);
            }
            Element::Layer(val) => {
                ora.add_layer(val);
            }
        };
    }
    let new_path = file_path.with_extension("ora");
    ora.write_to_file(&new_path)?;
    Ok("done".to_string())
}

/// Create a piece of an ora file (composite, layer, thumbnail) for the given piece of a tiff file (ifd)
///
/// # Arguments
///
/// * `layer_number` - Layer number
/// * `ifd` - Reference to the ifd data
/// * `image_file` - Reference to a tiff image for this ifd
///
/// # Returns
///
/// * Ora element for the given tiff image file directory
pub fn ifd_to_ora_element(
    layer_number: usize,
    ifd: &Ifd,
    image_file: &[u8]
) -> Result<ora::Element, Box<dyn Error>> {
    let mut is_composite = false;
    let mut is_thumbnail = false;

    if ifd.tags.contains_key(&305) {
        let thing = if let Data::Ascii(val) = ifd.tags[&305].data.clone() {
            val
        } else {
            String::from("")
        };
        if thing.eq(&"Alias MultiLayer TIFF V1.1\u{0}".to_string()) {
            is_composite = true;
        }
    }

    if ifd.tags.contains_key(&254) {
        let thing = if let Data::Long(val) = ifd.tags[&254].data.clone() {
            val
        } else {
            vec![0]
        };

        if thing[0] == 1 {
            is_thumbnail = true;
        } else {
            error!("  THERE WAS AN ERROR");
        }
    }

    let image = load_from_memory(image_file);
    let image = match image {
        Ok(file) => file,
        Err(error) => {
            // if we can't load the image then make a small image file so we can keep processing the rest of the layers
            error!("{}", error);
            let i = ImageBuffer::new(10, 10);
            let img: DynamicImage = DynamicImage::ImageRgba8(i);
            img
        } //panic!("Problem loading tiff layer {} from memory: {:?}", layer_number, error),
    };

    if is_thumbnail {
        trace!("This is a reduced resolution image (thumbnail)");
        Ok(ora::Element::Thumbnail(image_to_buf(image.to_rgba8())?))
    } else {
        let mut alias: String;
        let mut alias_values: Vec<&str> = Vec::new();

        if ifd.tags.contains_key(&50784) {
            alias = match ifd.tags[&50784].data.clone() {
                Data::Ascii(val) => val,
                _ => String::new(),
            };
            alias.pop(); // remove trailing char
            alias_values = alias.split(", ").collect();
        }

        if is_composite {
            trace!("This is a composite image ifd");
            let layer_count = alias_values[0];
            let current_layer = alias_values[1];
            let background_color = alias_values[2];
            let reduced_image_count = alias_values[3];
            info!("LayerCount: {}, CurrentLayer: {}, BackgroundColor: {}, ReducedImageCount (# thumbnails): {}", layer_count, current_layer,background_color, reduced_image_count);

            let colors = <[u8; 4]>::from_hex(background_color).expect("Decoding failed"); // this is ARGB from the tiff tag data per Alias Layer Metadata

            let width = image.width();
            let height = image.height();
            let background = fill_color(image.clone(), colors)?;

            let background = ora::Layer::new(
                layer_number as u8,
                image_to_buf(background)?,
                1.0,
                0_u32,
                0_u32,
                width,
                height,
            );

            Ok(ora::Element::Composite((
                image_to_buf(image.to_rgba8())?,
                background,
            )))
        } else {
            trace!("This is a layer ifd");
            let layer_opacity = alias_values[0];
            let layer_fill_color = alias_values[1];
            let layer_visible = alias_values[2];
            let layer_locked = alias_values[3];
            let layer_name_image_present = alias_values[4];
            let visibility_channel_count = alias_values[5];
            let mask_layer_count = alias_values[6];
            debug!("Layer Opacity: {}, Layer Fill Color: {}, Layer Visible: {}, Layer Locked: {}, Layer Name Image Present: {}, Visibility Channel Count: {}, Mask Layer Count: {}", layer_opacity, layer_fill_color, layer_visible, layer_locked, layer_name_image_present, visibility_channel_count, mask_layer_count);

            let mut x_pos: f64 = 0.0;
            let mut y_pos: f64 = 0.0;
            if ifd.tags.contains_key(&286) {
                x_pos = if let Data::Rational(val) = ifd.tags[&286].data.clone() {
                    val[0].0
                } else {
                    0.0
                };
            }
            if ifd.tags.contains_key(&287) {
                y_pos = if let Data::Rational(val) = ifd.tags[&287].data.clone() {
                    val[0].0
                } else {
                    0.0
                };
            }

            let mut better = bgra_to_rgba(image)?;
            image::imageops::flip_vertical_in_place(&mut better);

            let width = better.width();
            let height = better.height();

            let layer = ora::Layer::new(
                layer_number as u8,
                image_to_buf(better)?,
                layer_opacity.parse::<f32>()?,
                x_pos as u32,
                y_pos as u32,
                width,
                height,
            );
            Ok(ora::Element::Layer(layer))
        }
    }
}

/// Export an Image buffer to a png
///
/// # Arguments
///
/// * `input` - Image buffer to convert
///
/// # Returns
///
/// * PNG file of the image stored as a vector of u8 bytes
pub fn image_to_buf(
    input: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf: Vec<u8> = vec![];
    let img: DynamicImage = DynamicImage::ImageRgba8(input);
    img.write_to(&mut buf, image::ImageOutputFormat::Png)?;
    Ok(buf)
}

/// Convert BGRA image to RGBA
///
/// For whatever reason Sketchbook layers are stored in BGRA while the composite and thumbnail are RGBA.
/// In addition, the layers are stored with RGB premultiplied by alpha.  
/// This function swaps the B & R values for each pixel and removes the 'premultiplied alpha' scaling
/// ie divides each channel by alpha.
///
/// # Arguments
///
/// * `input` - Dynamic image to fill with some specified color
/// * `color_argb` - Slice denoting the alpha, red, green, blue channels to add as the image color
///
/// # Returns
///
/// * Image buffer converted to RGBA
pub fn bgra_to_rgba(
    input: DynamicImage,
) -> Result<ImageBuffer<image::Rgba<u8>, Vec<u8>>, Box<dyn Error>> {
    let (width, height) = input.dimensions();
    let mut buf = input.into_bytes();
    // The 4 u8's foe each pixel are packed together in a Ve so we iterate through in groups of 4 to extract each pixel
    buf.chunks_mut(4).for_each(|pixel| {
        let temp = pixel[0];
        let alpha = pixel[3] as f64 / 255.0; // this alpha is now between 0 and 1
        pixel[0] = (pixel[2] as f64 / alpha) as u8;
        pixel[1] = (pixel[1] as f64 / alpha) as u8;
        pixel[2] = (temp as f64 / alpha) as u8;
    });
    match image::RgbaImage::from_raw(width, height, buf) {
        Some(output) => Ok(output),
        None => panic!(),
    }
}

/// Fill a dynamic image with a specified color
///
/// # Arguments
///
/// * `input` - Dynamic image to fill with some specified color
/// * `color_argb` - Slice denoting the alpha, red, green, blue channels to add as the image color
///
/// # Returns
///
/// * Image buffer filled with the specified color
pub fn fill_color(
    input: DynamicImage,
    color_argb: [u8; 4],
) -> Result<ImageBuffer<image::Rgba<u8>, Vec<u8>>, Box<dyn Error>> {
    let (width, height) = input.dimensions();
    let mut output = input.into_bytes();
    output.chunks_mut(4).for_each(|pixel| {
        pixel[0] = color_argb[1]; // red
        pixel[1] = color_argb[2]; // green
        pixel[2] = color_argb[3]; // blue
        pixel[3] = color_argb[0]; // alpha
    });
    match image::RgbaImage::from_raw(width, height, output) {
        Some(val) => Ok(val),
        None => panic!(),
    }
}
