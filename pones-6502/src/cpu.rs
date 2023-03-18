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
            sp: 0,
            pc: 0,
        }
    }

    fn read_u16(&mut self, addr: u16) -> u16 {
        u16::from_le_bytes([self.bus.read(addr), self.bus.read(addr.wrapping_add(1))])
    }

    fn take_u8_at_pc(&mut self) -> u8 {
        let byte = self.bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    fn take_u16_at_pc(&mut self) -> u16 {
        u16::from_le_bytes([self.take_u8_at_pc(), self.take_u8_at_pc()])
    }

    fn stack_push(&mut self, value: u8) {
        self.bus.write(STACK_START + self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.bus.read(STACK_START + self.sp as u16)
    }

    fn interrupt(&mut self, vector: u16, brk: bool) {
        let [pc_low, pc_high] = self.pc.to_le_bytes();
        self.stack_push(pc_high);
        self.stack_push(pc_low);
        self.stack_push(self.reg.get_status(brk));
        self.reg.interrupt_disable = true;
        self.pc = self.read_u16(vector);
    }

    fn binary_adc(&mut self, operand: u8) {
        let operand = operand as u16;
        let carry = self.reg.carry as u16;
        let result = self.reg.a as u16 + operand + carry;
        let seven_bit_result = (self.reg.a as u16 & 0x7F) + (operand & 0x7F) + carry;
        let carry_out = result > 0xFF;
        let seven_bit_carry_out = seven_bit_result > 0x7F;
        self.reg.carry = carry_out;
        self.reg.overflow = carry_out != seven_bit_carry_out;
        self.reg.update_a(result as u8);
    }

    pub fn reset(&mut self) {
        self.reg.interrupt_disable = true;
        // Apparently RESET also attempts save the CPU state
        // to the stack, but it's hijacked to do reads instead
        // of writes. It still modifies sp, hence the subtraction.
        self.sp = self.sp.wrapping_sub(3);
        self.pc = self.read_u16(RESET_VECTOR);
    }

    pub fn irq(&mut self) {
        if !self.reg.interrupt_disable {
            self.interrupt(IRQ_BRK_VECTOR, false);
        }
    }

    pub fn nmi(&mut self) {
        self.interrupt(NMI_VECTOR, false);
    }

    // Branch ops
    fn bpl(&mut self, addr: u16) {
        if !self.reg.negative {
            self.pc = addr;
        }
    }
    
    fn bmi(&mut self, addr: u16) {
        if self.reg.negative {
            self.pc = addr;
        }
    }
    
    fn bvc(&mut self, addr: u16) {
        if !self.reg.overflow {
            self.pc = addr;
        }
    }
    
    fn bvs(&mut self, addr: u16) {
        if self.reg.overflow {
            self.pc = addr;
        }
    }
    
    fn bcc(&mut self, addr: u16) {
        if !self.reg.carry {
            self.pc = addr;
        }
    }
    
    fn bcs(&mut self, addr: u16) {
        if self.reg.carry {
            self.pc = addr;
        }
    }
    
    fn bne(&mut self, addr: u16) {
        if !self.reg.zero {
            self.pc = addr;
        }
    }
    
    fn beq(&mut self, addr: u16) {
        if self.reg.zero {
            self.pc = addr;
        }
    }

    // Flag ops
    fn clc_implied(&mut self) {
        self.reg.carry = false;
    }
    
    fn sec_implied(&mut self) {
        self.reg.carry = true;
    }
    
    fn cli_implied(&mut self) {
        self.reg.interrupt_disable = false;
    }
    
    fn sei_implied(&mut self) {
        self.reg.interrupt_disable = true;
    }
    
    fn cld_implied(&mut self) {
        self.reg.decimal = false;
    }
    
    fn sed_implied(&mut self) {
        self.reg.decimal = true;
    }

    fn clv_implied(&mut self) {
        self.reg.overflow = false;
    }
    
    // Jumps and subroutine ops
    fn jmp(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn jsr(&mut self, addr: u16) {
        let return_addr = self.pc.wrapping_sub(1);
        let [ret_low, ret_high] = return_addr.to_le_bytes();
        self.stack_push(ret_high);
        self.stack_push(ret_low);
        self.pc = addr;
    }

    fn rts_implied(&mut self) {
        let ret_low = self.stack_pop();
        let ret_high = self.stack_pop();
        let return_addr = u16::from_le_bytes([ret_low, ret_high]);
        self.pc = return_addr.wrapping_add(1);
    }

    // Interrupt ops
    fn brk_implied(&mut self) {
        self.pc = self.pc.wrapping_add(1);
        self.interrupt(IRQ_BRK_VECTOR, true);
    }

    fn rti_implied(&mut self) {
        let status = self.stack_pop();
        self.reg.set_status(status);
        let ret_low = self.stack_pop();
        let ret_high = self.stack_pop();
        self.pc = u16::from_le_bytes([ret_low, ret_high]);
    }

    // Stack ops
    fn php_implied(&mut self) {
        self.stack_push(self.reg.get_status(true));
    }
    
    fn plp_implied(&mut self) {
        let status = self.stack_pop();
        self.reg.set_status(status);
    }

    fn pha_implied(&mut self) {
        self.stack_push(self.reg.a);
    }

    fn pla_implied(&mut self) {
        let a = self.stack_pop();
        self.reg.update_a(a);
    }

    // Store and load ops
    fn sty(&mut self, addr: u16) {
        self.bus.write(addr, self.reg.y);
    }

    fn ldy(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_y(n);
    }

    fn stx(&mut self, addr: u16) {
        self.bus.write(addr, self.reg.x);
    }

    fn ldx(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_x(n);
    }

    fn sta(&mut self, addr: u16) {
        self.bus.write(addr, self.reg.a);
    }

    fn lda(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_a(n);
    }

    // Transfer ops
    fn tya_implied(&mut self) {
        self.reg.update_a(self.reg.y);
    }

    fn tay_implied(&mut self) {
        self.reg.update_y(self.reg.a);
    }

    fn txa_implied(&mut self) {
        self.reg.update_a(self.reg.x);
    }

    fn tax_implied(&mut self) {
        self.reg.update_x(self.reg.a);
    }

    fn txs_implied(&mut self) {
        self.sp = self.reg.x;
    }

    fn tsx_implied(&mut self) {
        self.reg.update_x(self.sp);
    }

    // Increment and decrement ops
    fn iny_implied(&mut self) {
        self.reg.update_y(self.reg.y.wrapping_add(1));
    }

    fn dey_implied(&mut self) {
        self.reg.update_y(self.reg.y.wrapping_sub(1))
    }

    fn inx_implied(&mut self) {
        self.reg.update_x(self.reg.x.wrapping_add(1));
    }

    fn dex_implied(&mut self) {
        self.reg.update_x(self.reg.x.wrapping_sub(1))
    }

    fn inc(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        let result = n.wrapping_add(1);
        self.bus.write(addr, result);
        self.reg.update_nz_flags(result);
    }

    fn dec(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        let result = n.wrapping_sub(1);
        self.bus.write(addr, result);
        self.reg.update_nz_flags(result);
    }

    // Compare ops
    fn cpy(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_nz_flags(self.reg.y.wrapping_sub(n));
        self.reg.carry = self.reg.y >= n;
    }

    fn cpx(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_nz_flags(self.reg.x.wrapping_sub(n));
        self.reg.carry = self.reg.x >= n;
    }

    fn cmp(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_nz_flags(self.reg.a.wrapping_sub(n));
        self.reg.carry = self.reg.a >= n;
    }

    // Math ops
    fn adc(&mut self, addr: u16) {
        let operand = self.bus.read(addr);
        if !self.reg.decimal {
            self.binary_adc(operand);
        } else {
            let mut carry_out = false;
            let mut lower = (self.reg.a & 0xF) + (operand & 0xF) + self.reg.carry as u8;
            let mut upper = (self.reg.a >> 4) + (operand >> 4);
            if lower >= 10 {
                lower = (lower - 10) & 0xF;
                upper += 1;
            }
            if upper >= 10 {
                upper = (upper - 10) & 0xF;
                carry_out = true;
            }
            let result = (upper << 4) | lower;
            self.reg.carry = carry_out;
            self.reg.update_a(result);
            //TODO set flags even in decimal mode
        }
    }

    fn sbc(&mut self, addr: u16) {
        let operand = self.bus.read(addr);
        if !self.reg.decimal {
            self.binary_adc(!operand); // works due to two's complement
        } else {
            let mut carry_out = true;
            let mut lower = (self.reg.a as i16 & 0xF) - (operand as i16 & 0xF) - !self.reg.carry as i16;
            let mut upper = (self.reg.a as i16 >> 4) - (operand as i16 >> 4);
            if lower & 0x10 != 0 {
                lower = (lower + 10) & 0xF;
                upper -= 1;
            }
            if upper & 0x10 != 0 {
                upper = (upper + 10) & 0xF;
                carry_out = false;
            }
            let result = (upper << 4) as u8 | lower as u8;
            self.reg.carry = carry_out;
            self.reg.update_a(result);
            //TODO set flags even in decimal mode
        }
    }

    // Bitwise ops
    fn ora(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_a(self.reg.a | n);
    }

    fn and(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_a(self.reg.a & n);
    }

    fn eor(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.update_a(self.reg.a ^ n);
    }

    fn bit(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.zero = self.reg.a & n == 0;
        self.reg.negative = n & 0b1000_0000 != 0;
        self.reg.overflow = n & 0b0100_0000 != 0;
    }

    // Bitwise read-modify-write ops
    fn asl(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.carry = n & 0b1000_0000 != 0;
        let result = n << 1;
        self.bus.write(addr, result);
        self.reg.update_nz_flags(result);
    }

    fn asl_implied(&mut self) {
        self.reg.carry = self.reg.a & 0b1000_0000 != 0;
        self.reg.update_a(self.reg.a << 1);
    }

    fn rol(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        let carry = self.reg.carry;
        self.reg.carry = n & 0b1000_0000 != 0;
        let result = (n << 1) | carry as u8;
        self.bus.write(addr, result);
        self.reg.update_nz_flags(result);
    }

    fn rol_implied(&mut self) {
        let carry = self.reg.carry;
        self.reg.carry = self.reg.a & 0b1000_0000 != 0;
        self.reg.update_a((self.reg.a << 1) | carry as u8);
    }

    fn lsr(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        self.reg.carry = n & 0b0000_0001 != 0;
        let result = n >> 1;
        self.bus.write(addr, result);
        self.reg.update_nz_flags(result);
    }

    fn lsr_implied(&mut self) {
        self.reg.carry = self.reg.a & 0b0000_0001 != 0;
        self.reg.update_a(self.reg.a >> 1);
    }

    fn ror(&mut self, addr: u16) {
        let n = self.bus.read(addr);
        let carry = self.reg.carry;
        self.reg.carry = n & 0b0000_0001 != 0;
        let result = (n >> 1) | ((carry as u8) << 7);
        self.bus.write(addr, result);
        self.reg.update_nz_flags(result);
    }

    fn ror_implied(&mut self) {
        let carry = self.reg.carry;
        self.reg.carry = self.reg.a & 0b0000_0001 != 0;
        self.reg.update_a((self.reg.a >> 1) | ((carry as u8) << 7));
    }

    // No op
    fn nop_implied(&mut self) {
    }

    pub fn step(&mut self) {
        // #i    - immediate value
        // d     - zero page address
        // *+d   - relative address
        // a     - absolute address
        // ($a)  - dereference $a
        // $l,$r - add $l and $r
        macro_rules! dispatch {
            ($($opcode:literal $handler:ident($($addr_mode:tt)*))*) => {
                match self.take_u8_at_pc() {
                    $($opcode => dispatch!(@call $handler $($addr_mode)*),)*
                    _ => {}
                }
            };

            (@call $handler:ident) => {{
                self.$handler();
            }};

            (@call $handler:ident "#i") => {{
                let addr = self.pc;
                self.take_u8_at_pc();
                self.$handler(addr);
            }};
            
            (@call $handler:ident "*+d") => {{
                let offset = self.take_u8_at_pc() as i8 as u16;
                let addr = self.pc.wrapping_add(offset);
                self.$handler(addr);
            }};
            
            (@call $handler:ident "d") => {{
                let addr = self.take_u8_at_pc() as u16;
                self.$handler(addr);
            }};
            
            (@call $handler:ident "(a)") => {{
                let addr = self.take_u16_at_pc();
                let addr = self.read_u16(addr);
                self.$handler(addr);
            }};
            
            (@call $handler:ident "a") => {{
                let addr = self.take_u16_at_pc();
                self.$handler(addr);
            }};
            
            (@call $handler:ident "a,x") => {{
                let addr = self.take_u16_at_pc().wrapping_add(self.reg.x as u16);
                self.$handler(addr);
            }};
            
            (@call $handler:ident "a,y") => {{
                let addr = self.take_u16_at_pc().wrapping_add(self.reg.y as u16);
                self.$handler(addr);
            }};
            
            (@call $handler:ident "d,x") => {{
                let addr = self.take_u8_at_pc().wrapping_add(self.reg.x) as u16;
                self.$handler(addr);
            }};
            
            (@call $handler:ident "d,y") => {{
                let addr = self.take_u8_at_pc().wrapping_add(self.reg.y) as u16;
                self.$handler(addr);
            }};
            
            (@call $handler:ident "(d,x)") => {{
                let addr = self.take_u8_at_pc().wrapping_add(self.reg.x) as u16;
                let addr = self.read_u16(addr);
                self.$handler(addr);
            }};
            
            (@call $handler:ident "(d),y") => {{
                let addr = self.take_u8_at_pc() as u16;
                let addr = self.read_u16(addr).wrapping_add(self.reg.y as u16);
                self.$handler(addr);
            }};
        }

        dispatch! {
            0x00 brk_implied()
            0x01 ora("(d,x)")
            // 0x02 stp_implied() // illegal
            // 0x03 slo("(d,x)") // illegal
            // 0x04 nop("d") // illegal
            0x05 ora("d")
            0x06 asl("d")
            // 0x07 slo("d") // illegal
            0x08 php_implied()
            0x09 ora("#i")
            0x0A asl_implied()
            // 0x0B anc("#i") // illegal
            // 0x0C nop("a") // illegal
            0x0D ora("a")
            0x0E asl("a")
            // 0x0F slo("a") // illegal
            0x10 bpl("*+d")
            0x11 ora("(d),y")
            // 0x12 stp_implied() // illegal
            // 0x13 slo("(d),y") // illegal
            // 0x14 nop("d,x") // illegal
            0x15 ora("d,x")
            0x16 asl("d,x")
            // 0x17 slo("d,x") // illegal
            0x18 clc_implied()
            0x19 ora("a,y")
            // 0x1A nop_implied() // illegal
            // 0x1B slo("a,y") // illegal
            // 0x1C nop("a,x") // illegal
            0x1D ora("a,x")
            0x1E asl("a,x")
            // 0x1F slo("a,x") // illegal
            0x20 jsr("a")
            0x21 and("(d,x)")
            // 0x22 stp_implied() // illegal
            // 0x23 rla("(d,x)") // illegal
            0x24 bit("d")
            0x25 and("d")
            0x26 rol("d")
            // 0x27 rla("d") // illegal
            0x28 plp_implied()
            0x29 and("#i")
            0x2A rol_implied()
            // 0x2B anc("#i") // illegal
            0x2C bit("a")
            0x2D and("a")
            0x2E rol("a")
            // 0x2F rla("a") // illegal
            0x30 bmi("*+d")
            0x31 and("(d),y")
            // 0x32 stp_implied() // illegal
            // 0x33 rla("(d),y") // illegal
            // 0x34 nop("d,x") // illegal
            0x35 and("d,x")
            0x36 rol("d,x")
            // 0x37 rla("d,x") // illegal
            0x38 sec_implied()
            0x39 and("a,y")
            // 0x3A nop_implied() // illegal
            // 0x3B rla("a,y") // illegal
            // 0x3C nop("a,x") // illegal
            0x3D and("a,x")
            0x3E rol("a,x")
            // 0x3F rla("a,x") // illegal
            0x40 rti_implied()
            0x41 eor("(d,x)")
            // 0x42 stp_implied() // illegal
            // 0x43 sre("(d,x)") // illegal
            // 0x44 nop("d") // illegal
            0x45 eor("d")
            0x46 lsr("d")
            // 0x47 sre("d") // illegal
            0x48 pha_implied()
            0x49 eor("#i")
            0x4A lsr_implied()
            // 0x4B alr("#i") // illegal
            0x4C jmp("a")
            0x4D eor("a")
            0x4E lsr("a")
            // 0x4F sre("a") // illegal
            0x50 bvc("*+d")
            0x51 eor("(d),y")
            // 0x52 stp_implied() // illegal
            // 0x53 sre("(d),y") // illegal
            // 0x54 nop("d,x") // illegal
            0x55 eor("d,x")
            0x56 lsr("d,x")
            // 0x57 sre("d,x") // illegal
            0x58 cli_implied()
            0x59 eor("a,y")
            // 0x5A nop_implied() // illegal
            // 0x5B sre("a,y") // illegal
            // 0x5C nop("a,x") // illegal
            0x5D eor("a,x")
            0x5E lsr("a,x")
            // 0x5F sre("a,x") // illegal
            0x60 rts_implied()
            0x61 adc("(d,x)")
            // 0x62 stp_implied() // illegal
            // 0x63 rra("(d,x)") // illegal
            // 0x64 nop("d") // illegal
            0x65 adc("d")
            0x66 ror("d")
            // 0x67 rra("d") // illegal
            0x68 pla_implied()
            0x69 adc("#i")
            0x6A ror_implied()
            // 0x6B arr("#i") // illegal
            0x6C jmp("(a)")
            0x6D adc("a")
            0x6E ror("a")
            // 0x6F rra("a") // illegal
            0x70 bvs("*+d")
            0x71 adc("(d),y")
            // 0x72 stp_implied() // illegal
            // 0x73 rra("(d),y") // illegal
            // 0x74 nop("d,x") // illegal
            0x75 adc("d,x")
            0x76 ror("d,x")
            // 0x77 rra("d,x") // illegal
            0x78 sei_implied()
            0x79 adc("a,y")
            // 0x7A nop_implied() // illegal
            // 0x7B rra("a,y") // illegal
            // 0x7C nop("a,x") // illegal
            0x7D adc("a,x")
            0x7E ror("a,x")
            // 0x7F rra("a,x") // illegal
            // 0x80 nop("#i") // illegal
            0x81 sta("(d,x)")
            // 0x82 nop("#i") // illegal
            // 0x83 sax("(d,x)") // illegal
            0x84 sty("d")
            0x85 sta("d")
            0x86 stx("d")
            // 0x87 sax("d") // illegal
            0x88 dey_implied()
            // 0x89 nop("#i") // illegal
            0x8A txa_implied()
            // 0x8B xaa("#i") // illegal
            0x8C sty("a")
            0x8D sta("a")
            0x8E stx("a")
            // 0x8F sax("a") // illegal
            0x90 bcc("*+d")
            0x91 sta("(d),y")
            // 0x92 stp_implied() // illegal
            // 0x93 ahx("(d),y") // illegal
            0x94 sty("d,x")
            0x95 sta("d,x")
            0x96 stx("d,y")
            // 0x97 sax("d,y") // illegal
            0x98 tya_implied()
            0x99 sta("a,y")
            0x9A txs_implied()
            // 0x9B tas("a,y") // illegal
            // 0x9C shy("a,x") // illegal
            0x9D sta("a,x")
            // 0x9E shx("a,y") // illegal
            // 0x9F ahx("a,y") // illegal
            0xA0 ldy("#i")
            0xA1 lda("(d,x)")
            0xA2 ldx("#i")
            // 0xA3 lax("(d,x)") // illegal
            0xA4 ldy("d")
            0xA5 lda("d")
            0xA6 ldx("d")
            // 0xA7 lax("d") // illegal
            0xA8 tay_implied()
            0xA9 lda("#i")
            0xAA tax_implied()
            // 0xAB lax("#i") // illegal
            0xAC ldy("a")
            0xAD lda("a")
            0xAE ldx("a")
            // 0xAF lax("a") // illegal
            0xB0 bcs("*+d")
            0xB1 lda("(d),y")
            // 0xB2 stp_implied() // illegal
            // 0xB3 lax("(d),y") // illegal
            0xB4 ldy("d,x")
            0xB5 lda("d,x")
            0xB6 ldx("d,y")
            // 0xB7 lax("d,y") // illegal
            0xB8 clv_implied()
            0xB9 lda("a,y")
            0xBA tsx_implied()
            // 0xBB las("a,y") // illegal
            0xBC ldy("a,x")
            0xBD lda("a,x")
            0xBE ldx("a,y")
            // 0xBF lax("a,y") // illegal
            0xC0 cpy("#i")
            0xC1 cmp("(d,x)")
            // 0xC2 nop("#i") // illegal
            // 0xC3 dcp("(d,x)") // illegal
            0xC4 cpy("d")
            0xC5 cmp("d")
            0xC6 dec("d")
            // 0xC7 dcp("d") // illegal
            0xC8 iny_implied()
            0xC9 cmp("#i")
            0xCA dex_implied()
            // 0xCB axs("#i") // illegal
            0xCC cpy("a")
            0xCD cmp("a")
            0xCE dec("a")
            // 0xCF dcp("a") // illegal
            0xD0 bne("*+d")
            0xD1 cmp("(d),y")
            // 0xD2 stp_implied() // illegal
            // 0xD3 dcp("(d),y") // illegal
            // 0xD4 nop("d,x") // illegal
            0xD5 cmp("d,x")
            0xD6 dec("d,x")
            // 0xD7 dcp("d,x") // illegal
            0xD8 cld_implied()
            0xD9 cmp("a,y")
            // 0xDA nop_implied() // illegal
            // 0xDB dcp("a,y") // illegal
            // 0xDC nop("a,x") // illegal
            0xDD cmp("a,x")
            0xDE dec("a,x")
            // 0xDF dcp("a,x") // illegal
            0xE0 cpx("#i")
            0xE1 sbc("(d,x)")
            // 0xE2 nop("#i") // illegal
            // 0xE3 isc("(d,x)") // illegal
            0xE4 cpx("d")
            0xE5 sbc("d")
            0xE6 inc("d")
            // 0xE7 isc("d") // illegal
            0xE8 inx_implied()
            0xE9 sbc("#i")
            0xEA nop_implied()
            // 0xEB sbc("#i") // illegal
            0xEC cpx("a")
            0xED sbc("a")
            0xEE inc("a")
            // 0xEF isc("a") // illegal
            0xF0 beq("*+d")
            0xF1 sbc("(d),y")
            // 0xF2 stp_implied() // illegal
            // 0xF3 isc("(d),y") // illegal
            // 0xF4 nop("d,x") // illegal
            0xF5 sbc("d,x")
            0xF6 inc("d,x")
            // 0xF7 isc("d,x") // illegal
            0xF8 sed_implied()
            0xF9 sbc("a,y")
            // 0xFA nop_implied() // illegal
            // 0xFB isc("a,y") // illegal
            // 0xFC nop("a,x") // illegal
            0xFD sbc("a,x")
            0xFE inc("a,x")
            // 0xFF isc("a,x") // illegal
        }
    }
}
