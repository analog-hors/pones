use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread::{spawn, JoinHandle};
use std::io::prelude::*;
use console::{Term, Key};
use pones_6502::*;

const PROGRAM: &[u8] = include_bytes!("../ehbasic/basic.bin");
const PROGRAM_ADDR: u16 = 0xC000;
const IO_READ_ADDR: u16 = 0xF004;
const IO_WRITE_ADDR: u16 = 0xF001;

struct EhBasicBus {
    mem: [u8; 65536],
    term: Term,
    input_rx: Receiver<Key>,
    _input_thread: JoinHandle<()>
}

impl EhBasicBus {
    pub fn new(term: Term) -> Self {
        let mut mem = [0; 65536];
        mem[PROGRAM_ADDR as usize..PROGRAM_ADDR as usize + PROGRAM.len()]
            .copy_from_slice(PROGRAM);

        let (input_tx, input_rx) = channel();
        let _input_thread = spawn({
            let term = term.clone();
            move || loop {
                let key = term.read_key().expect("failed to read char");
                if input_tx.send(key).is_err() {
                    break;
                }
            }
        });

        Self { mem, term, input_rx, _input_thread }
    }
}

impl Bus for EhBasicBus {
    fn read(&mut self, addr: u16) -> u8 {
        if addr == IO_READ_ADDR {
            return match self.input_rx.try_recv() {
                Ok(Key::Enter) => b'\r',
                Ok(Key::Backspace) => 8,
                Ok(Key::Char(c)) => c as u8,
                Ok(_) | Err(TryRecvError::Empty) => 0,
                Err(TryRecvError::Disconnected) => panic!("input channel disconnected")
            };
        }
        self.mem[addr as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        if addr == IO_WRITE_ADDR {
            match value {
                8 => self.term.clear_chars(1).expect("failed to clear last char"),
                c => write!(&mut self.term, "{}", c as char).expect("failed to write output"),
            }            
        }
        self.mem[addr as usize] = value;
    }
}

fn main() {
    let term = Term::stdout();
    let bus = EhBasicBus::new(term);
    let mut cpu = Cpu6502::new(bus);
    cpu.pc = PROGRAM_ADDR;
    cpu.reset();
    loop {
        cpu.step();
    }
}
