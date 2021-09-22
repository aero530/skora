# skora #

Sketchbook Open Raster Library

This library provides functions to read and extract data from tiff files as created by Autodesk Sketchbook and export them as Open Raster files for use in Gimp or Krita.

The tiff files from Sketchbook include layers as seperate images (ifds) in private tiff tags.  Information about the specific tiff format can be found at https://www.awaresystems.be/imaging/tiff/tifftags/docs/alias.html. The tiff parsing functionality of https://github.com/DigitalSlideArchive/tifftools was referenced when creating parts of this library.
