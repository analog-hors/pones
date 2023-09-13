use std::io::prelude::*;

use thiserror::Error;

use super::INesCart;
use super::mapper::INesMapper;

#[derive(Debug, Error)]
pub enum INesParseError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("invalid magic value")]
    InvalidMagic,
    #[error("unsupported mapper id {0}")]
    UnsupportedMapper(u8),
}

impl INesCart {
    pub fn parse(read: &mut impl Read) -> Result<Self, INesParseError> {
        use INesParseError::*;
        
        let mut header = [0; 16];
        read.read_exact(&mut header)?;
        if !header.starts_with(b"NES\x1A") {
            return Err(InvalidMagic);
        }

        let prg_rom_size = header[4] as usize * 16384;
        let chr_rom_size = header[5] as usize * 8192;
        let mapper_id = (header[7] << 4) | (header[6] & 0xF);
        
        let mut prg_rom = vec![0; prg_rom_size].into_boxed_slice();
        let mut chr_rom = vec![0; chr_rom_size].into_boxed_slice();
        read.read_exact(&mut prg_rom)?;
        read.read_exact(&mut chr_rom)?;
        let mapper = INesMapper::from_id(mapper_id)?;

        Ok(Self {
            prg_rom,
            chr_rom,
            mapper,
        })
    }
}
