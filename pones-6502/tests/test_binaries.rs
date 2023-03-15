use pones_6502::{Cpu6502, Bus};

struct Memory([u8; 65536]);

impl Bus for Memory {
    fn read(&mut self, addr: u16) -> u8 {
        self.0[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.0[addr as usize] = value;
    }
}

#[test]
fn functional_test() {
    let mem = Memory(*include_bytes!("test_data/6502_functional_test.bin"));
    let mut cpu = Cpu6502::new(mem);
    cpu.pc = 0x0400;
    loop {
        let prev = cpu.pc;
        cpu.step();
        if cpu.pc == prev {
            break;
        }
    }
    if cpu.pc != 0x3469 {
        panic!("trapped at {:#06X}", cpu.pc);
    }
}
