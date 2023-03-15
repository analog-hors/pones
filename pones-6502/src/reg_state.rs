#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegisterState {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub carry: bool,
    pub zero: bool,
    pub interrupt_disable: bool,
    pub decimal: bool,
    pub overflow: bool,
    pub negative: bool
}

impl RegisterState {
    pub fn update_a(&mut self, value: u8) {
        self.a = value;
        self.update_nz_flags(self.a);
    }

    pub fn update_x(&mut self, value: u8) {
        self.x = value;
        self.update_nz_flags(self.x);
    }

    pub fn update_y(&mut self, value: u8) {
        self.y = value;
        self.update_nz_flags(self.y);
    }

    pub fn update_nz_flags(&mut self, value: u8) {
        self.negative = (value as i8).is_negative();
        self.zero = value == 0;
    }

    pub fn get_status(&self, brk: bool) -> u8 {
        let b = |flag, shift| (flag as u8) << shift;
        b(self.carry, 0)
            | b(self.zero, 1)
            | b(self.interrupt_disable, 2)
            | b(self.decimal, 3)
            | b(brk, 4)
            | b(true, 5)
            | b(self.overflow, 6)
            | b(self.negative, 7)
    }

    pub fn set_status(&mut self, value: u8) {
        let b = |shift: u8| value & (1 << shift) != 0;
        self.carry = b(0);
        self.zero = b(1);
        self.interrupt_disable = b(2);
        self.decimal = b(3);
        self.overflow = b(6);
        self.negative = b(7);
    }
}
