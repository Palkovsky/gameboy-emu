#![allow(non_snake_case, non_camel_case_types)]

use super::*;

// 262144 Hz,    65536 Hz,     16384 Hz,      4096 Hz  freq
// 4             16            64             256      1MHz/freq
#[derive(Debug, PartialEq)]
pub enum TimerMode {
    FQ_4096HZ,
    FQ_16384HZ,
    FQ_65536HZ,
    FQ_262144HZ,
}
pub const STEPS_4096HZ: u64 = 256;
pub const STEPS_16384HZ: u64 = 64;
pub const STEPS_65536HZ: u64 = 16;
pub const STEPS_262144HZ: u64 = 4;

pub struct Timer {
    div_cycle: u64,
    tima_cycle: u64,
}

impl<T: BankController> Clocked<T> for Timer {
    // The timer clock is much slower than main 1MHz clock.
    // It means that timer does fraction of work per one machine cycle.
    // ie. timer with 65536Hz clock will increment by 1/16 per one machine cycle
    // for this reason next_time() returns 1, because Timer cannot overrun CPU
    fn next_time(&self, _: &mut MMU<T>) -> u64 {
        1
    }

    fn step(&mut self, mmu: &mut MMU<T>) {
        // DIV is clocked by 16384Hz clock
        if self.div_cycle % STEPS_16384HZ == 0 {
            let div = Timer::DIV(mmu);
            let new = if div == 0xFF { 0 } else { div + 1 };
            Timer::_DIV(mmu, new);
            self.div_cycle = 0;
        }
        self.div_cycle += 1;

        if !Timer::ENABLED(mmu) {
            return;
        };

        let mode = Timer::MODE(mmu);
        let mut check_ticks = |steps: u64| {
            // If not enough cycles passed
            if self.tima_cycle % steps != 0 {
                return;
            }

            let count = Timer::TIMA(mmu);
            if count == 0xFF {
                // Trigger timer interrupt
                Timer::timer_int(mmu);
                // Reload TIMA with TMA
                let tma = Timer::TMA(mmu);
                Timer::_TIMA(mmu, tma);
            } else {
                Timer::_TIMA(mmu, count + 1);
            }

            self.tima_cycle = 0;
        };

        match mode {
            TimerMode::FQ_16384HZ => check_ticks(STEPS_16384HZ),
            TimerMode::FQ_65536HZ => check_ticks(STEPS_65536HZ),
            TimerMode::FQ_262144HZ => check_ticks(STEPS_262144HZ),
            TimerMode::FQ_4096HZ => check_ticks(STEPS_4096HZ),
        };
        self.tima_cycle += 1;
    }
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div_cycle: 0,
            tima_cycle: 0
        }
    }

    fn timer_int<T: BankController>(mmu: &mut MMU<T>) {
        mmu.set_bit(ioregs::IF, 2, true);
    }

    pub fn div<T: BankController>(&mut self, mmu: &mut MMU<T>, _: u8) {
        mmu.write(ioregs::DIV, 0);
    }

    pub fn tima<T: BankController>(&mut self, mmu: &mut MMU<T>, val: u8) {
        mmu.write(ioregs::TIMA, val);
    }

    pub fn DIV<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::DIV)
    }
    pub fn TIMA<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::TIMA)
    }
    pub fn TMA<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::TMA)
    }

    fn _DIV<T: BankController>(mmu: &mut MMU<T>, val: u8) {
        mmu.write(ioregs::DIV, val);
    }
    fn _TIMA<T: BankController>(mmu: &mut MMU<T>, val: u8) {
        mmu.write(ioregs::TIMA, val);
    }
    pub fn _TMA<T: BankController>(mmu: &mut MMU<T>, val: u8) {
        mmu.write(ioregs::TMA, val);
    }

    pub fn ENABLED<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::TAC, 2)
    }
    pub fn _ENABLED<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::TAC, 2, flg);
    }

    /*
        Bits 1+0 - Input Clock Select
        00: 4.096 KHz (~4.194 KHz SGB)
        01: 262.144 Khz (~268.4 KHz SGB)
        10: 65.536 KHz (~67.11 KHz SGB)
        11: 16.384 KHz (~16.78 KHz SGB)
    */
    pub fn MODE<T: BankController>(mmu: &mut MMU<T>) -> TimerMode {
        match (mmu.read_bit(ioregs::TAC, 1), mmu.read_bit(ioregs::TAC, 0)) {
            (true, true) => TimerMode::FQ_16384HZ,   // 11
            (true, false) => TimerMode::FQ_65536HZ,  // 10
            (false, true) => TimerMode::FQ_262144HZ, // 01
            (false, false) => TimerMode::FQ_4096HZ,  // 00
        }
    }

    pub fn _MODE<T: BankController>(mmu: &mut MMU<T>, mode: TimerMode) {
        match mode {
            TimerMode::FQ_16384HZ =>
            // 11
            {
                mmu.set_bit(ioregs::TAC, 1, true);
                mmu.set_bit(ioregs::TAC, 0, true);
            }
            TimerMode::FQ_65536HZ =>
            // 10
            {
                mmu.set_bit(ioregs::TAC, 1, true);
                mmu.set_bit(ioregs::TAC, 0, false);
            }
            TimerMode::FQ_262144HZ =>
            // 01
            {
                mmu.set_bit(ioregs::TAC, 1, false);
                mmu.set_bit(ioregs::TAC, 0, true);
            }
            TimerMode::FQ_4096HZ =>
            // 00
            {
                mmu.set_bit(ioregs::TAC, 1, false);
                mmu.set_bit(ioregs::TAC, 0, false);
            }
        }
    }
}
