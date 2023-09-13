use pones_6502::Bus;

use crate::cart::NesCart;
use crate::ppu::PpuRegisters;

pub struct CpuMemMap<'m, C> {
    pub cpu_mem: &'m mut [u8; 2048],
    pub ppu_reg: &'m mut PpuRegisters,
    pub cart: &'m mut C,
}

impl<C: NesCart> Bus for CpuMemMap<'_, C> {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cpu_mem[addr as usize % self.cpu_mem.len()], // 2 KB internal RAM
            0x2000..=0x3FFF => *self.ppu_reg.get_mut(addr), // NES PPU registers
            0x4000..=0x4017 => 0, // NES APU and I/O registers
            0x4018..=0x401F => 0, // APU and I/O functionality that is normally disabled
            0x4020..=0xFFFF => self.cart.cpu_read(addr), // Cartridge space: PRG ROM, PRG RAM, and mapper registers
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.cpu_mem[addr as usize % self.cpu_mem.len()] = value,
            0x2000..=0x3FFF => *self.ppu_reg.get_mut(addr) = value,
            0x4000..=0x4017 => {},
            0x4018..=0x401F => {},
            0x4020..=0xFFFF => self.cart.cpu_write(addr, value),
        }
    }
}
