# skora #

Sketchbook Open Raster Library

This library provides functions to read and extract data from tiff files as created by Autodesk Sketchbook and export them as Open Raster files for use in Gimp or Krita.

The tiff files from Sketchbook include layers as seperate images (ifds) in private tiff tags.  Information about the specific tiff format can be found at https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html. The tiff parsing functionality of https://github.com/DigitalSlideArchive/tifftools was referenced when creating parts of this library.

## What's special about Sketchbook Tiffs? ##

Tiff files are used as a storage mechanism for Autodesk Sketchbook images.  Normally tiff files do not include layer information (ie they are single layer) but they do allow somewhat arbitrary data to be stored in them by including multiple IFDs (image file directory) in a single image file or by including additional data in Tags (which are stored inside IFDs).  Sketchbook takes advantage of this by storing a composite version of the image (all the layers merged) as the main image in the tiff file and putting all the layers (and thumbnail) in different IFDs inside the IFD of the main composite image.  This way, any program can open the tiff file and get the correct image, but if it doesn't support Sketchbook's specific way of manipulating tiffs for layers, then only the composite image shows up (ie the layers are lost).  As best I could find, there are no applications (other than Sketchbook) that support this tiff format.  While this isn't a 'normal' way to store layers, if what you are doing is documented its just as valid as anything else.  There is limited documentation about this format (noteably none from Autodesk directly) but https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html does document the format allowing for us to get all the layer information from the image as well.