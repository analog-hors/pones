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

fn load_mem(name: &str, start_addr: u16) -> Memory {
    let path = format!("tests/{}", name);
    let bin = std::fs::read(path).expect("failed to read test binary");
    let mut mem = [0; 65536];
    mem[start_addr as usize..start_addr as usize + bin.len()].copy_from_slice(&bin);
    Memory(mem)
}

#[test]
fn functional_test() {
    const BIN_START_ADDR: u16 = 0x000A;
    const PROGRAM_START: u16 = 0x0400;
    const SUCCESS_TRAP: u16 = 0x3469;

    let mem = load_mem("klaus/bin/6502_functional_test.bin", BIN_START_ADDR);
    let mut cpu = Cpu6502::new(mem);
    cpu.pc = PROGRAM_START;
    loop {
        let prev_pc = cpu.pc;
        cpu.step();
        if cpu.pc == prev_pc {
            break;
        }
    }
    assert!(cpu.pc == SUCCESS_TRAP, "trapped at {:#06X}", cpu.pc);
}

#[test]
fn interrupt_test() {
    const BIN_START_ADDR: u16 = 0x000A;
    const PROGRAM_START: u16 = 0x0400;
    const SUCCESS_TRAP: u16 = 0x06F5;
    const FEEDBACK_ADDR: u16 = 0xBFFC;
    const IRQ_BIT: u8 = 1 << 0;
    const NMI_BIT: u8 = 1 << 1;

    let mem = load_mem("klaus/bin/6502_interrupt_test.bin", BIN_START_ADDR);
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
    assert!(cpu.pc == SUCCESS_TRAP, "trapped at {:#06X}", cpu.pc);
}

#[test]
fn decimal_test() {
    const BIN_START_ADDR: u16 = 0x0200;
    const PROGRAM_START: u16 = 0x0200;
    const ERROR_ADDR: u16 = 0x000B;
    const DONE_ADDR: u16 = 0x024B;
    const N1_ADDR: u16 = 0x0000;
    const N2_ADDR: u16 = 0x0001;
    const DA_ADDR: u16 = 0x0004;
    const AR_ADDR: u16 = 0x0006;

    let mem = load_mem("decimal/bin/6502_decimal_test.bin", BIN_START_ADDR);
    let mut cpu = Cpu6502::new(mem);
    cpu.pc = PROGRAM_START;
    while cpu.pc != DONE_ADDR {
        cpu.step();
    }
    if cpu.bus.read(ERROR_ADDR) != 0 {
        eprintln!("N1 = {}", cpu.bus.read(N1_ADDR));
        eprintln!("N2 = {}", cpu.bus.read(N2_ADDR));
        eprintln!("DA = {}", cpu.bus.read(DA_ADDR));
        eprintln!("AR = {}", cpu.bus.read(AR_ADDR));
        panic!("decimal mode test failed");
    }
}
