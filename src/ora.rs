//! Open Raster File Format
//!
//! Object types and functions needed to generate open raster files.
//! <https://www.openraster.org>
//!

use std::io::Write;
use std::path::Path;
use zip::{result::ZipResult, ZipWriter};

/// A piece of data in an open raster image
pub enum Element {
    /// Thumbnail of the image (256x256 max)
    Thumbnail(Vec<u8>),
    /// Merged image and background color of the image
    Composite((Vec<u8>, Layer)),
    /// Single layer of the composite image
    Layer(Layer),
}

/// Structure of data used to store layer image data and meta information
#[derive(Clone, Debug)]
pub struct Layer {
    /// Order in which this layer should be placed in the ORA image
    pub layer_number: u8,
    /// Layer image data (png file)
    pub image: Vec<u8>,
    /// Layer opacity
    pub opacity: f32,
    /// X position (in pixels) of the layer
    pub x_pos: u32,
    /// Y position (in pixels) of the layer
    pub y_pos: u32,
    /// Width (in pixels) of the layer
    pub width: u32,
    /// Height (in pixels)  of the layer
    pub height: u32,
}

impl Layer {
    /// Create a new layer object
    /// 
    /// # Example
    /// ```rust
    /// let layer = Layer::new(1, image, 0.5, 0, 0, 100, 100);
    /// ```
    pub fn new(
        layer_number: u8,
        image: Vec<u8>,
        opacity: f32,
        x_pos: u32,
        y_pos: u32,
        width: u32,
        height: u32,
    ) -> Layer {
        Layer {
            layer_number,
            image,
            opacity,
            x_pos,
            y_pos,
            width,
            height,
        }
    }
}

/// Open raster image
#[derive(Clone, Debug)]
pub struct Ora {
    /// The thumbnail image data (png format)
    pub thumbnail: Vec<u8>,
    /// Vector of layers that comprise the full image
    pub layers: Vec<Layer>,
    /// Merged (resultant) image
    pub merged_image: Vec<u8>,
    /// Image width (in pixels)
    pub width: u32,
    /// Image width (in pixels)
    pub height: u32,
}

impl Default for Ora {
    /// Create a new instance of Ora with default values
    ///
    /// # Example
    /// ```rust
    /// let mut ora = Ora::default();
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

impl Ora {
    /// Create a new open raster image object
    /// 
    /// # Example
    /// ```rust
    /// let mut ora = Ora::new();
    /// ```
    pub fn new() -> Ora {
        Ora {
            thumbnail: Vec::new(),
            layers: Vec::new(),
            merged_image: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    /// Add a layer to the image
    ///
    /// # Arguments
    ///
    /// `layer` - Layer to add to the image
    /// 
    /// # Example
    /// ```rust
    /// let mut ora = Ora::default();
    /// let image : Vec<u8> = vec![1, 2, 3, 4]; // this needs to be an actual png image
    /// let layer = Layer::new(1, image, 0.5, 0, 0, 100, 100);
    /// ora.add_layer(layer);
    /// 
    /// ```
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Write the image to a file
    ///
    /// # Arguments
    ///
    /// `path` - Reference to the path to save the image
    /// 
    /// # Example
    /// ```rust
    /// let file_path = Path::new("image.ora");
    /// let mut ora = Ora::new();
    ///  // Ideally one would do something to put data in the ora here before writing the file out
    /// ora.write_to_file(&file_path)?;
    /// ```
    pub fn write_to_file(&self, path: &Path) -> ZipResult<()> {
        let file = std::fs::File::create(&path).unwrap();

        let mut zip = ZipWriter::new(file);

        zip.start_file("mimetype", Default::default())?;
        zip.write_all(b"image/openraster")?;

        let mut layers_xml = String::new();

        for layer in &self.layers {
            if layer.layer_number > 0 {
                let layer_info = format!(
                    include_str!("ora_layer.xml"),
                    layer_number = layer.layer_number,
                    opacity = layer.opacity,
                    x_pos = layer.x_pos,
                    y_pos = self.height - layer.y_pos - layer.height,
                );

                layers_xml.push_str(&layer_info);
                layers_xml.push('\n');
            }
        }

        // write background layer after all the other layers
        let layer_info = format!(
            include_str!("ora_layer.xml"),
            layer_number = &self.layers[0].layer_number,
            opacity = &self.layers[0].opacity,
            x_pos = &self.layers[0].x_pos,
            y_pos = self.height - self.layers[0].y_pos - self.layers[0].height,
        );
        layers_xml.push_str(&layer_info);
        layers_xml.push('\n');

        let xml = format!(
            include_str!("ora_stack.xml"),
            width = self.width,
            height = self.height,
            resolution = 100,
            layers = layers_xml,
        );

        zip.start_file("stack.xml", Default::default())?;
        zip.write_all(xml.as_bytes())?;

        zip.start_file("mergedimage.png", Default::default())?;
        zip.write_all(&self.merged_image)?;

        zip.add_directory("data/", Default::default())?;
        for layer in &self.layers {
            zip.start_file(
                format!("data/layer{:?}.png", layer.layer_number),
                Default::default(),
            )?;
            zip.write_all(&layer.image)?;
        }

        zip.add_directory("Thumbnails/", Default::default())?;
        zip.start_file("Thumbnails/thumbnail.png", Default::default())?;
        zip.write_all(&self.thumbnail)?;

        zip.finish()?;
        Ok(())
    }
}
