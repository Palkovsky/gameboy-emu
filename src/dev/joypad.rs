#![allow(non_snake_case, non_camel_case_types)]

use super::*;

#[derive(Debug, Default)]
pub struct Joypad {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool,
    interrupt: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn step(&mut self, mmu: &mut MMU<impl BankController>) {
        // Button keys selected
        if !mmu.read_bit(ioregs::P1, 5) {
            mmu.set_bit(ioregs::P1, 0, !self.a);
            mmu.set_bit(ioregs::P1, 1, !self.b);
            mmu.set_bit(ioregs::P1, 2, !self.select);
            mmu.set_bit(ioregs::P1, 3, !self.start);
        }
        // Direction keys selected
        else if !mmu.read_bit(ioregs::P1, 4) {
            mmu.set_bit(ioregs::P1, 0, !self.right);
            mmu.set_bit(ioregs::P1, 1, !self.left);
            mmu.set_bit(ioregs::P1, 2, !self.up);
            mmu.set_bit(ioregs::P1, 3, !self.down);
        }
        // No column selected
        else {
            mmu.write(ioregs::P1, 0xFF);
        }
        if self.interrupt {
            Joypad::joypad_int(mmu);
            self.interrupt = false;
        }
    }

    pub fn down(&mut self, val: bool) {
        if val && !self.down {
            self.interrupt = true;
        }
        self.down = val;
    }

    pub fn left(&mut self, val: bool) {
        if val && !self.left {
            self.interrupt = true;
        }
        self.left = val;
    }

    pub fn right(&mut self, val: bool) {
        if val && !self.right {
            self.interrupt = true;
        }
        self.right = val;
    }

    pub fn a(&mut self, val: bool) {
        if val && !self.a {
            self.interrupt = true;
        }
        self.a = val;
    }

    pub fn b(&mut self, val: bool) {
        if val && !self.b {
            self.interrupt = true;
        }
        self.b = val;
    }

    pub fn select(&mut self, val: bool) {
        if val && !self.select {
            self.interrupt = true;
        }
        self.select = val;
    }

    pub fn start(&mut self, val: bool) {
        if val && !self.start {
            self.interrupt = true;
        }
        self.start = val;
    }

    pub fn up(&mut self, val: bool) {
        if val && !self.up {
            self.interrupt = true;
        }
        self.up = val;
    }

    fn joypad_int(mmu: &mut MMU<impl BankController>) {
        mmu.set_bit(ioregs::IF, 4, true);
    }
}
