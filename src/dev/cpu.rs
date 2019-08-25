#![allow(non_snake_case, non_camel_case_types)]

use super::*;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

/* InstructionHandler takes CPU reference for register updates and 2 instruction operands as arguments. 
 * When instruction length is less than 3 the redundant bytes should be ignored. 
 * Handler returns number of machine cycles consumed. Hardcoding cycles wouldn't, because 
 * conditional jumps/calls take varying number of cycles.
 */
type InstructionHandler<T: BankController> = FnMut(&mut CPU, &mut State<T>, u8, u8) -> u8;

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

/*
 * Decoder for Gameboy CPU (LR35902) instruction set 
 */
fn decode<T: BankController>(op: u8) -> Option<Instruction<'static, T>> {
    let nibbles = ((op >> 4) & 0xF, op & 0xF);
    
    let (mnemo, size, f): (&str, u8, Box<InstructionHandler<T>>) = match nibbles {
        /* Misc/Control instructions */
        (0x0, 0x0) => ("NOP",    1, Box::new(|_, _, _, _| 1)),
        (0x1, 0x0) => ("STOP 0", 1, Box::new(|cpu, _, _, _| { cpu.STOP = true; 1 })),
        (0x7, 0x6) => ("HALT",   1, Box::new(|cpu, _, _, _| { cpu.HALT = true; 1 })),
        (0xF, 0x4) => ("DI",     1, Box::new(|cpu, _, _, _| { cpu.IME = false; 1 })),
        (0xF, 0xB) => ("EI",     1, Box::new(|cpu, _, _, _| { cpu.IME = true; 1 })),

        /* 8bit load/store/move instructions */
        // To B register
        (0x4, 0x0) => ("LD B, B",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_up(cpu.BC.up()); 1 })),
        (0x4, 0x1) => ("LD B, C",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_up(cpu.BC.low()); 1 })),
        (0x4, 0x2) => ("LD B, D",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_up(cpu.DE.up()); 1 })),
        (0x4, 0x3) => ("LD B, E",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_up(cpu.DE.low()); 1 })),
        (0x4, 0x4) => ("LD B, H",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_up(cpu.HL.up()); 1 })),
        (0x4, 0x5) => ("LD B, L",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_up(cpu.HL.up()); 1 })),
        (0x4, 0x6) => ("LD B, (HL)", 1, Box::new(|cpu, s, _, _| { cpu.BC.set_up(cpu.read_HL(s)); 2 })),
        (0x4, 0x7) => ("LD B, A",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_up(cpu.A); 1 })),
        // To C register
        (0x4, 0x8) => ("LD C, B",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_low(cpu.BC.up()); 1 })),
        (0x4, 0x9) => ("LD C, C",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_low(cpu.BC.low()); 1 })),
        (0x4, 0xA) => ("LD C, D",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_low(cpu.DE.up()); 1 })),
        (0x4, 0xB) => ("LD C, E",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_low(cpu.DE.low()); 1 })),
        (0x4, 0xC) => ("LD C, H",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_low(cpu.HL.up()); 1 })),
        (0x4, 0xD) => ("LD C, L",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_low(cpu.HL.up()); 1 })),
        (0x4, 0xE) => ("LD C, (HL)", 1, Box::new(|cpu, s, _, _| { cpu.BC.set_low(cpu.read_HL(s)); 2 })),
        (0x4, 0xF) => ("LD C, A",    1, Box::new(|cpu, _, _, _| { cpu.BC.set_low(cpu.A); 1 })),
        // To D register
        (0x5, 0x0) => ("LD D, B",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_up(cpu.BC.up()); 1 })),
        (0x5, 0x1) => ("LD D, C",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_up(cpu.BC.low()); 1 })),
        (0x5, 0x2) => ("LD D, D",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_up(cpu.DE.up()); 1 })),
        (0x5, 0x3) => ("LD D, E",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_up(cpu.DE.low()); 1 })),
        (0x5, 0x4) => ("LD D, H",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_up(cpu.HL.up()); 1 })),
        (0x5, 0x5) => ("LD D, L",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_up(cpu.HL.up()); 1 })),
        (0x5, 0x6) => ("LD D, (HL)", 1, Box::new(|cpu, s, _, _| { cpu.DE.set_up(cpu.read_HL(s)); 2 })),
        (0x5, 0x7) => ("LD D, A",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_up(cpu.A); 1 })),
        // To E register
        (0x5, 0x8) => ("LD E, B",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_low(cpu.BC.up()); 1 })),
        (0x5, 0x9) => ("LD E, C",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_low(cpu.BC.low()); 1 })),
        (0x5, 0xA) => ("LD E, D",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_low(cpu.DE.up()); 1 })),
        (0x5, 0xB) => ("LD E, E",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_low(cpu.DE.low()); 1 })),
        (0x5, 0xC) => ("LD E, H",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_low(cpu.HL.up()); 1 })),
        (0x5, 0xD) => ("LD E, L",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_low(cpu.HL.up()); 1 })),
        (0x5, 0xE) => ("LD E, (HL)", 1, Box::new(|cpu, s, _, _| { cpu.DE.set_low(cpu.read_HL(s)); 2 })),
        (0x5, 0xF) => ("LD E, A",    1, Box::new(|cpu, _, _, _| { cpu.DE.set_low(cpu.A); 1 })),
        // To H register
        (0x6, 0x0) => ("LD H, B",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_up(cpu.BC.up()); 1 })),
        (0x6, 0x1) => ("LD H, C",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_up(cpu.BC.low()); 1 })),
        (0x6, 0x2) => ("LD H, D",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_up(cpu.DE.up()); 1 })),
        (0x6, 0x3) => ("LD H, E",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_up(cpu.DE.low()); 1 })),
        (0x6, 0x4) => ("LD H, H",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_up(cpu.HL.up()); 1 })),
        (0x6, 0x5) => ("LD H, L",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_up(cpu.HL.up()); 1 })),
        (0x6, 0x6) => ("LD H, (HL)", 1, Box::new(|cpu, s, _, _| { cpu.HL.set_up(cpu.read_HL(s)); 2 })),
        (0x6, 0x7) => ("LD H, A",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_up(cpu.A); 1 })),
        // To L register
        (0x6, 0x8) => ("LD L, B",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_low(cpu.BC.up()); 1 })),
        (0x6, 0x9) => ("LD L, C",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_low(cpu.BC.low()); 1 })),
        (0x6, 0xA) => ("LD L, D",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_low(cpu.DE.up()); 1 })),
        (0x6, 0xB) => ("LD L, E",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_low(cpu.DE.low()); 1 })),
        (0x6, 0xC) => ("LD L, H",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_low(cpu.HL.up()); 1 })),
        (0x6, 0xD) => ("LD L, L",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_low(cpu.HL.up()); 1 })),
        (0x6, 0xE) => ("LD L, (HL)", 1, Box::new(|cpu, s, _, _| { cpu.HL.set_low(cpu.read_HL(s)); 2 })),
        (0x6, 0xF) => ("LD L, A",    1, Box::new(|cpu, _, _, _| { cpu.HL.set_low(cpu.A); 1 })),
        // To (HL) from register
        (0x7, 0x0) => ("LD (HL), B",    1, Box::new(|cpu, s, _, _| { cpu.write_HL(s, cpu.BC.up()); 2 })),
        (0x7, 0x1) => ("LD (HL), C",    1, Box::new(|cpu, s, _, _| { cpu.write_HL(s, cpu.BC.low()); 2 })),
        (0x7, 0x2) => ("LD (HL), D",    1, Box::new(|cpu, s, _, _| { cpu.write_HL(s, cpu.DE.up()); 2 })),
        (0x7, 0x3) => ("LD (HL), E",    1, Box::new(|cpu, s, _, _| { cpu.write_HL(s, cpu.DE.low()); 2 })),
        (0x7, 0x4) => ("LD (HL), H",    1, Box::new(|cpu, s, _, _| { cpu.write_HL(s, cpu.HL.up()); 2 })),
        (0x7, 0x5) => ("LD (HL), L",    1, Box::new(|cpu, s, _, _| { cpu.write_HL(s, cpu.HL.low()); 2 })),
        // 0x76 is HALT
        (0x7, 0x7) => ("LD (HL), A",    1, Box::new(|cpu, s, _, _| { cpu.write_HL(s, cpu.A); 2 })),
        // To A register
        (0x7, 0x8) => ("LD A, B",    1, Box::new(|cpu, _, _, _| { cpu.A = cpu.BC.up(); 1 })),
        (0x7, 0x9) => ("LD A, C",    1, Box::new(|cpu, _, _, _| { cpu.A = cpu.BC.low(); 1 })),
        (0x7, 0xA) => ("LD A, D",    1, Box::new(|cpu, _, _, _| { cpu.A = cpu.DE.up(); 1 })),
        (0x7, 0xB) => ("LD A, E",    1, Box::new(|cpu, _, _, _| { cpu.A = cpu.DE.low(); 1 })),
        (0x7, 0xC) => ("LD A, H",    1, Box::new(|cpu, _, _, _| { cpu.A = cpu.HL.up(); 1 })),
        (0x7, 0xD) => ("LD A, L",    1, Box::new(|cpu, _, _, _| { cpu.A = cpu.HL.low(); 1 })),
        (0x7, 0xE) => ("LD A, (HL)", 1, Box::new(|cpu, s, _, _| { cpu.A = cpu.read_HL(s); 2 })),
        (0x7, 0xF) => ("LD A, A",    1, Box::new(|cpu, _, _, _| { cpu.A = cpu.A; 1 })),
        // To (BC) from A
        (0x0, 0x2) => ("LD (BC), A",    1, Box::new(|cpu, s, _, _| { s.safe_write(cpu.BC.val(), cpu.A); 2 })),
        // To (DE) from A
        (0x1, 0x2) => ("LD (DE), A",    1, Box::new(|cpu, s, _, _| { s.safe_write(cpu.DE.val(), cpu.A); 2 })),
        // To (HL) from A with post-increment
        (0x2, 0x2) => ("LD (HL+), A",   1, Box::new(|cpu, s, _, _| { 
            s.safe_write(cpu.HL.val(), cpu.A); 
            cpu.HL.set(cpu.HL.val() + 1); // Wrapping!
            2 
        })),
        // To (HL) from A with pre-decrement
        (0x3, 0x2) => ("LD (HL-), A",    1, Box::new(|cpu, s, _, _| { 
            cpu.HL.set(cpu.HL.val() - 1); // Wrapping!
            s.safe_write(cpu.HL.val(), cpu.A); 
            2 
        })),
        // To A from (BC)
        (0x0, 0xA) => ("LD A, (BC)",    1, Box::new(|cpu, s, _, _| { cpu.A = s.safe_read(cpu.BC.val()); 2 })),
        // To A from (DE)
        (0x1, 0xA) => ("LD A, (DE)",    1, Box::new(|cpu, s, _, _| { cpu.A = s.safe_read(cpu.DE.val()); 2 })),
        // To A from (HL) with post-increment
        (0x2, 0xA) => ("LD A, (HL+)",   1, Box::new(|cpu, s, _, _| { 
            cpu.A = s.safe_read(cpu.HL.val()); 
            cpu.HL.set(cpu.HL.val() + 1); // Wrapping!
            2 
        })),
        // To A from (HL) with pre-decrement
        (0x3, 0xA) => ("LD A, (HL-)",   1, Box::new(|cpu, s, _, _| { 
            cpu.HL.set(cpu.HL.val() - 1); // Wrapping!
            cpu.A = s.safe_read(cpu.HL.val()); 
            2 
        })),
        // To B from d8
        (0x0, 0x6) => ("LD B, d8",    2, Box::new(|cpu, _, op1, _| { cpu.BC.set_up(op1); 2 })),
        // To D from d8
        (0x1, 0x6) => ("LD D, d8",    2, Box::new(|cpu, _, op1, _| { cpu.DE.set_up(op1); 2 })),
        // To H from d8
        (0x2, 0x6) => ("LD H, d8",    2, Box::new(|cpu, _, op1, _| { cpu.HL.set_up(op1); 2 })),
        // To (HL) from d8
        (0x3, 0x6) => ("LD (HL), d8", 2, Box::new(|cpu, s, op1, _| { cpu.write_HL(s, op1); 3})),
        // To C from d8
        (0x0, 0xE) => ("LD C, d8",    2, Box::new(|cpu, _, op1, _| { cpu.BC.set_low(op1); 2 })),
        // To E from d8
        (0x1, 0xE) => ("LD E, d8",    2, Box::new(|cpu, _, op1, _| { cpu.DE.set_low(op1); 2 })),
        // To L from d8
        (0x2, 0xE) => ("LD L, d8",    2, Box::new(|cpu, _, op1, _| { cpu.HL.set_low(op1); 2 })),
        // To A from d8
        (0x3, 0xE) => ("LD A, d8",    2, Box::new(|cpu, s, op1, _| { cpu.A = op1; 2})),

        _ => return None,
    };
        
    Some(Instruction::new(mnemo, size, f))
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

        let pc = self.PC.val();

        // No instruction longer than 3 bytes on this CPU
        let op = state.safe_read(pc);
        let Instruction { size, handler: mut f, ..} = decode(op)
            .unwrap_or_else(|| panic!("Unrecognized OPCODE 0x{:x} at 0x{:x}. {:?}", op, pc, self));
        let argc = size - 1;

        let op1 = if argc >= 1 { state.safe_read(pc+1) } else { 0 };
        let op2 = if argc >= 2 { state.safe_read(pc+2) } else { 0 };
        let cycles = f(self, state, op1, op2) as u64;
        
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
    fn read_HL<T: BankController>(&self, state: &mut State<T>) -> u8 { state.safe_read(self.HL.val()) }
    fn write_HL<T: BankController>(&self, state: &mut State<T>, val: u8) { state.safe_write(self.HL.val(), val) }
}