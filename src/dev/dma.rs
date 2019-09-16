#![allow(non_snake_case, non_camel_case_types)]

use super::*;

const TRANSFER_SIZE: usize = 140;

pub struct DMA {
    active: bool,
    buff: [u8; TRANSFER_SIZE],
}

impl<T: BankController> Clocked<T> for DMA {
    fn next_time(&self, _: &mut MMU<T>) -> u64 {
        if self.active {
            162
        } else {
            1
        }
    }

    fn step(&mut self, mmu: &mut MMU<T>) {
        if !self.active {
            return;
        }
        let addr = DMA::FROM(mmu);
        //println!("Started DMA transfer from 0x{:x} to OAM.", addr);
        for i in 0..TRANSFER_SIZE {
            self.buff[i] = mmu.read(addr + i as u16);
        }
        let dest = &mut mmu.oam[..];
        for i in 0..TRANSFER_SIZE {
            dest[i] = self.buff[i];
        }
        println!("DMA finished!!!!!!!!!!");
        self.active = false;
    }
}

impl DMA {
    pub fn new() -> Self {
        Self {
            active: false,
            buff: [0; TRANSFER_SIZE],
        }
    }
    pub fn start(&mut self) {
        self.active = true;
    }
    pub fn active(&self) -> bool {
        self.active
    }
    fn FROM(mmu: &mut MMU<impl BankController>) -> u16 {
        (mmu.read(ioregs::DMA) as u16) << 8
    }
}
