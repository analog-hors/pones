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

    let mut mem = load_mem("klaus/bin/6502_functional_test.bin", BIN_START_ADDR);
    let mut cpu = Cpu6502::new();
    cpu.pc = PROGRAM_START;
    loop {
        let prev_pc = cpu.pc;
        cpu.step(&mut mem);
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

    let mut mem = load_mem("klaus/bin/6502_interrupt_test.bin", BIN_START_ADDR);
    let mut cpu = Cpu6502::new();
    cpu.pc = PROGRAM_START;
    mem.write(FEEDBACK_ADDR, 0);
    loop {
        let prev_feedback = mem.read(FEEDBACK_ADDR);
        let prev_pc = cpu.pc;
        cpu.step(&mut mem);
        let feedback = mem.read(FEEDBACK_ADDR);
        if (feedback & !prev_feedback) & NMI_BIT != 0 {
            cpu.nmi(&mut mem);
        } else if feedback & IRQ_BIT != 0 {
            cpu.irq(&mut mem);
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
    const DNVZC_ADDR: u16 = 0x0005;
    const NF_ADDR: u16 = 0x0007;
    const VF_ADDR: u16 = 0x0008;
    const ZF_ADDR: u16 = 0x0009;
    const CF_ADDR: u16 = 0x000A;

    let mut mem = load_mem("decimal/bin/6502_decimal_test.bin", BIN_START_ADDR);
    let mut cpu = Cpu6502::new();
    cpu.pc = PROGRAM_START;
    while cpu.pc != DONE_ADDR {
        cpu.step(&mut mem);
    }
    if mem.read(ERROR_ADDR) != 0 {
        eprintln!("CB = {}", cpu.reg.y);
        eprintln!("N1 = {}", mem.read(N1_ADDR));
        eprintln!("N2 = {}", mem.read(N2_ADDR));
        eprintln!("DA = {}", mem.read(DA_ADDR));
        eprintln!("AR = {}", mem.read(AR_ADDR));
        eprintln!("ND = {}", mem.read(DNVZC_ADDR) & (1 << 7) != 0);
        eprintln!("VD = {}", mem.read(DNVZC_ADDR) & (1 << 6) != 0);
        eprintln!("ZD = {}", mem.read(DNVZC_ADDR) & (1 << 1) != 0);
        eprintln!("CD = {}", mem.read(DNVZC_ADDR) & (1 << 0) != 0);
        eprintln!("NF = {}", mem.read(NF_ADDR) & (1 << 7) != 0);
        eprintln!("VF = {}", mem.read(VF_ADDR) & (1 << 6) != 0);
        eprintln!("ZF = {}", mem.read(ZF_ADDR) & (1 << 1) != 0);
        eprintln!("CF = {}", mem.read(CF_ADDR) & (1 << 0) != 0);
        panic!("decimal mode test failed");
    }
}
