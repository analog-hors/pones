use super::NesCart;

mod mapper;
mod parse;

use mapper::INesMapper;

pub struct INesCart {
    prg_rom: Box<[u8]>,
    chr_rom: Box<[u8]>,
    mapper: INesMapper,
}

impl NesCart for INesCart {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        use INesMapper::*;
        
        match self.mapper {
            NRom => match addr {
                //TODO consider PRG RAM
                0x8000..=0xFFFF => self.prg_rom[(addr - 0x8000) as usize % self.prg_rom.len()],
                _ => 0
            }
        }
    }

    fn cpu_write(&mut self, addr: u16, value: u8) {
        use INesMapper::*;
        
        match self.mapper {
            NRom => match addr {
                //TODO consider PRG RAM
                0x8000..=0xFFFF => self.prg_rom[(addr - 0x8000) as usize % self.prg_rom.len()] = value,
                _ => {}
            }
        }
    }
}
