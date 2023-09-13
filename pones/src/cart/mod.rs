mod ines;

pub use ines::*;

pub trait NesCart {
    /// A read from the part of the CPU address space mapped to the cartridge (`$4020-$FFFF`).
    fn cpu_read(&mut self, addr: u16) -> u8;

    /// A write to the part of the CPU address space mapped to the cartridge (`$4020-$FFFF`).
    fn cpu_write(&mut self, addr: u16, value: u8);
}
