use super::errors::FxError;

use std::{io::Read, path::Path};

const HEADER_JPG1: [u8; 4] = [0xff, 0xd8, 0xff, 0xdb];
const HEADER_JPG2: [u8; 4] = [0xff, 0xd8, 0xff, 0xe0];
const HEADER_JPG3: [u8; 4] = [0xff, 0xd8, 0xff, 0xee];
const HEADER_JPG_EXIF: [u8; 4] = [0xff, 0xd8, 0xff, 0xe1];
const HEADER_JPG_EXIF_AFTER: [u8; 6] = [0x45, 0x78, 0x69, 0x66, 0x00, 0x00];
const HEADER_PNG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const HEADER_GIF1: [u8; 6] = [0x47, 0x49, 0x46, 0x38, 0x37, 0x61];
const HEADER_GIF2: [u8; 6] = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
const HEADER_WEBP: [u8; 4] = [0x52, 0x49, 0x46, 0x46];
const HEADER_WEBP_AFTER: [u8; 4] = [0x57, 0x45, 0x42, 0x50];
const HEADER_TIF_LITTLE: [u8; 4] = [0x49, 0x49, 0x2A, 0x00];
const HEADER_TIF_BIG: [u8; 4] = [0x4D, 0x4D, 0x00, 0x2A];
const HEADER_BMP: [u8; 2] = [0x42, 0x4D];
const HEADER_ICO: [u8; 4] = [0x00, 0x00, 0x01, 0x00];
const HEADER_HDR: [u8; 11] = [
    0x23, 0x3f, 0x52, 0x41, 0x44, 0x49, 0x41, 0x4e, 0x43, 0x45, 0x0a,
];
const HEADER_EXR: [u8; 4] = [0x76, 0x2F, 0x31, 0x01];
const HEADER_PBM1: [u8; 3] = [0x50, 0x31, 0x0A];
const HEADER_PBM2: [u8; 3] = [0x50, 0x34, 0x0A];
const HEADER_PGM1: [u8; 3] = [0x50, 0x32, 0x0A];
const HEADER_PGM2: [u8; 3] = [0x50, 0x35, 0x0A];
const HEADER_PPM1: [u8; 3] = [0x50, 0x33, 0x0A];
const HEADER_PPM2: [u8; 3] = [0x50, 0x36, 0x0A];

#[derive(Debug, PartialEq, Eq)]
enum ImageSignature {
    Jpg,
    Png,
    Gif,
    Webp,
    Tif,
    Bmp,
    Ico,
    Hdr,
    Exr,
    Pbm,
    Pgm,
    Ppm,
    NotSupported,
}

fn inspect_image(p: &Path) -> Result<ImageSignature, FxError> {
    let mut file = std::fs::File::open(p)?;
    let mut buffer = [0; 12];
    file.read_exact(&mut buffer)?;

    let sign = if buffer[..4] == HEADER_JPG1
        || buffer[..4] == HEADER_JPG2
        || buffer[..4] == HEADER_JPG3
        || (buffer[..4] == HEADER_JPG_EXIF && buffer[6..] == HEADER_JPG_EXIF_AFTER)
    {
        ImageSignature::Jpg
    } else if buffer[..8] == HEADER_PNG {
        ImageSignature::Png
    } else if buffer[..6] == HEADER_GIF1 || buffer[..6] == HEADER_GIF2 {
        ImageSignature::Gif
    } else if buffer[..4] == HEADER_WEBP || buffer[8..] == HEADER_WEBP_AFTER {
        ImageSignature::Webp
    } else if buffer[..4] == HEADER_TIF_LITTLE || buffer[..4] == HEADER_TIF_BIG {
        ImageSignature::Tif
    } else if buffer[..2] == HEADER_BMP {
        ImageSignature::Bmp
    } else if buffer[..4] == HEADER_ICO {
        ImageSignature::Ico
    } else if buffer[..11] == HEADER_HDR {
        ImageSignature::Hdr
    } else if buffer[..4] == HEADER_EXR {
        ImageSignature::Exr
    } else if buffer[..3] == HEADER_PBM1 || buffer[..3] == HEADER_PBM2 {
        ImageSignature::Pbm
    } else if buffer[..3] == HEADER_PGM1 || buffer[..3] == HEADER_PGM2 {
        ImageSignature::Pgm
    } else if buffer[..3] == HEADER_PPM1 || buffer[..3] == HEADER_PPM2 {
        ImageSignature::Ppm
    } else {
        ImageSignature::NotSupported
    };

    Ok(sign)
}

pub fn is_supported_image_type(p: &Path) -> bool {
    if let Ok(sign) = inspect_image(p) {
        !matches!(sign, ImageSignature::NotSupported)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    /// Supported:
    /// jpg, png, gif, webp, tif, bmp, ico, hdr, exr, pbm, pgm, ppm
    fn test_inspect_image() {
        let p = PathBuf::from("testfiles/images/sample.jpg");
        assert_eq!(ImageSignature::Jpg, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.png");
        assert_eq!(ImageSignature::Png, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.gif");
        assert_eq!(ImageSignature::Gif, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.webp");
        assert_eq!(ImageSignature::Webp, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.tiff");
        assert_eq!(ImageSignature::Tif, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.bmp");
        assert_eq!(ImageSignature::Bmp, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.ico");
        assert_eq!(ImageSignature::Ico, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.hdr");
        assert_eq!(ImageSignature::Hdr, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.exr");
        assert_eq!(ImageSignature::Exr, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.pbm");
        assert_eq!(ImageSignature::Pbm, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.pgm");
        assert_eq!(ImageSignature::Pgm, inspect_image(&p).unwrap());

        let p = PathBuf::from("testfiles/images/sample.ppm");
        assert_eq!(ImageSignature::Ppm, inspect_image(&p).unwrap());
    }
}
