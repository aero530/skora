# skora #

Sketchbook Open Raster Library

This library provides functions to read and extract data from tiff files as created by [Autodesk Sketchbook](https://www.sketchbook.com) and export them as [Open raster](https://www.openraster.org) files that can be opened / modified in Gimp or Krita.  Notably thie is done while retaining layer information.

The tiff files from Sketchbook include layers as seperate images (ifds) in private tiff tags.  Information about the specific tiff format can be found at [Aware Systems](https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html). The tiff parsing functionality of [TiffTools](https://github.com/DigitalSlideArchive/tifftools) was referenced when creating parts of this library.

## What's special about Sketchbook Tiffs? ##

Tiff files are used as a storage mechanism for Autodesk Sketchbook images.  Normally tiff files do not include layer information (ie they are single layer) but they do allow somewhat arbitrary data to be stored in them by including multiple IFDs (image file directory) in a single image file or by including additional data in Tags (which are stored inside IFDs).  Sketchbook takes advantage of this by storing a composite version of the image (all the layers merged) as the main image in the tiff file and putting all the layers (and thumbnail) in different IFDs inside the IFD of the main composite image.  This way, any program can open the tiff file and get the correct image, but if it doesn't support Sketchbook's specific way of manipulating tiffs for layers, then only the composite image shows up (ie the layers are lost).

As best I could find, there are no applications (other than Sketchbook) that support this tiff format.  While this isn't a 'normal' way to store layers, if what you are doing is documented its just as valid as anything else.  As it turns out there is limited documentation about this format (noteably none from Autodesk directly).  The tag used to specify this proprietary format is called `Alias Layer Metadata` and there is a bit of documentation for the image format at [Aware Systems](https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html).

## Usage ##

This library was primarily made in service of a small command line app to convert files.  If you just want to convert some files then [SketchbookTiffConverter](https://github.com/aero530/SketchbookTiffConverter) is probably what you actually want.  If you want to make your own app to convert files then the library might help you out.  As the library was made in service of SketchbookTiffConverter, the main functionality of this library is wrapped up into the `convert_file` function.  That is the best place to start if you are hoping to convert some files in your app.  All the relevant sub-functions are exposed so you can go to any level you want to in processing your own files.
