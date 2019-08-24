#![allow(non_snake_case, non_camel_case_types)]

use super::*;
use std::fmt;

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
fn decode<T: BankController>(op: u8, op1: u8, op2: u8) -> Option<Instruction<'static, T>> {
    let nibbles = ((op >> 4) & 0xF, op & 0xF);
    
    let (mnemo, size, f): (&str, u8, Box<InstructionHandler<T>>) = match (nibbles, op1, op2) {

        // Misc/Control instructions
        ((0x0, 0x0), _, _) => ("NOP", 1, Box::new(|_, _, _, _| 1)),
        ((0x1, 0x0), _, _) => ("STOP 0", 1, Box::new(|cpu, _, _, _| {
            cpu.STOP = true;
            1
        })),
        ((0x7, 0x6), _, _) => ("HALT", 1, Box::new(|cpu, _, _, _| {
                cpu.HALT = true;
                1
        })),
        ((0xF, 0x4), _, _) => ("DI", 1, Box::new(|cpu, _, _, _| {
                cpu.IME = false;
                1
        })),
        ((0xF, 0xB), _, _) => ("EI", 1, Box::new(|cpu, _, _, _| {
                cpu.IME = true;
                1
        })),
        _ => return None,
    };
        
    Some(Instruction::new(mnemo, size, f))
}


#[repr(C)]
union Reg {
    /* For lower and upper register bytes */
    bytes: [u8; 2],
    /* For accessing as 16 bit register */
    word: u16,
}
impl Reg {
    fn new(value: u16) -> Self { Self {word: value} }

    // It is assumed that u16 is little endian
    fn low(&self) -> u8  { unsafe { self.bytes[0] } }
    fn set_low(&mut self, value: u8) { unsafe { self.bytes[0] = value; } }

    fn up(&self) -> u8  { unsafe { self.bytes[1] } }
    fn set_up(&mut self, value: u8) { unsafe { self.bytes[1] = value; } }

    fn val(&self) -> u16 { unsafe { self.word } }
    fn set(&mut self, value: u16) { self.word = value; }
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
    A: u8,
    BC: Reg,
    DE: Reg,
    HL: Reg,
    SP: u16,
    PC: Reg,
    /* Members of flag register */
    Z: bool,
    N: bool,
    H: bool,
    C: bool,
    /* Other flags */
    IME: bool,
    STOP: bool,
    HALT: bool,
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
const IVT_SIZE: usize = 5;
const IVT: [u8; IVT_SIZE] = [0x40, 0x48, 0x50, 0x58, 0x60];

impl CPU {
    pub fn new() -> Self { Default::default() }

    // step() executes single instruction and returns number of taken machine cycles
    pub fn step<T: BankController>(&mut self, state: &mut State<T>) -> u64 {
        // If HALT set CPU executes NOPs without incrementing PC.
        if self.HALT { return 1 }

        let pc = self.PC.val();

        // No instruction longer than 3 bytes on this CPU
        let (op, op1, op2) = (state.safe_read(pc), state.safe_read(pc+1), state.safe_read(pc+2));
        let Instruction { size, handler: mut f, ..} = decode(op, op1, op2)
            .unwrap_or_else(|| panic!("Unrecognized OPCODE 0x{:x} at 0x{:x}. {:?}", op, pc, self));
        let argc = size - 1;
        let cycles = f(self, state, if argc >= 1 { op1 } else { 0 }, if argc >= 2 { op2 } else { 0 }) as u64;
        
        self.PC.set(self.PC.val() + size as u16);
        cycles
    }

    // interrupts() will check for interrupt requests and pass control to appropriate ISR(Interrupt Service Routine)
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
            if is_requested(bit) {
                state.mmu.set_bit(ioregs::IF, bit as u8, false);
                self.IME = false;

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
}