#![allow(non_snake_case, non_camel_case_types)]

use super::*;
use std::fmt;
use std::num::Wrapping;

/* InstructionHandler takes CPU reference for register updates and 2 instruction operands as arguments. 
 * When instruction length is less than 3 the redundant bytes should be ignored. 
 * Handler returns number of machine cycles consumed. Hardcoding cycles wouldn't, because 
 * conditional jumps/calls take varying number of cycles.
 */
type InstructionHandler<T> = FnMut(&mut CPU, &mut State<T>, u8, u8, u8) -> u8;

struct Instruction<'a, T: BankController> {
    mnemo: &'a str,
    size: u8,
    handler: Box<InstructionHandler<T>>,
}
impl <'a, T: BankController>Instruction<'a, T> {
    pub fn new(mnemo: &'a str, size: u8, handler: Box<InstructionHandler<T>>) -> Self {
        Self { mnemo: mnemo, size: size, handler: handler, }
    }
}

// Retruns word from two bytes
fn word(upper: u8, lower: u8) -> u16 {
    ((upper as u16) << 8) + (lower as u16)
}
fn word_split(val: u16) -> (u8, u8) {
    ((val >> 8) as u8, (val & 0xFF) as u8)
}

// Predicates for carry flag check
fn add_b_carry(op1: u8, op2: u8) -> bool { op1 as u16 + op2 as u16 > 0xFF }
fn add_w_carry(op1: u16, op2: u16) -> bool { op1 as u32 + op2 as u32 > 0xFFFF }
fn sub_b_carry(op1: u8, op2: u8) -> bool { op1 < op2 }
fn sub_w_carry(op1: u16, op2: u16) -> bool { op1 < op2 }

// Predicates for half carry flag check
fn add_b_hcarry(op1: u8, op2: u8) -> bool { ((op1 & 0xF) + (op2 & 0xF)) & 0x10 == 0x10 }
fn add_w_hcarry(op1: u16, op2: u16) -> bool { ((op1 & 0xFFF) + (op2 & 0xFFF)) & 0x1000 == 0x1000 }
fn sub_b_hcarry(op1: u8, op2: u8) -> bool { (op1 & 0xF) < (op2 & 0xF) }
fn sub_w_hcarry(op1: u16, op2: u16) -> bool { (op1 & 0xFFF) < (op2 & 0xFFF) }

// Safe add/sub to prevent runtime overflow errors
fn safe_b_add(op1: u8, op2: u8) -> u8 { (Wrapping(op1) + Wrapping(op2)).0 }
fn safe_w_add(op1: u16, op2: u16) -> u16 { (Wrapping(op1) + Wrapping(op2)).0 }
fn safe_b_sub(op1: u8, op2: u8) -> u8 { (Wrapping(op1) - Wrapping(op2)).0 }
fn safe_w_sub(op1: u16, op2: u16) -> u16 { (Wrapping(op1) - Wrapping(op2)).0 }

pub const ZP_ADDR: u16 = 0xFF00;
const B_IDX: u8 = 0;
const C_IDX: u8 = 1;
const D_IDX: u8 = 2;
const E_IDX: u8 = 3;
const H_IDX: u8 = 4;
const L_IDX: u8 = 5;
const ADDR_HL_IDX: u8 = 6;
const A_IDX: u8 = 7;

/* Decoder for Gameboy CPU (LR35902) instructions */
fn decode<T: BankController>(op: u8) -> Option<Instruction<'static, T>> {    
    let (mnemo, size, f): (&str, u8, Box<InstructionHandler<T>>) = match op {
        /* Misc/Control instructions */
        0x00 => ("NOP",    1, Box::new(|_, _, _, _, _| 1)),
        0x10 => ("STOP 0", 1, Box::new(|cpu, _, _, _, _| { cpu.STOP = true; 1 })),
        0x76 => ("HALT",   1, Box::new(|cpu, _, _, _, _| { cpu.HALT = true; 1 })),
        0xF4 => ("DI",     1, Box::new(|cpu, _, _, _, _| { cpu.IME = false; 1 })),
        0xFB => ("EI",     1, Box::new(|cpu, _, _, _, _| { cpu.IME = true; 1 })),
        // Decimal adjust A
        0x27 => ("DAA", 1, Box::new(|cpu, _, _, _, _| {
            1 // To be done when test roms will work
        })),
        // Set carry flag
        0x37 => ("SCF", 1, Box::new(|cpu, _, _, _, _| {
            cpu.N = false;
            cpu.H = false;
            cpu.C = true;
            1
        })),
        // Flip all bits in A
        0x2F => ("CPL", 1, Box::new(|cpu, _, _, _, _| {
            cpu.N = true;
            cpu.H = true;
            cpu.A ^= 0xFF;
            1
        })),
        // Flip carry flag(complement)
        0x3F => ("CCF", 1, Box::new(|cpu, _, _, _, _| {
            cpu.N = false;
            cpu.H = false;
            cpu.C ^= true;
            1
        })),

        /* 8bit load/store/move instructions */
        // To B register
        0x40 | 0x41 | 0x42 | 0x43 | 0x44 | 0x45 | 0x46 | 0x47 => ("LD B, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.BC.set_up(val);
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // To C register
        0x48 | 0x49 | 0x4A | 0x4B | 0x4C | 0x4D | 0x4E | 0x4F => ("LD C, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.BC.set_low(val);
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // To D register
        0x50 | 0x51 | 0x52 | 0x53 | 0x54 | 0x55 | 0x56 | 0x57 => ("LD D, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.DE.set_up(val);
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // To E register
        0x58 | 0x59 | 0x5A | 0x5B | 0x5C | 0x5D | 0x5E | 0x5F => ("LD E, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.DE.set_low(val);
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // To H register
        0x60 | 0x61 | 0x62 | 0x63 | 0x64 | 0x65 | 0x66 | 0x67 => ("LD H, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.HL.set_up(val);
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // To L register
        0x68 | 0x69 | 0x6A | 0x6B | 0x6C | 0x6D | 0x6E | 0x6F => ("LD L, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.HL.set_low(val);
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // To (HL) from register
        0x70 | 0x71 | 0x72 | 0x73 | 0x74 | 0x75 | 0x77 => ("LD (HL), reg", 1, Box::new(|cpu, s, op, _, _| {
            let val = cpu.reg(s, op & 0x7);
            cpu.write_HL(s, val);
            2
        })),
        // To A register
        0x78 | 0x79 | 0x7A | 0x7B | 0x7C | 0x7D | 0x7E | 0x7F => ("LD A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.A = val;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // To (BC) from A
        0x02 => ("LD (BC), A",    1, Box::new(|cpu, s, _, _, _| { s.safe_write(cpu.BC.val(), cpu.A); 2 })),
        // To (DE) from A
        0x12 => ("LD (DE), A",    1, Box::new(|cpu, s, _, _, _| { s.safe_write(cpu.DE.val(), cpu.A); 2 })),
        // To (HL) from A with post-increment
        0x22 => ("LD (HL+), A",   1, Box::new(|cpu, s, _, _, _| { 
            s.safe_write(cpu.HL.val(), cpu.A); 
            cpu.HL.set(safe_w_add(cpu.HL.val(), 1));
            2 
        })),
        // To (HL) from A with pre-decrement
        0x32 => ("LD (HL-), A",    1, Box::new(|cpu, s, _, _, _| { 
            cpu.HL.set(safe_w_sub(cpu.HL.val(), 1));
            s.safe_write(cpu.HL.val(), cpu.A); 
            2 
        })),
        // To A from (BC)
        0x0A => ("LD A, (BC)",    1, Box::new(|cpu, s, _, _, _| { cpu.A = s.safe_read(cpu.BC.val()); 2 })),
        // To A from (DE)
        0x1A => ("LD A, (DE)",    1, Box::new(|cpu, s, _, _, _| { cpu.A = s.safe_read(cpu.DE.val()); 2 })),
        // To A from (HL) with post-increment
        0x2A => ("LD A, (HL+)",   1, Box::new(|cpu, s, _, _, _| { 
            cpu.A = s.safe_read(cpu.HL.val()); 
            cpu.HL.set(safe_w_add(cpu.HL.val(), 1));
            2 
        })),
        // To A from (HL) with pre-decrement
        0x3A => ("LD A, (HL-)",   1, Box::new(|cpu, s, _, _, _| { 
            cpu.HL.set(safe_w_sub(cpu.HL.val(), 1));
            cpu.A = s.safe_read(cpu.HL.val()); 
            2 
        })),
        // To B from d8
        0x06 => ("LD B, d8",    2, Box::new(|cpu, _, _, op1, _| { cpu.BC.set_up(op1); 2 })),
        // To D from d8
        0x16 => ("LD D, d8",    2, Box::new(|cpu, _, _, op1, _| { cpu.DE.set_up(op1); 2 })),
        // To H from d8
        0x26 => ("LD H, d8",    2, Box::new(|cpu, _, _, op1, _| { cpu.HL.set_up(op1); 2 })),
        // To (HL) from d8
        0x36 => ("LD (HL), d8", 2, Box::new(|cpu, s, _, op1, _| { cpu.write_HL(s, op1); 3})),
        // To C from d8
        0x0E => ("LD C, d8",    2, Box::new(|cpu, _, _, op1, _| { cpu.BC.set_low(op1); 2 })),
        // To E from d8
        0x1E => ("LD E, d8",    2, Box::new(|cpu, _, _, op1, _| { cpu.DE.set_low(op1); 2 })),
        // To L from d8
        0x2E => ("LD L, d8",    2, Box::new(|cpu, _, _, op1, _| { cpu.HL.set_low(op1); 2 })),
        // To A from d8
        0x3E => ("LD A, d8",    2, Box::new(|cpu, _, _, op1, _| { cpu.A = op1; 2})),
        // To ($FF00 + a8) from A
        0xE0 => ("LDH (a8), A", 2, Box::new(|cpu, s, _, op1, _| { s.safe_write(ZP_ADDR + op1 as u16, cpu.A); 3 })),
        // To A from ($FF00 + a8)
        0xF0 => ("LDH A, (a8)", 2, Box::new(|cpu, s, _, op1, _| { cpu.A = s.safe_read(ZP_ADDR + op1 as u16); 3 })),
        // To ($FF00 + C) from A
        0xE2 => ("LD (C), A", 1, Box::new(|cpu, s, _, _, _| { s.safe_write(ZP_ADDR + cpu.BC.low() as u16, cpu.A); 2 })),
        // To A from ($FF00 + C)
        0xF2 => ("LD A, (C)", 1, Box::new(|cpu, s, _, _, _| { cpu.A = s.safe_read(ZP_ADDR + cpu.BC.low() as u16); 2 })),
        // To (a16) from A
        0xEA => ("LD (a16), A", 3, Box::new(|cpu, s, _, op1, op2| { s.safe_write(word(op2, op1), cpu.A); 4 })),
        // To A from (a16)
        0xFA => ("LD A, (a16)", 3, Box::new(|cpu, s, _, op1, op2| { cpu.A = s.safe_read(word(op2, op1)); 4 })),

        /* 16bit load/store/move instructions */
        // To BC from d16
        0x01 => ("LD BC, d16", 3, Box::new(|cpu, _, _, op1, op2| { cpu.BC.set(word(op2, op1)); 3 })),
        // To DE from d16
        0x11 => ("LD DE, d16", 3, Box::new(|cpu, _, _, op1, op2| { cpu.DE.set(word(op2, op1)); 3 })),
        // TO HL from d16
        0x21 => ("LD HL, d16", 3, Box::new(|cpu, _, _, op1, op2| { cpu.HL.set(word(op2, op1)); 3 })),
        // To SP from d16
        0x31 => ("LD SP, d16", 3, Box::new(|cpu, _, _, op1, op2| { cpu.SP = word(op2, op1); 3 })),
        // To (a16) from SP
        0x08 => ("LD (a16), SP", 3, Box::new(|cpu, s, _, op1, op2| { 
            let addr = word(op2, op1);
            s.write_word(addr, cpu.SP);
            5 
        })),
        // Value of SP+r8 to HL
        0xF8 => ("LD HL, SP+r8", 2, Box::new(|cpu, _, _, op1, _| {
            cpu.C = add_w_carry(cpu.SP, op1 as u16);
            cpu.H = add_w_hcarry(cpu.SP, op1 as u16);
            cpu.Z = false;
            cpu.N = false;
            cpu.HL.set(safe_w_add(cpu.SP, op1 as u16));
            3
        })),
        // To SP from HL
        0xF9 => ("LD SP, HL", 1, Box::new(|cpu, _, _, _, _| { cpu.SP = cpu.HL.val(); 2 })),
       
       /* STACK STUFF */
        0xC5 => ("PUSH BC", 1, Box::new(|cpu, s, _, _, _| { cpu.push_u16(s, cpu.BC.val()); 4 })),
        0xD5 => ("PUSH DE", 1, Box::new(|cpu, s, _, _, _| { cpu.push_u16(s, cpu.DE.val()); 4 })),
        0xE5 => ("PUSH HL", 1, Box::new(|cpu, s, _, _, _| { cpu.push_u16(s, cpu.HL.val()); 4 })),
        0xF5 => ("PUSH AF", 1, Box::new(|cpu, s, _, _, _| { cpu.push_u16(s, word(cpu.A, cpu.F())); 4 })),
        0xC1 => ("POP BC",  1, Box::new(|cpu, s, _, _, _| { let val = cpu.pop_u16(s); cpu.BC.set(val); 3 })),
        0xD1 => ("POP DE",  1, Box::new(|cpu, s, _, _, _| { let val = cpu.pop_u16(s); cpu.DE.set(val); 3 })),
        0xE1 => ("POP HL",  1, Box::new(|cpu, s, _, _, _| { let val = cpu.pop_u16(s); cpu.HL.set(val); 3 })),
        0xF1 => ("POP AF",  1, Box::new(|cpu, s, _, _, _| { 
            let (a, f) = word_split(cpu.pop_u16(s));
            cpu.set_F(f);
            cpu.A = a;
            3 
        })),

        /* 8-bit ALU */
        // Add register without carry
        0x80 | 0x81 | 0x82 | 0x83 | 0x84 | 0x85 | 0x86 | 0x87 => ("ADD A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.N = false;
            cpu.H = add_b_hcarry(cpu.A, val);
            cpu.C = add_b_carry(cpu.A, val);
            cpu.A = safe_b_add(cpu.A, val);
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // Add immediate without carry
        0xC6 => ("ADD A, d8", 2, Box::new(|cpu, _, _, val, _| {
            cpu.N = false;
            cpu.H = add_b_hcarry(cpu.A, val);
            cpu.C = add_b_carry(cpu.A, val);
            cpu.A = safe_b_add(cpu.A, val);
            cpu.Z = cpu.A == 0;
            2
        })),
        // Add register with carry
        0x88 | 0x89 | 0x8A | 0x8B | 0x8C | 0x8D | 0x8E | 0x8F => ("ADC A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            let carry = if cpu.C { 1 } else { 0 };
            // If carry happens when adding (+ val)
            cpu.H = add_b_hcarry(cpu.A, val);
            cpu.C = add_b_carry(cpu.A, val);
            cpu.A = safe_b_add(cpu.A, val);
            // If carry happens when (+ carry)
            cpu.H |= add_b_hcarry(cpu.A, carry);
            cpu.C |= add_b_carry(cpu.A, carry);
            cpu.A  = safe_b_add(cpu.A, carry);
            cpu.N = false;
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // Add immediate with carry
        0xCE => ("ADC A, d8", 2, Box::new(|cpu, _, _, val, _| {
            let carry = if cpu.C { 1 } else { 0 };
            // If carry happens when (+ val)
            cpu.H = add_b_hcarry(cpu.A, val);
            cpu.C = add_b_carry(cpu.A, val);
            cpu.A = safe_b_add(cpu.A, val);
            // If carry happens when (+ carry)
            cpu.H |= add_b_hcarry(cpu.A, carry);
            cpu.C |= add_b_carry(cpu.A, carry);
            cpu.A  = safe_b_add(cpu.A, carry);
            cpu.N = false;
            cpu.Z = cpu.A == 0;
            2
        })),
        // Sub register without carry
        0x90 | 0x91 | 0x92 | 0x93 | 0x94 | 0x95 | 0x96 | 0x97 => ("SUB A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.H = sub_b_hcarry(cpu.A, val);
            cpu.C = sub_b_carry(cpu.A, val);
            cpu.A = safe_b_sub(cpu.A, val);
            cpu.N = true;
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // Sub immediate without carry
        0xD6 => ("SUB A, d8", 2, Box::new(|cpu, _, _, val, _| {            
            cpu.H = sub_b_hcarry(cpu.A, val);
            cpu.C = sub_b_carry(cpu.A, val);
            cpu.A = safe_b_sub(cpu.A, val);
            cpu.N = true;
            cpu.Z = cpu.A == 0;
            2
        })),
        // Sub register with cary
        0x98 | 0x99 | 0x9A | 0x9B | 0x9C | 0x9D | 0x9E | 0x9F => ("SBC A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            let carry = if cpu.C { 1 } else { 0 };
            // If carry happens when (- reg)
            cpu.H = sub_b_hcarry(cpu.A, val);
            cpu.C = sub_b_carry(cpu.A, val);
            cpu.A = safe_b_sub(cpu.A, val);
            // If carry happens when (- carry)
            cpu.H |= sub_b_hcarry(cpu.A, carry);
            cpu.C |= sub_b_carry(cpu.A, carry);
            cpu.A  = safe_b_sub(cpu.A, carry);
            cpu.N = true;
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // Sub immediate with carry
        0xDE => ("SBC A, d8", 2, Box::new(|cpu, _, _, val, _| {
            let carry = if cpu.C { 1 } else { 0 };
            // If carry happens when (- reg)
            cpu.H = sub_b_hcarry(cpu.A, val);
            cpu.C = sub_b_carry(cpu.A, val);
            cpu.A = safe_b_sub(cpu.A, val);
            // If carry happens when (- carry)
            cpu.H |= sub_b_hcarry(cpu.A, carry);
            cpu.C |= sub_b_carry(cpu.A, carry);
            cpu.A  = safe_b_sub(cpu.A, carry);
            cpu.N = true;
            cpu.Z = cpu.A == 0;
            2
        })),
        // AND with register
        0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5 | 0xA6 | 0xA7 => ("AND A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);

            cpu.A &= val;
            cpu.N = false;
            cpu.H = true;
            cpu.C = false;
            cpu.Z = cpu.A == 0;

            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // AND with immediate
        0xE6 => ("AND A, d8", 2, Box::new(|cpu, _, _, val, _| {            
            cpu.A &= val;
            cpu.N = false;
            cpu.H = true;
            cpu.C = false;
            cpu.Z = cpu.A == 0;
            2
        })),
        // XOR with register
        0xA8 | 0xA9 | 0xAA | 0xAB | 0xAC | 0xAD | 0xAE | 0xAF => ("XOR A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.A ^= val;
            cpu.N = false;
            cpu.H = false;
            cpu.C = false;
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // XOR with immediate
        0xEE => ("XOR A, d8", 2, Box::new(|cpu, _, _, val, _| {            
            cpu.A ^= val;
            cpu.N = false;
            cpu.H = false;
            cpu.C = false;
            cpu.Z = cpu.A == 0;
            2
        })),
        // OR with register
        0xB0 | 0xB1 | 0xB2 | 0xB3 | 0xB4 | 0xB5 | 0xB6 | 0xB7 => ("OR A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.A |= val;
            cpu.N = false;
            cpu.H = false;
            cpu.C = false;
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // OR with immediate
        0xF6 => ("OR A, d8", 2, Box::new(|cpu, _, _, val, _| {            
            cpu.A ^= val;
            cpu.N = false;
            cpu.H = false;
            cpu.C = false;
            cpu.Z = cpu.A == 0;
            2
        })),
        // Compare regs
        0xB8 | 0xB9 | 0xBA | 0xBB | 0xBC | 0xBD | 0xBE | 0xBF => ("CP A, reg", 1, Box::new(|cpu, s, op, _, _| {
            let idx = op & 0x7;
            let val = cpu.reg(s, idx);
            cpu.N = true;
            cpu.H = sub_b_hcarry(cpu.A, val);
            cpu.C = sub_b_carry(cpu.A, val);
            cpu.Z = cpu.A == val;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // Compare with immediate
        0xFE => ("CP A, reg", 2, Box::new(|cpu, _, _, val, _| {
            cpu.N = true;
            cpu.H = sub_b_hcarry(cpu.A, val);
            cpu.C = sub_b_carry(cpu.A, val);
            cpu.Z = cpu.A == val;
            2
        })),
        // Increments regsister
        0x04 | 0x14 | 0x24 | 0x34 | 0x0C | 0x1C | 0x2C | 0x3C => ("INC reg", 1, Box::new(|cpu, _, op, _, _| {
            let (n1, n2) = (op >> 4, op & 0xF);
            let idx = 2*n1 + {if n2 == 0xC { 1 } else { 0 }};
            cpu.N = false;
            cpu.H = add_b_hcarry(cpu.A, 1);
            cpu.A = safe_b_add(cpu.A, 1);
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        // Decrements register
        0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => ("DEC reg", 1, Box::new(|cpu, _, op, _, _| {
            let (n1, n2) = (op >> 4, op & 0xF);
            let idx = 2*n1 + {if n2 == 0xC { 1 } else { 0 }};
            cpu.N = true;
            cpu.H = sub_b_hcarry(cpu.A, 1);
            cpu.A = safe_b_sub(cpu.A, 1);
            cpu.Z = cpu.A == 0;
            if idx == ADDR_HL_IDX { 2 } else { 1 }
        })),
        _ => return None,
    };
        
    Some(Instruction::new(mnemo, size, f))
}

fn decode_cb<T: BankController>(op: u8) -> Option<Instruction<'static, T>> {
    let (mnemo, f): (&str, Box<InstructionHandler<T>>) = match op {
        _ => return None,
    };

    Some(Instruction::new(mnemo, 2, f))
}


#[repr(C)]
pub union Reg {
    /* For lower and upper register bytes */
    bytes: [u8; 2],
    /* For accessing as 16 bit register */
    word: u16,
}
impl Reg {
    fn new(value: u16) -> Self { Self {word: value} }

    // It is assumed that u16 is little endian
    pub fn low(&self) -> u8  { unsafe { self.bytes[0] } }
    pub fn set_low(&mut self, value: u8) { unsafe { self.bytes[0] = value; } }

    pub fn up(&self) -> u8  { unsafe { self.bytes[1] } }
    pub fn set_up(&mut self, value: u8) { unsafe { self.bytes[1] = value; } }

    pub fn val(&self) -> u16 { unsafe { self.word } }
    pub fn set(&mut self, value: u16) { self.word = value; }
}
impl Default for Reg {
    fn default() -> Self { Self { word: 0x0000} }
}
impl fmt::Debug for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hex Value: 0x{:x}, Decimal: {}, Lower Decimal: {} Upper Decimal {} ",
               self.val(), self.val(),
               self.low(), self.up())
    }
}

#[derive(Debug)]
pub struct CPU {
    /* Main registers */
    pub A: u8,
    pub BC: Reg,
    pub DE: Reg,
    pub HL: Reg,
    pub SP: u16,
    pub PC: Reg,
    /* Members of flag register */
    pub Z: bool,
    pub N: bool,
    pub H: bool,
    pub C: bool,
    /* Other flags */
    pub IME: bool,
    pub STOP: bool,
    pub HALT: bool,
}
impl Default for CPU {
    // Default F = 0xB0 = 0b10110000 = ZHC
    fn default() -> Self { 
        Self { 
            A: 0x01,
            BC: Reg::new(0x0013),
            DE: Reg::new(0x00D8),
            HL: Reg::new(0x014D),
            SP: 0xFFFE,
            PC: Reg::new(0x0000),
            Z: true,
            N: false,
            H: true,
            C: true,
            IME: true,
            STOP: false,
            HALT: false,
        }
    }
}

/*
 * Bit 0: V-Blank  Interrupt Request (INT 40h)
 * Bit 1: LCD STAT Interrupt Request (INT 48h)
 * Bit 2: Timer    Interrupt Request (INT 50h)
 * Bit 3: Serial   Interrupt Request (INT 58h)
 * Bit 4: Joypad   Interrupt Request (INT 60h)
 */ 
const VBLANK_INT: usize = 0;
const STAT_INT: usize = 1;
const TIMER_INT: usize = 2;
const SERIAL_INT: usize = 3;
const JOYPAD_INT: usize = 4;

const IVT_SIZE: usize = 5;
const IVT: [u8; IVT_SIZE] = [0x40, 0x48, 0x50, 0x58, 0x60];

impl CPU {
    pub fn new() -> Self { Default::default() }

    // step() executes single instruction and returns number of taken machine cycles
    pub fn step<T: BankController>(&mut self, state: &mut State<T>) -> u64 {
        // If HALT or STOP set CPU executes NOPs without incrementing PC.
        if self.HALT || self.STOP { return 1 }

        let mut pc = self.PC.val();
        let mut op = state.safe_read(pc);
        let mut cb_inst_set = false;

        println!("----- PC: 0x{:X} -----", pc);

        // Opcode 0xCB is special case. It's a prefix for another set of 256 2-byte instructions.
        if op == 0xCB {
            pc += 1;
            self.PC.set(pc);
            op = state.safe_read(pc);
            cb_inst_set = true;
            println!("CB prefixed 0x{:x} from 0x{:x}", op, pc);
        } else {
            println!("Fetched 0x{:x} from 0x{:x}", op, pc);
        }

        let Instruction { size, handler: mut f, mnemo } = if cb_inst_set { decode_cb(op) } else { decode(op) }
            .unwrap_or_else(|| panic!("Unrecognized OPCODE 0x{:x} at 0x{:x}. {:?}", op, pc, self));
        
        let argc = size - 1;
        let op1 = if argc >= 1 { state.safe_read(pc+1) } else { 0 };
        let op2 = if argc >= 2 { state.safe_read(pc+2) } else { 0 };

        println!("Executing '{}' with size {}.", mnemo, size);
        
        let cycles = f(self, state, op, op1, op2) as u64;
        self.PC.set(self.PC.val() + size as u16);
        cycles
    }

    // interrupts() will check for interrupt requests and pass control to appropriate ISR(Interrupt Service Routine)
    // If HALT=true -> any enabled interrupt will reset HALT
    // If STOP=true -> only joypad interrupt will reset STOP
    // Not sure how these things work when interrupts disabled in IE.
    pub fn interrupts<T: BankController>(&mut self, state: &mut State<T>) -> u64 {
        /*
         * IME - Interrupt Master Enable Flag
         * 0 - Disable all Interrupts
         * 1 - Enable all Interrupts that are enabled in IE Register (FFFF)
         */
        if !self.IME { return 0 }

        let in_e = state.safe_read(ioregs::IE);
        let in_f = state.safe_read(ioregs::IF);
        let is_requested = |bit: usize| (in_f & (1 << bit)) & in_e != 0;

        for bit in 0..IVT_SIZE {
            // If it's stopped only JOYPAD interrupt can resume.
            if self.STOP && bit != JOYPAD_INT { continue; }

            if is_requested(bit)  {
                self.IME = false;
                self.HALT = false;
                self.STOP = false;
                state.mmu.set_bit(ioregs::IF, bit as u8, false);

                // Put PC on the stack
                state.safe_write(self.SP, self.PC.up());
                state.safe_write(self.SP - 1, self.PC.low());
                self.SP -= 2;

                // Set PC to 0x00NN
                self.PC.set_low(IVT[bit]);
                self.PC.set_up(0x00);

                // http://gbdev.gg8.se/wiki/articles/Interrupts - they say control passing to ISR should take 5 cycles
                return 5
            }
        }

        0
    }

    // Some utility methods
    fn read_HL(&self, state: &mut State<impl BankController>) -> u8 { state.safe_read(self.HL.val()) }
    fn write_HL(&self, state: &mut State<impl BankController>, val: u8) { state.safe_write(self.HL.val(), val) }

    // Gets reg value by index
    fn reg(&self, state: &mut State<impl BankController>, idx: u8) -> u8 {
        match idx {
            B_IDX => self.BC.up(),
            C_IDX => self.BC.low(),
            D_IDX => self.DE.up(),
            E_IDX => self.DE.low(),
            H_IDX => self.HL.up(),
            L_IDX => self.HL.low(),
            ADDR_HL_IDX => self.read_HL(state),
            A_IDX => self.A,
            _ => panic!("reg({}) INVALID REG INDEX: {}!. Only 0-7.", idx, idx),
        }
    }

    // Sets reg value by index
    fn reg_set(&mut self, state: &mut State<impl BankController>, idx: u8, val: u8) {
        match idx {
            B_IDX => self.BC.set_up(val),
            C_IDX => self.BC.set_low(val),
            D_IDX => self.DE.set_up(val),
            E_IDX => self.DE.set_low(val),
            H_IDX => self.HL.set_up(val),
            L_IDX => self.HL.set_low(val),
            ADDR_HL_IDX => self.write_HL(state, val),
            A_IDX => self.A = val,
            _ => panic!("reg_set({}) INVALID REG INDEX: {}!. Only 0-7.", idx, idx),
        };
    }

    // Returns flag register as byte
    pub fn F(&self) -> u8 {
        let mut f = 0u8;
        f |= if self.Z { 1 << 7 } else { 0 };
        f |= if self.N { 1 << 6 } else { 0 };
        f |= if self.H { 1 << 5 } else { 0 };
        f |= if self.C { 1 << 4 } else { 0 };
        f
    }

    // Updates flags using received byte
    pub fn set_F(&mut self, val: u8) {
        self.Z = val & (1 << 7) != 0;
        self.N = val & (1 << 6) != 0;
        self.H = val & (1 << 5) != 0;
        self.C = val & (1 << 4) != 0;
    }

    fn push_u16(&mut self, state: &mut State<impl BankController>, val: u16) {
        self.SP -= 2;
        state.write_word(self.SP, val);
    }

    fn pop_u16(&mut self, state: &mut State<impl BankController>) -> u16 {
        let val = state.read_word(self.SP);
        self.SP += 2;
        val
    }
}