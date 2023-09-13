use std::io::BufRead;

use pones::NesEmulator;
use pones::cart::INesCart;

struct NesTestEntry {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    sp: u8
}

fn nestest_log() -> impl Iterator<Item=NesTestEntry> {
    include_bytes!("data/nestest.log").lines().map(|line| {
        let line = line.unwrap();
        let reg = |prefix: &str| {
            let index = line.find(prefix).unwrap() + prefix.len();
            u8::from_str_radix(&line[index..index + 2], 16).unwrap()
        };

        NesTestEntry {
            pc: u16::from_str_radix(&line[0..4], 16).unwrap(),
            a: reg("A:"),
            x: reg("X:"),
            y: reg("Y:"),
            sp: reg("SP:"),
        }
    })
}

#[test]
pub fn nestest() {
    let mut nes = NesEmulator::new();
    let mut rom = include_bytes!("data/nestest.nes") as &[u8];
    let mut cart = INesCart::parse(&mut rom).expect("failed to parse nestest rom");
    let mut log = nestest_log();

    nes.cpu.pc = 0xC000;
    nes.cpu.sp = 0xFD;
    while let Some(entry) = log.next() {
        let mut pass = true;
        let check = |name, got, expected| {
            if got != expected {
                eprintln!("`{}` mismatch: {:#04X} != {:#04X}", name, got, expected);
                return false;
            }
            true
        };

        if nes.cpu.pc != entry.pc {
            eprintln!("`pc` mismatch: {:#06X} != {:#06X}", nes.cpu.pc, entry.pc);
            pass = false;
        }
        pass &= check("sp", nes.cpu.sp, entry.sp);
        pass &= check("a", nes.cpu.reg.a, entry.a);
        pass &= check("x", nes.cpu.reg.x, entry.x);
        pass &= check("y", nes.cpu.reg.y, entry.y);
        if !pass {
            eprintln!("pc: {:#06X}", nes.cpu.pc);
            eprintln!("sp: {:#04X}", nes.cpu.sp);
            eprintln!("a: {:#04X}", nes.cpu.reg.a);
            eprintln!("x: {:#04X}", nes.cpu.reg.x);
            eprintln!("y: {:#04X}", nes.cpu.reg.y);

            panic!("nestest failed");
        }

        nes.step(&mut cart);
    }
}
