use std::{io::Read, path::PathBuf};

use crate::errors::FxError;

#[derive(PartialEq, Debug)]
enum Signatures {
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

fn inspect_signatures(p: PathBuf) -> Result<Signatures, FxError> {
    let mut file = std::fs::File::open(p)?;
    let mut buffer = [0; 8];
    file.read_exact(&mut buffer)?;
    for byte in buffer {
        print!("{:0x} ", byte);
    }

    let sign = match buffer[0] {
        0x1f => match buffer[1] {
            0x9d => Signatures::TarzLZW,
            0xA0 => Signatures::TarzLZH,
            0x8b => Signatures::Gzip,
            _ => Signatures::Others,
        },
        0x2d => match buffer[1..5] {
            [0x68, 0x6c, 0x30, 0x2d] => Signatures::Lzh0,
            [0x68, 0x6c, 0x35, 0x2d] => Signatures::Lzh5,
            _ => Signatures::Others,
        },
        0x42 => match buffer[1..3] {
            [0x5a, 0x68] => Signatures::Bz2,
            _ => Signatures::Others,
        },
        // 0x52 => todo!("rnc, rar, rsvkdata"),
        0x52 => match buffer[1] {
            0x4e => match buffer[2..4] {
                [0x43, 0x01] => Signatures::Rnc1,
                [0x43, 0x02] => Signatures::Rnc2,
                _ => Signatures::Others,
            },
            0x61 => match buffer[2..] {
                [0x72, 0x21, 0x1a, 0x07, 0x00, _] => Signatures::Rar1,
                [0x72, 0x21, 0x1a, 0x07, 0x01, 0x00] => Signatures::Rar5,
                _ => Signatures::Others,
            },
            0x53 => match buffer[2..] {
                [0x56, 0x4b, 0x44, 0x41, 0x54, 0x41] => Signatures::Rsvkdata,
                _ => Signatures::Others,
            },
            _ => Signatures::Others,
        },
        0x4c => match buffer[2..5] {
            [0x5a, 0x49, 0x50] => Signatures::Lzip,
            _ => Signatures::Others,
        },
        // 0x53 => todo!("szddquantum, szdd9x"),
        0x53 => match buffer[1..] {
            [0x5a, 0x44, 0x44, 0x88, 0xf0, 0x27, 0x33] => Signatures::SzddQuantum,
            [0x5a, 0x44, 0x44, _, _, _, _] => Signatures::Szdd9x,
            _ => Signatures::Others,
        },
        // 0x2a => todo!("ace"),
        // 0x4b => todo!("kwaj"),
        // 0x49 => todo!("ISO"),
        // 0x44 => todo!("draco, dcm"),
        // 0x21 => todo!("slob"),
        0xfd => match buffer[1..6] {
            [0x37, 0x7a, 0x58, 0x5a, 0x00] => Signatures::Xz,
            _ => Signatures::Others,
        },
        // 0x78 => todo!("zlibs"),
        // 0x62 => todo!("lzfse"),
        _ => Signatures::Others,
    };

    Ok(sign)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_signatures() {
        let p = PathBuf::from("/home/kyohei/test/archive.tar.gz");
        assert_eq!(Signatures::Gzip, inspect_signatures(p).unwrap());

        let p = PathBuf::from("/home/kyohei/test/archive.tar.xz");
        assert_eq!(Signatures::Xz, inspect_signatures(p).unwrap());
    }
}
