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

fn load_mem(name: &str) -> Memory {
    let path = format!("tests/test_src/bin/{}", name);
    let mem = std::fs::read(path).expect("failed to read test binary");
    let mem = mem.try_into().expect("invalid test binary");
    Memory(mem)
}

#[test]
fn functional_test() {
    const PROGRAM_START: u16 = 0x0400;
    const SUCCESS_TRAP: u16 = 0x3469;

    let mem = load_mem("6502_functional_test.bin");
    let mut cpu = Cpu6502::new(mem);
    cpu.pc = PROGRAM_START;
    loop {
        let prev_pc = cpu.pc;
        cpu.step();
        if cpu.pc == prev_pc {
            break;
        }
    }
    if cpu.pc != SUCCESS_TRAP {
        panic!("trapped at {:#06X}", cpu.pc);
    }
}

#[test]
fn interrupt_test() {
    const PROGRAM_START: u16 = 0x0400;
    const SUCCESS_TRAP: u16 = 0x06F5;
    const FEEDBACK_ADDR: u16 = 0xBFFC;
    const IRQ_BIT: u8 = 1 << 0;
    const NMI_BIT: u8 = 1 << 1;

    let mem = load_mem("6502_interrupt_test.bin");
    let mut cpu = Cpu6502::new(mem);
    cpu.pc = PROGRAM_START;
    cpu.bus.write(FEEDBACK_ADDR, 0);
    loop {
        let prev_feedback = cpu.bus.read(FEEDBACK_ADDR);
        let prev_pc = cpu.pc;
        cpu.step();
        let feedback = cpu.bus.read(FEEDBACK_ADDR);
        if (feedback & !prev_feedback) & NMI_BIT != 0 {
            cpu.nmi();
        } else if feedback & IRQ_BIT != 0 {
            cpu.irq();
        }
        if cpu.pc == prev_pc {
            break;
        }
    }
    if cpu.pc != SUCCESS_TRAP {
        panic!("trapped at {:#06X}", cpu.pc);
    }
}
