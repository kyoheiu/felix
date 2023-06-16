/// Based on [List of file signatures - Wikipedia](https://en.wikipedia.org/wiki/List_of_file_signatures)
use super::errors::FxError;
use std::io::Read;
use std::path::Path;

const HEADER_GZIP: [u8; 2] = [0x1F, 0x8B];
const HEADER_XZ: [u8; 6] = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00];
const HEADER_ZSTD: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
const HEADER_TAR1: [u8; 8] = [0x75, 0x73, 0x74, 0x61, 0x72, 0x00, 0x30, 0x30];
const HEADER_TAR2: [u8; 8] = [0x75, 0x73, 0x74, 0x61, 0x72, 0x20, 0x20, 0x00];
const HEADER_PKZIP: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
const HEADER_TARZ_LZW: [u8; 2] = [0x1F, 0x9D];
const HEADER_TARZ_LZH: [u8; 2] = [0x1F, 0xA0];
const HEADER_SEVENZ: [u8; 6] = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];
const HEADER_LZH0: [u8; 5] = [0x2D, 0x68, 0x6C, 0x30, 0x2D];
const HEADER_LZH5: [u8; 5] = [0x2D, 0x68, 0x6C, 0x35, 0x2D];
const HEADER_BZ2: [u8; 3] = [0x42, 0x5A, 0x68];
const HEADER_RNC1: [u8; 4] = [0x52, 0x4E, 0x43, 0x01];
const HEADER_RNC2: [u8; 4] = [0x52, 0x4E, 0x43, 0x02];
const HEADER_LZIP: [u8; 4] = [0x4C, 0x5A, 0x49, 0x50];
const HEADER_RAR1: [u8; 7] = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00];
const HEADER_RAR5: [u8; 8] = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];
const HEADER_SZDDQUANTUM: [u8; 8] = [0x53, 0x5A, 0x44, 0x44, 0x88, 0xF0, 0x27, 0x33];
const HEADER_RSVKDATA: [u8; 8] = [0x52, 0x53, 0x56, 0x4B, 0x44, 0x41, 0x54, 0x41];
const HEADER_ACE: [u8; 7] = [0x2A, 0x2A, 0x41, 0x43, 0x45, 0x2A, 0x2A];
const HEADER_KWAJ: [u8; 4] = [0x4B, 0x57, 0x41, 0x4A];
const HEADER_SZDD9X: [u8; 4] = [0x53, 0x5A, 0x44, 0x44];
const HEADER_ISZ: [u8; 4] = [0x49, 0x73, 0x5A, 0x21];
const HEADER_DRACO: [u8; 5] = [0x44, 0x52, 0x41, 0x43, 0x4F];
const HEADER_SLOB: [u8; 8] = [0x21, 0x2D, 0x31, 0x53, 0x4C, 0x4F, 0x42, 0x1F];
const HEADER_DCMPA30: [u8; 8] = [0x44, 0x43, 0x4D, 0x01, 0x50, 0x41, 0x33, 0x30];
const HEADER_PA30: [u8; 4] = [0x50, 0x41, 0x33, 0x30];
const HEADER_LZFSE: [u8; 4] = [0x62, 0x76, 0x78, 0x32];
const HEADER_ZLIB_NO_COMPRESSION_WITHOUT_PRESET: [u8; 2] = [0x78, 0x01];
const HEADER_ZLIB_BEST_SPEED_WITHOUT_PRESET: [u8; 2] = [0x78, 0x5E];
const HEADER_ZLIB_DEFAULT_COMPRESSION_WITHOUT_PRESET: [u8; 2] = [0x78, 0x9C];
const HEADER_ZLIB_BEST_COMPRESSION_WITHOUT_PRESET: [u8; 2] = [0x78, 0xDA];
const HEADER_ZLIB_NO_COMPRESSION_WITH_PRESET: [u8; 2] = [0x78, 0x20];
const HEADER_ZLIB_BEST_SPEED_WITH_PRESET: [u8; 2] = [0x78, 0x7D];
const HEADER_ZLIB_DEFAULT_COMPRESSION_WITH_PRESET: [u8; 2] = [0x78, 0xBB];
const HEADER_ZLIB_BEST_COMPRESSION_WITH_PRESET: [u8; 2] = [0x78, 0xF9];

#[derive(PartialEq, Eq, Debug)]
enum CompressionSignature {
    Gzip,
    Xz,
    Zstd,
    Tar,
    Pkzip,
    TarzLZW,
    TarzLZH,
    SevenZ,
    Lzh0,
    Lzh5,
    Bzip2,
    Rnc1,
    Rnc2,
    Lzip,
    Rar1,
    Rar5,
    SzddQuantum,
    Rsvkdata,
    Ace,
    Kwaj,
    Szdd9x,
    Isz,
    Draco,
    Slob,
    DCMPa30,
    Pa30,
    Lzfse,
    Zlib(ZlibCompression),
    NonArchived,
}

#[derive(PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
enum ZlibCompression {
    NoCompressionWithoutPreset,
    BestSpeedWithoutPreset,
    DefaultCompressionWithoutPreset,
    BestCompressionWithoutPreset,
    NoCompressionWithPreset,
    BestSpeedWithPreset,
    DefaultCompressionWithPreset,
    BestCompressionWithPreset,
}

impl std::fmt::Display for CompressionSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let printable = match self {
            CompressionSignature::Gzip => "Gzip",
            CompressionSignature::Xz => "xz",
            CompressionSignature::Zstd => "zstd",
            CompressionSignature::Tar => "tar",
            CompressionSignature::Pkzip => "zip",
            CompressionSignature::TarzLZW => "tar(LZW)",
            CompressionSignature::TarzLZH => "tar(LZH)",
            CompressionSignature::SevenZ => "7z",
            CompressionSignature::Lzh0 => "lzh method 0",
            CompressionSignature::Lzh5 => "lzh method 5",
            CompressionSignature::Bzip2 => "Bzip2",
            CompressionSignature::Rnc1 => "rnc ver.1",
            CompressionSignature::Rnc2 => "rnc ver.2",
            CompressionSignature::Lzip => "lzip",
            CompressionSignature::Rar1 => "rar v1.50",
            CompressionSignature::Rar5 => "rar v5.00",
            CompressionSignature::SzddQuantum => "Quantum",
            CompressionSignature::Rsvkdata => "QuickZip rs",
            CompressionSignature::Ace => "ACE",
            CompressionSignature::Kwaj => "KWAJ",
            CompressionSignature::Szdd9x => "SZDD",
            CompressionSignature::Isz => "ISO",
            CompressionSignature::Draco => "Google Draco",
            CompressionSignature::Slob => "Slob",
            CompressionSignature::DCMPa30 => "Binary Delta",
            CompressionSignature::Pa30 => "Binary Delta",
            CompressionSignature::Lzfse => "LZFSE",
            CompressionSignature::Zlib(_) => "zlib",
            CompressionSignature::NonArchived => "Non archived",
        };
        write!(f, "{}", printable)
    }
}

fn inspect_compression(p: &Path) -> Result<CompressionSignature, FxError> {
    let mut file = std::fs::File::open(p)?;
    let len = file.metadata().unwrap().len();
    let buffer = if len < 265 {
        let mut v = vec![];
        file.read_to_end(&mut v)?;
        v
    } else {
        let mut buffer = [0; 265];
        file.read_exact(&mut buffer)?;
        buffer.to_vec()
    };

    let sign = if buffer[..2] == HEADER_GZIP {
        CompressionSignature::Gzip
    } else if buffer[..6] == HEADER_XZ {
        CompressionSignature::Xz
    } else if buffer[..4] == HEADER_ZSTD {
        CompressionSignature::Zstd
    } else if buffer[..4] == HEADER_PKZIP {
        CompressionSignature::Pkzip
    } else if buffer[..2] == HEADER_TARZ_LZW {
        CompressionSignature::TarzLZW
    } else if buffer[..2] == HEADER_TARZ_LZH {
        CompressionSignature::TarzLZH
    } else if buffer[..6] == HEADER_SEVENZ {
        CompressionSignature::SevenZ
    } else if buffer[..6] == HEADER_LZH0 {
        CompressionSignature::Lzh0
    } else if buffer[..6] == HEADER_LZH5 {
        CompressionSignature::Lzh5
    } else if buffer[..3] == HEADER_BZ2 {
        CompressionSignature::Bzip2
    } else if buffer[..4] == HEADER_RNC1 {
        CompressionSignature::Rnc1
    } else if buffer[..4] == HEADER_RNC2 {
        CompressionSignature::Rnc2
    } else if buffer[..7] == HEADER_RAR1 {
        CompressionSignature::Rar1
    } else if buffer[..4] == HEADER_LZIP {
        CompressionSignature::Lzip
    } else if buffer[..8] == HEADER_RAR5 {
        CompressionSignature::Rar5
    } else if buffer[..8] == HEADER_SZDDQUANTUM {
        CompressionSignature::SzddQuantum
    } else if buffer[..8] == HEADER_RSVKDATA {
        CompressionSignature::Rsvkdata
    } else if buffer[..7] == HEADER_ACE {
        CompressionSignature::Ace
    } else if buffer[..4] == HEADER_KWAJ {
        CompressionSignature::Kwaj
    } else if buffer[..4] == HEADER_SZDD9X {
        CompressionSignature::Szdd9x
    } else if buffer[..4] == HEADER_ISZ {
        CompressionSignature::Isz
    } else if buffer[..5] == HEADER_DRACO {
        CompressionSignature::Draco
    } else if buffer[..8] == HEADER_SLOB {
        CompressionSignature::Slob
    } else if buffer[..8] == HEADER_DCMPA30 {
        CompressionSignature::DCMPa30
    } else if buffer[..4] == HEADER_PA30 {
        CompressionSignature::Pa30
    } else if buffer[..4] == HEADER_LZFSE {
        CompressionSignature::Lzfse
    } else if buffer[..2] == HEADER_ZLIB_NO_COMPRESSION_WITHOUT_PRESET {
        CompressionSignature::Zlib(ZlibCompression::NoCompressionWithoutPreset)
    } else if buffer[..2] == HEADER_ZLIB_DEFAULT_COMPRESSION_WITHOUT_PRESET {
        CompressionSignature::Zlib(ZlibCompression::DefaultCompressionWithoutPreset)
    } else if buffer[..2] == HEADER_ZLIB_BEST_SPEED_WITHOUT_PRESET {
        CompressionSignature::Zlib(ZlibCompression::BestSpeedWithoutPreset)
    } else if buffer[..2] == HEADER_ZLIB_BEST_COMPRESSION_WITHOUT_PRESET {
        CompressionSignature::Zlib(ZlibCompression::BestCompressionWithoutPreset)
    } else if buffer[..2] == HEADER_ZLIB_NO_COMPRESSION_WITH_PRESET {
        CompressionSignature::Zlib(ZlibCompression::NoCompressionWithPreset)
    } else if buffer[..2] == HEADER_ZLIB_DEFAULT_COMPRESSION_WITH_PRESET {
        CompressionSignature::Zlib(ZlibCompression::DefaultCompressionWithPreset)
    } else if buffer[..2] == HEADER_ZLIB_BEST_SPEED_WITH_PRESET {
        CompressionSignature::Zlib(ZlibCompression::BestSpeedWithPreset)
    } else if buffer[..2] == HEADER_ZLIB_BEST_COMPRESSION_WITH_PRESET {
        CompressionSignature::Zlib(ZlibCompression::BestCompressionWithPreset)
    } else if is_tar(&buffer) {
        CompressionSignature::Tar
    } else {
        CompressionSignature::NonArchived
    };
    Ok(sign)
}

fn is_tar(b: &[u8]) -> bool {
    b.len() >= 265 && (b[257..265] == HEADER_TAR1 || b[257..265] == HEADER_TAR2)
}

pub fn unpack(p: &Path, dest: &Path) -> Result<(), FxError> {
    let sign = inspect_compression(p)?;
    match sign {
        CompressionSignature::Gzip => {
            let file = std::fs::File::open(p)?;
            let file = std::io::BufReader::new(file);
            let mut decoder = flate2::bufread::GzDecoder::new(file);
            let mut output = std::fs::File::create(dest)?;
            std::io::copy(&mut decoder, &mut output)?;
            let input = std::fs::read(dest)?;
            if is_tar(&input) {
                let mut archive = tar::Archive::new(input.as_slice());
                std::fs::remove_file(dest)?;
                archive.unpack(dest)?;
            }
        }
        CompressionSignature::Xz => {
            let file = std::fs::File::open(p)?;
            let mut file = std::io::BufReader::new(file);
            let mut decoder: Vec<u8> = Vec::new();
            lzma_rs::xz_decompress(&mut file, &mut decoder).unwrap();
            if is_tar(&decoder) {
                let mut archive = tar::Archive::new(decoder.as_slice());
                archive.unpack(dest)?;
            } else {
                std::fs::write(dest, decoder)?;
            }
        }
        CompressionSignature::Zstd => {
            let file = std::fs::File::open(p)?;
            let file = std::io::BufReader::new(file);
            let decoder = zstd::stream::decode_all(file).unwrap();
            if is_tar(&decoder) {
                let mut archive = tar::Archive::new(decoder.as_slice());
                archive.unpack(dest)?;
            } else {
                std::fs::write(dest, decoder)?;
            }
        }
        CompressionSignature::Tar => {
            let file = std::fs::read(p)?;
            let mut archive = tar::Archive::new(file.as_slice());
            archive.unpack(dest)?;
        }
        CompressionSignature::Pkzip => {
            let file = std::fs::File::open(p)?;
            let mut archive = zip::ZipArchive::new(file)?;
            archive.extract(dest)?;
        }
        CompressionSignature::NonArchived => {
            return Err(FxError::Unpack("Seems not an archive file.".to_owned()))
        }
        _ => {
            return Err(FxError::Unpack(format!(
                "Cannot unpack this type: {}",
                sign
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Supported:
    /// gz(Gzip),
    /// tar.gz,
    /// xz(lzma),
    /// tar.xz,
    /// zst(Zstandard),
    /// tar.zst,
    /// tar,
    /// zip file format and formats based on it(zip, docx, ...)
    #[test]
    fn test_inspect_signature_tar_gz() {
        let p = PathBuf::from("testfiles/archives/archive.tar.gz");
        assert_eq!(CompressionSignature::Gzip, inspect_compression(&p).unwrap());
        let dest = PathBuf::from("testfiles/archives/gz1");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_dir());
        std::fs::remove_dir_all("testfiles/archives/gz1").unwrap();
    }

    #[test]
    fn test_inspect_signature_gz() {
        let p = PathBuf::from("testfiles/archives/archive.txt.gz");
        assert_eq!(CompressionSignature::Gzip, inspect_compression(&p).unwrap());
        let dest = PathBuf::from("testfiles/archives/gz.txt");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_file());
        assert_eq!(std::fs::read_to_string(dest).unwrap(), "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n".to_string());
        std::fs::remove_file("testfiles/archives/gz.txt").unwrap();
    }

    #[test]
    fn test_inspect_signature_tar_xz() {
        let p = PathBuf::from("testfiles/archives/archive.tar.xz");
        assert_eq!(CompressionSignature::Xz, inspect_compression(&p).unwrap());
        let dest = PathBuf::from("testfiles/archives/xz");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_dir());
        std::fs::remove_dir_all("testfiles/archives/xz").unwrap();
    }

    #[test]
    fn test_inspect_signature_xz() {
        let p = PathBuf::from("testfiles/archives/archive.txt.xz");
        assert_eq!(CompressionSignature::Xz, inspect_compression(&p).unwrap());
        let dest = PathBuf::from("testfiles/archives/xz.txt");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_file());
        assert_eq!(std::fs::read_to_string(dest).unwrap(), "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n".to_string());
        std::fs::remove_file("testfiles/archives/xz.txt").unwrap();
    }

    #[test]
    fn test_inspect_signature_tar_zst() {
        let p = PathBuf::from("testfiles/archives/archive.tar.zst");
        assert_eq!(CompressionSignature::Zstd, inspect_compression(&p).unwrap());
        let dest = PathBuf::from("testfiles/archives/zst");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_dir());
        std::fs::remove_dir_all("testfiles/archives/zst").unwrap();
    }

    #[test]
    fn test_inspect_signature_zst() {
        let p = PathBuf::from("testfiles/archives/archive.txt.zst");
        assert_eq!(CompressionSignature::Zstd, inspect_compression(&p).unwrap());
        let dest = PathBuf::from("testfiles/archives/zst_no_tar");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_file());
        assert_eq!(std::fs::read_to_string(dest).unwrap(), "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n".to_string());
        std::fs::remove_file("testfiles/archives/zst_no_tar").unwrap();
    }

    #[test]
    fn test_inspect_signature_tar() {
        let p = PathBuf::from("testfiles/archives/archive.tar");
        assert_eq!(CompressionSignature::Tar, inspect_compression(&p).unwrap());
        let dest = PathBuf::from("testfiles/archives/tar");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_dir());
        std::fs::remove_dir_all("testfiles/archives/tar").unwrap();
    }

    #[test]
    fn test_inspect_signature_bzip2() {
        let p = PathBuf::from("testfiles/archives/archive_bzip2.zip");
        assert_eq!(
            CompressionSignature::Pkzip,
            inspect_compression(&p).unwrap()
        );
        let dest = PathBuf::from("testfiles/archives/bzip2");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_dir());
        std::fs::remove_dir_all("testfiles/archives/bzip2").unwrap();
    }

    #[test]
    fn test_inspect_signature_store() {
        let p = PathBuf::from("testfiles/archives/archive_store.zip");
        assert_eq!(
            CompressionSignature::Pkzip,
            inspect_compression(&p).unwrap()
        );
        let dest = PathBuf::from("testfiles/archives/store");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_dir());
        std::fs::remove_dir_all("testfiles/archives/store").unwrap();
    }

    #[test]
    fn test_inspect_signature_deflate() {
        let p = PathBuf::from("testfiles/archives/archive_deflate.zip");
        assert_eq!(
            CompressionSignature::Pkzip,
            inspect_compression(&p).unwrap()
        );
        let dest = PathBuf::from("testfiles/archives/deflate");
        assert!(unpack(&p, &dest).is_ok());
        assert!(dest.is_dir());
        std::fs::remove_dir_all("testfiles/archives/deflate").unwrap();
    }

    #[test]
    fn test_inspect_signature_bz2() {
        //bz2 not available now
        let p = PathBuf::from("testfiles/archives/archive.tar.bz2");
        assert_eq!(
            CompressionSignature::Bzip2,
            inspect_compression(&p).unwrap()
        );
        let dest = PathBuf::from("testfiles/archives/bz2");
        assert!(unpack(&p, &dest).is_err());
    }
}
