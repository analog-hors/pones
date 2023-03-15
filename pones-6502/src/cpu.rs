use crate::reg_state::RegisterState;

const STACK_START: u16 = 0x0100;
const IRQ_BRK_VECTOR: u16 = 0xFFFE;
const RESET_VECTOR: u16 = 0xFFFC;
const NMI_VECTOR: u16 = 0xFFFA;

pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CpuFlag {
    Carry,
    Zero,
    InterruptDisable,
    Decimal,
    Break,
    Reserved,
    Overflow,
    Negative
}

pub struct Cpu6502<B> {
    pub bus: B,
    pub reg: RegisterState,
    pub sp: u8,
    pub pc: u16,
}

impl<B: Bus> Cpu6502<B> {
    pub fn new(bus: B) -> Self {
        Self {
            bus,
            reg: RegisterState::default(),
            sp: 255,
            pc: 0,
        }
    }

    pub fn reset(&mut self) {
        self.reg.interrupt_disable = true;
        self.pc = self.read_absolute(RESET_VECTOR);
    }

    fn read_absolute(&mut self, addr: u16) -> u16 {
        u16::from_le_bytes([self.bus.read(addr), self.bus.read(addr.wrapping_add(1))])
    }

    fn stack_push(&mut self, value: u8) {
        self.bus.write(STACK_START + self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.bus.read(STACK_START + self.sp as u16)
    }

    fn take_byte_at_pc(&mut self) -> u8 {
        let byte = self.bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    /// #i
    fn take_immediate(&mut self) -> u16 {
        let addr = self.pc;
        self.take_byte_at_pc();
        addr
    }

    /// *+d
    fn take_relative(&mut self) -> u16 {
        let offset = self.take_byte_at_pc() as i8 as u16;
        self.pc.wrapping_add(offset)
    }

    /// d
    fn take_zero_page(&mut self) -> u8 {
        self.take_byte_at_pc()
    }

    /// (a)
    fn take_indirect(&mut self) -> u16 {
        let addr = self.take_absolute();
        self.read_absolute(addr)
    }

    /// a
    fn take_absolute(&mut self) -> u16 {
        u16::from_le_bytes([self.take_byte_at_pc(), self.take_byte_at_pc()])
    }

    /// a,x
    fn take_absolute_indexed_x(&mut self) -> u16 {
        self.take_absolute().wrapping_add(self.reg.x as u16)
    }

    /// a,y
    fn take_absolute_indexed_y(&mut self) -> u16 {
        self.take_absolute().wrapping_add(self.reg.y as u16)
    }

    /// d,x
    fn take_zero_page_indexed_x(&mut self) -> u8 {
        self.take_zero_page().wrapping_add(self.reg.x)
    }

    /// d,y
    fn take_zero_page_indexed_y(&mut self) -> u8 {
        self.take_zero_page().wrapping_add(self.reg.y)
    }

    /// (d,x)
    fn take_indexed_indirect(&mut self) -> u16 {
        let addr = self.take_zero_page_indexed_x() as u16;
        self.read_absolute(addr)
    }

    /// (d),y
    fn take_indirect_indexed(&mut self) -> u16 {
        let addr = self.take_zero_page() as u16;
        self.read_absolute(addr).wrapping_add(self.reg.y as u16)
    }

    pub fn step(&mut self) {
        let opcode = self.take_byte_at_pc();

        // It appears that 6502 opcodes have a layout of
        // AAABBBCC, where:
        // - AAA and CC define the opcode type
        // - BBB and CC define the addressing mode
        // - CC defines some sort of "opcode group"
        //
        // #i    - immediate value
        // d     - zero page address
        // *+d   - relative address
        // a     - absolute address
        // ($a)  - dereference $a
        // $l,$r - add $l and $r
        // [!]   - illegal opcode

        let aaa = (opcode >> 5) & 0b111;
        let bbb = (opcode >> 2) & 0b111;
        let cc = opcode & 0b11;
        match (aaa, bbb, cc) {
            (8.., _, _) | (_, 8.., _) | (_, _, 4..) => unreachable!(),
        
            // Group 0 (control flow ops)
            // Branch ops
            (op @ 0..=7, 4, 0) => {
                let addr = self.take_relative();
                let branch = match op {
                    0 => !self.reg.negative, // BPL *+d
                    1 => self.reg.negative, // BMI *+d
                    2 => !self.reg.overflow, // BVC *+d
                    3 => self.reg.overflow, // BVS *+d
                    4 => !self.reg.carry, // BCC *+d
                    5 => self.reg.carry, // BCS *+d
                    6 => !self.reg.zero, // BNE *+d
                    7 => self.reg.zero, // BEQ *+d
                    8.. => unreachable!()
                };
                if branch {
                    self.pc = addr;
                }
            }

            // Flag ops
            (0, 6, 0) => self.reg.carry = false, // CLC
            (1, 6, 0) => self.reg.carry = true, // SEC
            (2, 6, 0) => self.reg.interrupt_disable = false, // CLI
            (3, 6, 0) => self.reg.interrupt_disable = true, // SEI
            (5, 6, 0) => self.reg.overflow = false, // CLV
            (6, 6, 0) => self.reg.decimal = false, // CLD
            (7, 6, 0) => self.reg.decimal = true, // SED

            // Jumps and subroutine ops
            (2, 3, 0) => self.pc = self.take_absolute(), // JMP a
            (3, 3, 0) => self.pc = self.take_indirect(), // JMP (a)
            (1, 0, 0) => { // JSR a
                let subroutine = self.take_absolute();
                let return_addr = self.pc.wrapping_sub(1);
                let [ret_low, ret_high] = return_addr.to_le_bytes();
                self.stack_push(ret_high);
                self.stack_push(ret_low);
                self.pc = subroutine;
            }
            (3, 0, 0) => { // RTS
                let ret_low = self.stack_pop();
                let ret_high = self.stack_pop();
                let return_addr = u16::from_le_bytes([ret_low, ret_high]);
                self.pc = return_addr.wrapping_add(1);
            }

            // Interrupt ops
            (0, 0, 0) => { // BRK
                self.pc = self.pc.wrapping_add(1);
                let [pc_low, pc_high] = self.pc.to_le_bytes();
                self.stack_push(pc_high);
                self.stack_push(pc_low);
                self.stack_push(self.reg.get_status(true));
                self.reg.interrupt_disable = true;
                self.pc = self.read_absolute(IRQ_BRK_VECTOR);
            }
            (2, 0, 0) => { // RTI
                let status = self.stack_pop();
                self.reg.set_status(status);
                let ret_low = self.stack_pop();
                let ret_high = self.stack_pop();
                self.pc = u16::from_le_bytes([ret_low, ret_high]);
            }

            // Bit test ops
            (1, 1, 0) => { // BIT d
                let addr = self.take_zero_page() as u16;
                let n = self.bus.read(addr);
                self.reg.zero = self.reg.a & n == 0;
                self.reg.negative = n & 0b1000_0000 != 0;
                self.reg.overflow = n & 0b0100_0000 != 0;
            }
            (1, 3, 0) => { // BIT a
                let addr = self.take_absolute();
                let n = self.bus.read(addr);
                self.reg.zero = self.reg.a & n == 0;
                self.reg.negative = n & 0b1000_0000 != 0;
                self.reg.overflow = n & 0b0100_0000 != 0;
            }

            // Stack ops
            (0, 2, 0) => self.stack_push(self.reg.get_status(true)), // PHP
            (1, 2, 0) => { // PLP
                let status = self.stack_pop();
                self.reg.set_status(status);
            }
            (2, 2, 0) => self.stack_push(self.reg.a), // PHA
            (3, 2, 0) => { // PLA
                let n = self.stack_pop();
                self.reg.update_a(n)
            }

            (0, 1, 0) => {} // NOP d [!]
            (0, 3, 0) => {} // NOP a [!]
            (0, 5, 0) => {} // NOP d,x [!]
            (0, 7, 0) => {} // NOP a,x [!]
            (1, 5, 0) => {} // NOP d,x [!]
            (1, 7, 0) => {} // NOP a,x [!]
            (2, 1, 0) => {} // NOP d [!]
            (2, 5, 0) => {} // NOP d,x [!]
            (2, 7, 0) => {} // NOP a,x [!]
            (3, 1, 0) => {} // NOP d [!]
            (3, 5, 0) => {} // NOP d,x [!]
            (3, 7, 0) => {} // NOP a,x [!]

            (4, 6, 0) => self.reg.update_a(self.reg.y), // TYA
            (5, 2, 0) => self.reg.update_y(self.reg.a), // TAY
            (6, 2, 0) => self.reg.update_y(self.reg.y.wrapping_add(1)), // INY
            (7, 2, 0) => self.reg.update_x(self.reg.x.wrapping_add(1)), // INX
            (4, 2, 0) => self.reg.update_y(self.reg.y.wrapping_sub(1)), // DEY

            (4, 0, 0) => {} // NOP #i [!]
            (4, 1, 0) => { // STY d
                let addr = self.take_zero_page() as u16;
                self.bus.write(addr, self.reg.y);
            }
            (4, 3, 0) => { // STY a
                let addr = self.take_absolute();
                self.bus.write(addr, self.reg.y);
            }
            (4, 5, 0) => { // STY d,x
                let addr = self.take_zero_page_indexed_x() as u16;
                self.bus.write(addr, self.reg.y);
            }
            (4, 7, 0) => {} // SHY a,x [!]

            (5, 0, 0) => { // LDY #i
                let addr = self.take_immediate();
                let n = self.bus.read(addr);
                self.reg.update_y(n);
            }
            (5, 1, 0) => { // LDY d
                let addr = self.take_zero_page() as u16;
                let n = self.bus.read(addr);
                self.reg.update_y(n);
            }
            (5, 3, 0) => { // LDY a
                let addr = self.take_absolute();
                let n = self.bus.read(addr);
                self.reg.update_y(n);
            }
            (5, 5, 0) => { // LDY d,x
                let addr = self.take_zero_page_indexed_x() as u16;
                let n = self.bus.read(addr);
                self.reg.update_y(n);
            }
            (5, 7, 0) => { // LDY a,x
                let addr = self.take_absolute_indexed_x();
                let n = self.bus.read(addr);
                self.reg.update_y(n);
            }

            (6, 0, 0) => { // CPY #i
                let addr = self.take_immediate();
                let n = self.bus.read(addr);
                self.reg.update_nz_flags(self.reg.y.wrapping_sub(n));
                self.reg.carry = self.reg.y >= n;
            }
            (6, 1, 0) => { // CPY d
                let addr = self.take_zero_page() as u16;
                let n = self.bus.read(addr);
                self.reg.update_nz_flags(self.reg.y.wrapping_sub(n));
                self.reg.carry = self.reg.y >= n;
            }
            (6, 3, 0) => { // CPY a
                let addr = self.take_absolute();
                let n = self.bus.read(addr);
                self.reg.update_nz_flags(self.reg.y.wrapping_sub(n));
                self.reg.carry = self.reg.y >= n;
            }
            (6, 5, 0) => {} // NOP d,x [!]
            (6, 7, 0) => {} // NOP a,x [!]

            (7, 0, 0) => { // CPX #i
                let addr = self.take_immediate();
                let n = self.bus.read(addr);
                self.reg.update_nz_flags(self.reg.x.wrapping_sub(n));
                self.reg.carry = self.reg.x >= n;
            }
            (7, 1, 0) => { // CPX d
                let addr = self.take_zero_page() as u16;
                let n = self.bus.read(addr);
                self.reg.update_nz_flags(self.reg.x.wrapping_sub(n));
                self.reg.carry = self.reg.x >= n;
            }
            (7, 3, 0) => { // CPX a
                let addr = self.take_absolute();
                let n = self.bus.read(addr);
                self.reg.update_nz_flags(self.reg.x.wrapping_sub(n));
                self.reg.carry = self.reg.x >= n;
            }
            (7, 5, 0) => {} // NOP d,x [!]
            (7, 7, 0) => {} // NOP a,x [!]
        
            // Group 1 (ALU ops)
            (op, addr_mode, 1) => {
                let addr_writable = addr_mode != 2;
                let addr = match addr_mode {
                    0 => self.take_indexed_indirect(), // (d,x)
                    1 => self.take_zero_page() as u16, // d
                    2 => self.take_immediate(), // #i
                    3 => self.take_absolute(), // a
                    4 => self.take_indirect_indexed(), // (d),y
                    5 => self.take_zero_page_indexed_x() as u16, // d,x
                    6 => self.take_absolute_indexed_y(), // a,y
                    7 => self.take_absolute_indexed_x(), // a,x,
                    8.. => unreachable!()
                };
        
                match op {
                    0 => { // ORA
                        let n = self.bus.read(addr);
                        self.reg.update_a(self.reg.a | n);
                    }
                    1 => { // AND
                        let n = self.bus.read(addr);
                        self.reg.update_a(self.reg.a & n);
                    }
                    2 => { // EOR
                        let n = self.bus.read(addr);
                        self.reg.update_a(self.reg.a ^ n);
                    }
                    3 => { // ADC
                        // this is weird as hell will explain later maybe
                        let operand = self.bus.read(addr) as u16;
                        let carry = self.reg.carry as u16;
                        let result = self.reg.a as u16 + operand + carry;
                        let sevenbit_result = (self.reg.a as u16 & 0x7F) + (operand & 0x7F) + carry;
                        let carryout = result > 0xFF;
                        self.reg.carry = carryout;
                        self.reg.overflow = carryout != (sevenbit_result > 0x7F);
                        self.reg.update_nz_flags(result as u8);
                        // self.reg.a is directly set here because decimal mode is weird
                        if self.reg.decimal {
                            self.reg.carry = false;
                            let mut lower = (self.reg.a as u16 & 0xF) + (operand & 0xF) + carry;
                            let mut upper = (self.reg.a as u16 >> 4) + (operand >> 4);
                            if lower >= 10 {
                                lower = (lower - 10) & 0xF;
                                upper += 1;
                            }
                            if upper >= 10 {
                                upper = (upper - 10) & 0xF;
                                self.reg.carry = true;
                            }
                            self.reg.a = ((upper << 4) | lower) as u8;
                        } else {
                            self.reg.a = result as u8;
                        }
                    }
                    4 => { // STA
                        if addr_writable {
                            self.bus.write(addr, self.reg.a);
                        }
                        // !addr_writable case is NOP #i [!]
                    }
                    5 => { // LDA
                        let n = self.bus.read(addr);
                        self.reg.update_a(n);
                    }
                    6 => { // CMP
                        let n = self.bus.read(addr);
                        self.reg.update_nz_flags(self.reg.a.wrapping_sub(n));
                        self.reg.carry = self.reg.a >= n;
                    }
                    7 => { // SBC
                        let operand = (!self.bus.read(addr)) as u16;
                        let carry = self.reg.carry as u16;
                        let result = self.reg.a as u16 + operand + carry;
                        let sevenbit_result = (self.reg.a as u16 & 0x7F) + (operand & 0x7F) + carry;
                        let carryout = result > 0xFF;
                        self.reg.carry = carryout;
                        self.reg.overflow = carryout != (sevenbit_result > 0x7F);
                        self.reg.update_nz_flags(result as u8);
                        // self.reg.a is directly set here because decimal mode is weird
                        if self.reg.decimal {
                            let operand = (!(operand as u8)) as u16;
                            self.reg.carry = true;
                            let mut lower = (self.reg.a as u16 & 0xF)
                                .wrapping_sub(operand & 0xF)
                                .wrapping_sub(1 - carry);
                            let mut upper = (self.reg.a as u16 >> 4)
                                .wrapping_sub(operand >> 4);
                            if lower >= 10 {
                                lower = (lower.wrapping_add(10)) & 0xF;
                                upper = upper.wrapping_sub(1);
                            }
                            if upper >= 10 {
                                upper = (upper.wrapping_add(10)) & 0xF;
                                self.reg.carry = false;
                            }
                            self.reg.a = ((upper << 4) | lower) as u8;
                        } else {
                            self.reg.a = result as u8;
                        }
                    }
                    8.. => unreachable!()
                }
            }
        
            // Group 2 (read-modify-write ops)
            // ASL, ROL, LSR, ROR
            (0..=3, 0, 2) => {} // STP [!]
            (0..=3, 4, 2) => {} // STP [!]
            (0..=3, 6, 2) => {} // NOP [!]
            (op @ 0..=3, 2, 2) => { // accumulator case
                let n = self.reg.a;
                let result = match op {
                    0 => { // ASL
                        self.reg.carry = n & 0b1000_0000 != 0;
                        n << 1
                    }
                    1 => { // ROL
                        let carry = self.reg.carry;
                        self.reg.carry = n & 0b1000_0000 != 0;
                        (n << 1) | carry as u8
                    }
                    2 => { // LSR
                        self.reg.carry = n & 0b0000_0001 != 0;
                        n >> 1
                    }
                    3 => { // ROR
                        let carry = self.reg.carry;
                        self.reg.carry = n & 0b0000_0001 != 0;
                        (n >> 1) | ((carry as u8) << 7)
                    }
                    4.. => unreachable!()
                };
                self.reg.update_a(result);
            }
            (op @ 0..=3, addr_mode @ (1 | 3 | 5 | 7), 2) => { // non-accumulator case
                let addr = match addr_mode {
                    1 => self.take_zero_page() as u16, // d
                    3 => self.take_absolute(), // a
                    5 => self.take_zero_page_indexed_x() as u16, // d,x
                    7 => self.take_absolute_indexed_x(), // a,x
                    _ => unreachable!()
                };
                let n = self.bus.read(addr);
                let result = match op {
                    0 => { // ASL
                        self.reg.carry = n & 0b1000_0000 != 0;
                        n << 1
                    }
                    1 => { // ROL
                        let carry = self.reg.carry;
                        self.reg.carry = n & 0b1000_0000 != 0;
                        (n << 1) | carry as u8
                    }
                    2 => { // LSR
                        self.reg.carry = n & 0b0000_0001 != 0;
                        n >> 1
                    }
                    3 => { // ROR
                        let carry = self.reg.carry;
                        self.reg.carry = n & 0b0000_0001 != 0;
                        (n >> 1) | ((carry as u8) << 7)
                    }
                    4.. => unreachable!()
                };
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
        
            // Store ops
            (4, 0, 2) => {} // NOP #i [!]
            (4, 1, 2) => { // STX d
                let addr = self.take_zero_page() as u16;
                self.bus.write(addr, self.reg.x);
            }
            (4, 2, 2) => self.reg.update_a(self.reg.x), // TXA
            (4, 3, 2) => { // STX a
                let addr = self.take_absolute();
                self.bus.write(addr, self.reg.x);
            }
            (4, 4, 2) => {} // STP [!]
            (4, 5, 2) => { // STX d,y
                let addr = self.take_zero_page_indexed_y() as u16;
                self.bus.write(addr, self.reg.x);
            }
            (4, 6, 2) => self.sp = self.reg.x, // TXS
            (4, 7, 2) => {} // SHX a,y [!]

            // Load ops
            (5, 0, 2) => { // LDX #i
                let addr = self.take_immediate();
                let n = self.bus.read(addr);
                self.reg.update_x(n);
            }
            (5, 1, 2) => { // LDX d
                let addr = self.take_zero_page() as u16;
                let n = self.bus.read(addr);
                self.reg.update_x(n);
            }
            (5, 2, 2) => self.reg.update_x(self.reg.a), // TAX
            (5, 3, 2) => { // LDX a
                let addr = self.take_absolute();
                let n = self.bus.read(addr);
                self.reg.update_x(n);
            }
            (5, 4, 2) => {} // STP [!]
            (5, 5, 2) => { // LDX d,y
                let addr = self.take_zero_page_indexed_y() as u16;
                let n = self.bus.read(addr);
                self.reg.update_x(n);
            }
            (5, 6, 2) => self.reg.update_x(self.sp), // TSX
            (5, 7, 2) => { // LDX a,y
                let addr = self.take_absolute_indexed_y();
                let n = self.bus.read(addr);
                self.reg.update_x(n);
            }

            // Decrement ops
            (6, 0, 2) => {} // NOP #i [!]
            (6, 1, 2) => { // DEC d
                let addr = self.take_zero_page() as u16;
                let n = self.bus.read(addr);
                let result = n.wrapping_sub(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
            (6, 2, 2) => self.reg.update_x(self.reg.x.wrapping_sub(1)), // DEX
            (6, 3, 2) => { // DEC a
                let addr = self.take_absolute();
                let n = self.bus.read(addr);
                let result = n.wrapping_sub(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
            (6, 4, 2) => {} // STP [!]
            (6, 5, 2) => { // DEC d,x
                let addr = self.take_zero_page_indexed_x() as u16;
                let n = self.bus.read(addr);
                let result = n.wrapping_sub(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
            (6, 6, 2) => {} // NOP [!]
            (6, 7, 2) => { // DEC a,x
                let addr = self.take_absolute_indexed_x();
                let n = self.bus.read(addr);
                let result = n.wrapping_sub(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }

            // Increment ops
            (7, 0, 2) => {} // NOP #i [!]
            (7, 1, 2) => { // INC d
                let addr = self.take_zero_page() as u16;
                let n = self.bus.read(addr);
                let result = n.wrapping_add(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
            (7, 2, 2) => {} // NOP
            (7, 3, 2) => { // INC a
                let addr = self.take_absolute();
                let n = self.bus.read(addr);
                let result = n.wrapping_add(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
            (7, 4, 2) => {} // STP [!]
            (7, 5, 2) => { // INC d,x
                let addr = self.take_zero_page_indexed_x() as u16;
                let n = self.bus.read(addr);
                let result = n.wrapping_add(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
            (7, 6, 2) => {} // NOP [!]
            (7, 7, 2) => { // INC a,x
                let addr = self.take_absolute_indexed_x();
                let n = self.bus.read(addr);
                let result = n.wrapping_add(1);
                self.bus.write(addr, result);
                self.reg.update_nz_flags(result);
            }
        
            // Group 3 (weird unofficial ops)
            (0, 0, 3) => {} // SLO (d,x) [!]
            (0, 1, 3) => {} // SLO d [!]
            (0, 2, 3) => {} // ANC #i [!]
            (0, 3, 3) => {} // SLO a [!]
            (0, 4, 3) => {} // SLO (d),y [!]
            (0, 5, 3) => {} // SLO d,x [!]
            (0, 6, 3) => {} // SLO a,y [!]
            (0, 7, 3) => {} // SLO a,x [!]
            (1, 0, 3) => {} // RLA (d,x) [!]
            (1, 1, 3) => {} // RLA d [!]
            (1, 2, 3) => {} // ANC #i [!]
            (1, 3, 3) => {} // RLA a [!]
            (1, 4, 3) => {} // RLA (d),y [!]
            (1, 5, 3) => {} // RLA d,x [!]
            (1, 6, 3) => {} // RLA a,y [!]
            (1, 7, 3) => {} // RLA a,x [!]
            (2, 0, 3) => {} // SRE (d,x) [!]
            (2, 1, 3) => {} // SRE d [!]
            (2, 2, 3) => {} // ALR #i [!]
            (2, 3, 3) => {} // SRE a [!]
            (2, 4, 3) => {} // SRE (d),y [!]
            (2, 5, 3) => {} // SRE d,x [!]
            (2, 6, 3) => {} // SRE a,y [!]
            (2, 7, 3) => {} // SRE a,x [!]
            (3, 0, 3) => {} // RRA (d,x) [!]
            (3, 1, 3) => {} // RRA d [!]
            (3, 2, 3) => {} // ARR #i [!]
            (3, 3, 3) => {} // RRA a [!]
            (3, 4, 3) => {} // RRA (d),y [!]
            (3, 5, 3) => {} // RRA d,x [!]
            (3, 6, 3) => {} // RRA a,y [!]
            (3, 7, 3) => {} // RRA a,x [!]
            (4, 0, 3) => {} // SAX (d,x) [!]
            (4, 1, 3) => {} // SAX d [!]
            (4, 2, 3) => {} // XAA #i [!]
            (4, 3, 3) => {} // SAX a [!]
            (4, 4, 3) => {} // AHX (d),y [!]
            (4, 5, 3) => {} // SAX d,y [!]
            (4, 6, 3) => {} // TAS a,y [!]
            (4, 7, 3) => {} // AHX a,y [!]
            (5, 0, 3) => {} // LAX (d,x) [!]
            (5, 1, 3) => {} // LAX d [!]
            (5, 2, 3) => {} // LAX #i [!]
            (5, 3, 3) => {} // LAX a [!]
            (5, 4, 3) => {} // LAX (d),y [!]
            (5, 5, 3) => {} // LAX d,y [!]
            (5, 6, 3) => {} // LAS a,y [!]
            (5, 7, 3) => {} // LAX a,y [!]
            (6, 0, 3) => {} // DCP (d,x) [!]
            (6, 1, 3) => {} // DCP d [!]
            (6, 2, 3) => {} // AXS #i [!]
            (6, 3, 3) => {} // DCP a [!]
            (6, 4, 3) => {} // DCP (d),y [!]
            (6, 5, 3) => {} // DCP d,x [!]
            (6, 6, 3) => {} // DCP a,y [!]
            (6, 7, 3) => {} // DCP a,x [!]
            (7, 0, 3) => {} // ISC (d,x) [!]
            (7, 1, 3) => {} // ISC d [!]
            (7, 2, 3) => {} // SBC #i [!]
            (7, 3, 3) => {} // ISC a [!]
            (7, 4, 3) => {} // ISC (d),y [!]
            (7, 5, 3) => {} // ISC d,x [!]
            (7, 6, 3) => {} // ISC a,y [!]
            (7, 7, 3) => {} // ISC a,x [!]
        }        
    }
}
