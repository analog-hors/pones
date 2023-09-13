use pones_6502::Cpu6502;

pub mod mem;
pub mod ppu;
pub mod cart;

use mem::CpuMemMap;
use cart::NesCart;
use ppu::NesPpu;

pub struct NesEmulator {
    pub cpu_mem: [u8; 2048],
    pub ppu_mem: [u8; 2048],
    pub cpu: Cpu6502,
    pub ppu: NesPpu,
}

impl NesEmulator {
    pub fn new() -> Self {
        Self {
            cpu_mem: [0; 2048],
            ppu_mem: [0; 2048],
            cpu: Cpu6502::with_no_decimal(),
            ppu: NesPpu::new(),
        }
    }

    pub fn step(&mut self, cart: &mut impl NesCart) {
        self.cpu.step(&mut CpuMemMap {
            cpu_mem: &mut self.cpu_mem,
            ppu_reg: &mut self.ppu.reg,
            cart,
        });
    }

    pub fn cpu_mem_map<'m, C: NesCart>(&'m mut self, cart: &'m mut C) -> CpuMemMap<'m, C> {
        CpuMemMap {
            cpu_mem: &mut self.cpu_mem,
            ppu_reg: &mut self.ppu.reg,
            cart,
        }
    }
}
