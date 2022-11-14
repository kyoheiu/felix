use std::{io::Read, path::PathBuf};

const HEADER_TARZ_LZW: [u8; 2] = [0x1F, 0x9D];
const HEADER_TARZ_LZH: [u8; 2] = [0x1F, 0xA0];
const HEADER_GZIP: [u8; 2] = [0x1F, 0x8B];
const HEADER_LZH0: [u8; 5] = [0x2D, 0x68, 0x6C, 0x30, 0x2D];
const HEADER_LZH5: [u8; 5] = [0x2D, 0x68, 0x6C, 0x35, 0x2D];
const HEADER_BZ2: [u8; 3] = [0x42, 0x5A, 0x68];
const HEADER_RNC1: [u8; 4] = [0x52, 0x4E, 0x43, 0x01];
const HEADER_RNC2: [u8; 4] = [0x52, 0x4E, 0x43, 0x02];
const HEADER_RAR1: [u8; 7] = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00];
const HEADER_RAR5: [u8; 8] = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];
const HEADER_RSVKDATA: [u8; 8] = [0x52, 0x53, 0x56, 0x4B, 0x44, 0x41, 0x54, 0x41];
const HEADER_LZIP: [u8; 4] = [0x4C, 0x5A, 0x49, 0x50];
const HEADER_SZDD9X: [u8; 4] = [0x53, 0x5A, 0x44, 0x44];
const HEADER_SZDDQUANTUM: [u8; 8] = [0x53, 0x5A, 0x44, 0x44, 0x88, 0xF0, 0x27, 0x33];
const HEADER_ACE: [u8; 7] = [0x2A, 0x2A, 0x41, 0x43, 0x45, 0x2A, 0x2A];
const HEADER_KWAJ: [u8; 4] = [0x4B, 0x57, 0x41, 0x4A];
const HEADER_ISZ: [u8; 4] = [0x49, 0x73, 0x5A, 0x21];
const HEADER_DRACO: [u8; 5] = [0x44, 0x52, 0x41, 0x43, 0x4F];
const HEADER_DCMPA30: [u8; 8] = [0x44, 0x43, 0x4D, 0x01, 0x50, 0x41, 0x33, 0x30];
const HEADER_SLOB: [u8; 8] = [0x21, 0x2D, 0x31, 0x53, 0x4C, 0x4F, 0x42, 0x1F];
const HEADER_XZ: [u8; 6] = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00];
const HEADER_PA30: [u8; 4] = [0x50, 0x41, 0x33, 0x30];
const HEADER_ZLIB_NO_COMPRESSION_WITHOUT_PRESET: [u8; 2] = [0x78, 0x01];
const HEADER_ZLIB_BEST_SPEED_WITHOUT_PRESET: [u8; 2] = [0x78, 0x5E];
const HEADER_ZLIB_DEFAULT_COMPRESSION_WITHOUT_PRESET: [u8; 2] = [0x78, 0x9C];
const HEADER_ZLIB_BEST_COMPRESSION_WITHOUT_PRESET: [u8; 2] = [0x78, 0xDA];
const HEADER_ZLIB_NO_COMPRESSION_WITH_PRESET: [u8; 2] = [0x78, 0x20];
const HEADER_ZLIB_BEST_SPEED_WITH_PRESET: [u8; 2] = [0x78, 0x7D];
const HEADER_ZLIB_DEFAULT_COMPRESSION_WITH_PRESET: [u8; 2] = [0x78, 0xBB];
const HEADER_ZLIB_BEST_COMPRESSION_WITH_PRESET: [u8; 2] = [0x78, 0xF9];
const HEADER_LZFSE: [u8; 4] = [0x62, 0x76, 0x78, 0x32];
const HEADER_ZSTD: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

use crate::errors::FxError;

#[derive(PartialEq, Debug)]
enum Signature {
    TarzLZW,
    TarzLZH,
    Lzh0,
    Lzh5,
    Bz2,
    Rnc1,
    Rnc2,
    Lzip,
    Rar1,
    Rar5,
    Gzip,
    SzddQuantum,
    Rsvkdata,
    Ace,
    Kwaj,
    Szdd9x,
    Isz,
    Draco,
    Slob,
    Xz,
    DCMPa30,
    Pa30,
    Zlib(ZlibCompression),
    Lzfse,
    Zstd,
    Others,
}

#[derive(PartialEq, Debug)]
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

fn inspect_signatures(p: PathBuf) -> Result<Signature, FxError> {
    let mut file = std::fs::File::open(p)?;
    let mut buffer = [0; 8];
    file.read_exact(&mut buffer)?;

    let sign = match buffer[0] {
        0x1f => {
            if buffer[..2] == HEADER_TARZ_LZW {
                Signature::TarzLZW
            } else if buffer[..2] == HEADER_TARZ_LZH {
                Signature::TarzLZH
            } else if buffer[..2] == HEADER_GZIP {
                Signature::Gzip
            } else {
                Signature::Others
            }
        }
        0x2d => {
            if buffer[..6] == HEADER_LZH0 {
                Signature::Lzh0
            } else if buffer[..6] == HEADER_LZH5 {
                Signature::Lzh5
            } else {
                Signature::Others
            }
        }
        0x42 => {
            if buffer[..2] == HEADER_BZ2 {
                Signature::Bz2
            } else {
                Signature::Others
            }
        }
        // 0x52 => todo!("rnc, rar, rsvkdata"),
        0x52 => {
            if buffer[..4] == HEADER_RNC1 {
                Signature::Rnc1
            } else if buffer[..4] == HEADER_RNC2 {
                Signature::Rnc2
            } else if buffer[..7] == HEADER_RAR1 {
                Signature::Rar1
            } else if buffer == HEADER_RAR5 {
                Signature::Rar5
            } else if buffer == HEADER_RSVKDATA {
                Signature::Rsvkdata
            } else {
                Signature::Others
            }
        }
        0x4c => {
            if buffer[..4] == HEADER_LZIP {
                Signature::Lzip
            } else {
                Signature::Others
            }
        }
        // 0x53 => todo!("szddquantum, szdd9x"),
        0x53 => {
            if buffer[..4] == HEADER_SZDD9X {
                Signature::Szdd9x
            } else if buffer == HEADER_SZDDQUANTUM {
                Signature::SzddQuantum
            } else {
                Signature::Others
            }
        }
        0x2a => {
            if buffer[..7] == HEADER_ACE {
                Signature::Ace
            } else {
                Signature::Others
            }
        }
        0x4b => {
            if buffer[..4] == HEADER_KWAJ {
                Signature::Kwaj
            } else {
                Signature::Others
            }
        }
        0x49 => {
            if buffer[..4] == HEADER_ISZ {
                Signature::Isz
            } else {
                Signature::Others
            }
        }
        0x44 => {
            if buffer[..5] == HEADER_DRACO {
                Signature::Draco
            } else if buffer == HEADER_DCMPA30 {
                Signature::DCMPa30
            } else {
                Signature::Others
            }
        }
        0x21 => {
            if buffer == HEADER_SLOB {
                Signature::Slob
            } else {
                Signature::Others
            }
        }
        0xfd => {
            if buffer[..6] == HEADER_XZ {
                Signature::Xz
            } else {
                Signature::Others
            }
        }
        0x78 => {
            if buffer[..2] == HEADER_ZLIB_NO_COMPRESSION_WITHOUT_PRESET {
                Signature::Zlib(ZlibCompression::NoCompressionWithoutPreset)
            } else if buffer[..2] == HEADER_ZLIB_DEFAULT_COMPRESSION_WITHOUT_PRESET {
                Signature::Zlib(ZlibCompression::DefaultCompressionWithoutPreset)
            } else if buffer[..2] == HEADER_ZLIB_BEST_SPEED_WITHOUT_PRESET {
                Signature::Zlib(ZlibCompression::BestSpeedWithoutPreset)
            } else if buffer[..2] == HEADER_ZLIB_BEST_COMPRESSION_WITHOUT_PRESET {
                Signature::Zlib(ZlibCompression::BestCompressionWithoutPreset)
            } else if buffer[..2] == HEADER_ZLIB_NO_COMPRESSION_WITH_PRESET {
                Signature::Zlib(ZlibCompression::NoCompressionWithPreset)
            } else if buffer[..2] == HEADER_ZLIB_DEFAULT_COMPRESSION_WITH_PRESET {
                Signature::Zlib(ZlibCompression::DefaultCompressionWithPreset)
            } else if buffer[..2] == HEADER_ZLIB_BEST_SPEED_WITH_PRESET {
                Signature::Zlib(ZlibCompression::BestSpeedWithPreset)
            } else if buffer[..2] == HEADER_ZLIB_BEST_COMPRESSION_WITH_PRESET {
                Signature::Zlib(ZlibCompression::BestCompressionWithPreset)
            } else {
                Signature::Others
            }
        }
        0x62 => {
            if buffer[..4] == HEADER_LZFSE {
                Signature::Lzfse
            } else {
                Signature::Others
            }
        }
        0x28 => {
            if buffer[..4] == HEADER_ZSTD {
                Signature::Zstd
            } else {
                Signature::Others
            }
        }
        _ => Signature::Others,
    };

    Ok(sign)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_signatures() {
        let p = PathBuf::from("/home/kyohei/test/archive.tar.gz");
        assert_eq!(Signature::Gzip, inspect_signatures(p).unwrap());

        let p = PathBuf::from("/home/kyohei/test/archive.tar.xz");
        assert_eq!(Signature::Xz, inspect_signatures(p).unwrap());
    }
}
