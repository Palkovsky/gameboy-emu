use super::{IO_REGS_ADDR, MutMem, Byte};

pub const P1: u16 = 0xFF00;
pub const SB: u16 = 0xFF01;
pub const SC: u16 = 0xFF02;
pub const DIV: u16 = 0xFF04;
pub const TIMA: u16 = 0xFF05;
pub const TMA: u16 = 0xFF06;
pub const TAC: u16 = 0xFF07;
pub const IF: u16 = 0xFF0F;
pub const NR_10: u16 = 0xFF10;
pub const NR_11: u16 = 0xFF11;
pub const NR_12: u16 = 0xFF12;
pub const NR_13: u16 = 0xFF13;
pub const NR_14: u16 = 0xFF14;
pub const NR_21: u16 = 0xFF16;
pub const NR_22: u16 = 0xFF17;
pub const NR_23: u16 = 0xFF18;
pub const NR_24: u16 = 0xFF19;
pub const NR_30: u16 = 0xFF1A;
pub const NR_31: u16 = 0xFF1B;
pub const NR_32: u16 = 0xFF1C;
pub const NR_33: u16 = 0xFF1D;
pub const NR_34: u16 = 0xFF1E;
pub const NR_41: u16 = 0xFF20;
pub const NR_42: u16 = 0xFF21;
pub const NR_43: u16 = 0xFF22;
pub const NR_44: u16 = 0xFF23;
pub const NR_50: u16 = 0xFF24;
pub const NR_51: u16 = 0xFF25;
pub const NR_52: u16 = 0xFF26;
/* WAVE PATTERN FROM 0xFF30-0xFF3F */
pub const LCDC: u16 = 0xFF40;
pub const STAT: u16 = 0xFF41;
pub const SCY: u16 = 0xFF42;
pub const SCX: u16 = 0xFF43;
pub const LY: u16 = 0xFF44;
pub const LYC: u16 = 0xFF45;
pub const DMA: u16 = 0xFF46;
pub const BGP: u16 = 0xFF47;
pub const OBP_0: u16 = 0xFF48;
pub const OBP_1: u16 = 0xFF49;
pub const WY: u16 = 0xFF4A;
pub const WX: u16 = 0xFF4B;
pub const BOOT_END: u16 = 0xFF50;
pub const IE: u16 = 0xFFFF;

pub struct IORegs {
    regs: Vec<Byte>,
}

impl IORegs {
    pub fn new() -> Self {
        let mut res = Self { regs: vec![0u8; 0x100] };

        // Set default non-zero values
        res.set(NR_10, 0x80);
        res.set(NR_11, 0xBF);
        res.set(NR_12, 0xF3);
        res.set(NR_14, 0xBF);
        res.set(NR_21, 0x3F);
        res.set(NR_24, 0xBF);
        res.set(NR_30, 0x7F);
        res.set(NR_31, 0xFF);
        res.set(NR_32, 0x9F);
        res.set(NR_33, 0xBF);
        res.set(NR_41, 0xFF);
        res.set(NR_44, 0xBF);
        res.set(NR_50, 0x77);
        res.set(NR_51, 0xF3);
        res.set(NR_52, 0xF1); // 0xF0 in SGB
        res.set(LCDC, 0x91);
        res.set(BGP, 0xFC);
        res.set(OBP_0, 0xFF);
        res.set(OBP_1, 0xFF);

        res
    }

    pub fn slice(&mut self) -> MutMem { 
        &mut self.regs[..]
    }

    pub fn set(&mut self, addr: u16, value: Byte) {
        self.regs[(addr - IO_REGS_ADDR) as usize] = value;
    }

    pub fn get(&self, addr: u16) -> Byte {
        self.regs[(addr - IO_REGS_ADDR) as usize]
    }
}
