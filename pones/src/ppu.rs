#[derive(Debug, Default)]
pub struct NesPpu {
    pub reg: PpuRegisters,
}

impl NesPpu {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Default)]
pub struct PpuRegisters {
    pub ppu_ctrl: u8,   // [VPHB SINN] NMI enable (V), PPU master/slave (P), sprite height (H), background tile select (B), sprite tile select (S), increment mode (I), nametable select (NN)
    pub ppu_mask: u8,   // [BGRs bMmG] color emphasis (BGR), sprite enable (s), background enable (b), sprite left column enable (M), background left column enable (m), greyscale (G)
    pub ppu_status: u8, // [VSO- ----] vblank (V), sprite 0 hit (S), sprite overflow (O); read resets write pair for $2005/$2006
    pub oam_addr: u8,   // [aaaa aaaa] OAM read/write address
    pub oam_data: u8,   // [dddd dddd] OAM data read/write
    pub ppu_scroll: u8, // [xxxx xxxx] fine scroll position (two writes: X scroll, Y scroll)
    pub ppu_addr: u8,   // [aaaa aaaa] PPU read/write address (two writes: most significant byte, least significant byte)
    pub ppu_data: u8,   // [dddd dddd] PPU data read/write
    // pub oam_dma: u8,    // [aaaa aaaa] OAM DMA high address
}

impl PpuRegisters {
    pub fn get_mut(&mut self, addr: u16) -> &mut u8 {
        match addr % 8 {
            0 => &mut self.ppu_ctrl,
            1 => &mut self.ppu_mask,
            2 => &mut self.ppu_status,
            3 => &mut self.oam_addr,
            4 => &mut self.oam_data,
            5 => &mut self.ppu_scroll,
            6 => &mut self.ppu_addr,
            7 => &mut self.ppu_data,
            8.. => unreachable!(),
        }
    }
}
