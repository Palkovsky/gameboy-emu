#![allow(non_snake_case, non_camel_case_types)]

use super::*;

const CPU_FREQUENCY: u32 = 1 << 20;
const SEQUENCER_FREQUENCY: u32 = 512;
const SEQUENCER_UPDATE_RATE: u16 = (CPU_FREQUENCY/SEQUENCER_FREQUENCY) as u16;
const SEQUENCER_STEP_COUNT: u16 = 8;
const DUTY_CYCLE_COUNT: u16 = 4;
const DUTY_CYCLE_STEPS: u16 = 8;
pub const BUFF_SIZE: usize = 1024;
pub const PLAYBACK_FREQUENCY: u32 = 48000;
const SAMPLE_APPEND_RATE: u16 = (CPU_FREQUENCY/PLAYBACK_FREQUENCY) as u16;

const DUTY_CYCLES: [[bool; DUTY_CYCLE_STEPS as usize]; DUTY_CYCLE_COUNT as usize] = [
    [false, true, true, true, true, true, true, true], // 12.5%
    [false, false, true, true, true, true, true, true], // 25%
    [false, false, false, false, true, true, true, true], // 50%
    [false, false, false, false, false, false, true, true], // 75%
];

struct Chan1 {
    /* frequency with sweep function transforms */
    frequency: u16,
    /* volume with envelope function transforms */
    volume: u16,
    /* Decremented by frame sequencer. 256Hz */
    length: u16,
    /* Initialized with (2048-frequency). Decremented in each CPU cycle. If 0 reached, increment duty cycle. */
    timer: u16,
    /* 8 duty cycles. Wraps when over 7. */
    duty_cycle: u16,
    /* sweep timer */
    sweep_timer: u16,
    envelope_count: u8,
    /* Output buffer for samples. */
    buff: [u16; BUFF_SIZE],
    out_buff: Option<[u16; BUFF_SIZE]>,
    /* Index of next position to write. */
    buff_idx: usize,
    /* Used to fillup buffer for player with PLAYBACK_FREQUENCY sampling rate, not CPU_FREQUENCY */
    sample_counter: u16,
}

impl Chan1 {
    fn new(mmu: &mut MMU<impl BankController>) -> Self { 
        Self {
            frequency:  Chan1::FREQ(mmu),
            volume:     Chan1::INITIAL_VOLUME(mmu),
            length:     Chan1::SOUND_LENGTH(mmu),
            timer:      2048 - Chan1::FREQ(mmu),
            duty_cycle: 0,
            sweep_timer: Chan1::SWEEP_TIME(mmu),
            envelope_count: Chan1::ENVELOPE_SHIFTS(mmu),
            buff:       [0; BUFF_SIZE],
            out_buff:   None,
            buff_idx:   0,
            sample_counter: 0,
        }
    }

    fn reset(&mut self, mmu: &mut MMU<impl BankController>) {
        self.frequency = Chan1::FREQ(mmu);
        self.volume = Chan1::INITIAL_VOLUME(mmu);
        self.length = Chan1::SOUND_LENGTH(mmu);
        self.timer = 2048 - self.frequency;
        self.duty_cycle = 0;
        self.sweep_timer = Chan1::SWEEP_TIME(mmu);
        self.envelope_count = Chan1::ENVELOPE_SHIFTS(mmu);
        self.buff = [0; BUFF_SIZE];
        self.out_buff = None;
        self.buff_idx = 0;
        self.sample_counter = 0;
    }

    fn tick(&mut self, mmu: &mut MMU<impl BankController>) {
        // If triggered start.
        if Chan1::INITIAL(mmu) { 
            self.reset(mmu);
            Chan1::_INITIAL(mmu, false);
            Chan1::_ENABLED(mmu, true);
        }
        if !Chan1::ENABLED(mmu) { return }
        // Update timer and duty cycle
        if self.timer > 0 { self.timer -= 1 };
        if self.timer == 0 {
            self.duty_cycle = (self.duty_cycle + 1) % DUTY_CYCLE_STEPS;
            self.timer = 2048 - self.frequency;
        }
        // Generate sample
        self.sample_counter += 1;
        if self.sample_counter == SAMPLE_APPEND_RATE {
            let is_on = DUTY_CYCLES[Chan1::WAVE_DUTY(mmu) as usize][self.duty_cycle as usize];
            self.buff[self.buff_idx] = if is_on && self.volume > 0 { (1<<16 - 1)/self.volume } else { 0 };
            // When temporary buffer filled, flush to output buff.
            self.buff_idx += 1;
            if self.buff_idx == BUFF_SIZE {
                self.out_buff = Some(self.buff);
                self.buff_idx = 0;
            }
            self.sample_counter = 0;
        }
    }

    pub fn buffer(&mut self) -> Option<[u16; BUFF_SIZE]> { 
        let res = self.out_buff; 
        self.out_buff = None;
        res
    }

    fn length(&mut self, mmu: &mut MMU<impl BankController>) {
        if !Chan1::ENABLED(mmu) { return }
        if self.length > 0 { self.length -= 1; }
        if self.length == 0 {
            // Disable
            self.reset(mmu);
            if Chan1::COUNTER_CONSECUTIVE_SELECT(mmu) { Chan1::_ENABLED(mmu, false); }
        }
    }
    fn sweep(&mut self, mmu: &mut MMU<impl BankController>) {
        if !Chan1::ENABLED(mmu) { return }

        self.sweep_timer -= 1;
        if self.sweep_timer == 0 {
            let delta = self.frequency/(2 as u16).pow(Chan1::SWEEP_SHIFTS(mmu) as u32);
            if Chan1::SWEEP_DIRECTION(mmu) { 
                if delta >= self.frequency { self.frequency -= delta; }
            } else if self.frequency + delta > 0x7FF { 
                Chan1::_ENABLED(mmu, false);
            } else {
                self.frequency += delta;
            }
            self.sweep_timer = Chan1::SWEEP_TIME(mmu);
        }
    }
    fn envelope(&mut self, mmu: &mut MMU<impl BankController>) {
        if !Chan1::ENABLED(mmu) || self.envelope_count == 0 { return }
        if Chan1::ENVELOPE_DIRECTION(mmu) {
            if self.volume < 0xF { self.volume += 1 };
        } else {
            if self.volume > 0   { self.volume -= 1 };
        }
        self.envelope_count -= 1;
    }

    // NR 10 - Sweep register
    fn SWEEP_TIME(mmu: &mut MMU<impl BankController>) -> u16          { (mmu.read(ioregs::NR_10) >> 4) as u16}
    fn SWEEP_SHIFTS(mmu: &mut MMU<impl BankController>) -> u8        { mmu.read(ioregs::NR_10) & 7 }
    // true = subtraction, false = addition
    fn SWEEP_DIRECTION(mmu: &mut MMU<impl BankController>) -> bool   { mmu.read(ioregs::NR_10) & 8 != 0 }

    // NR 11 - Length and wave duty registers
    fn SOUND_LENGTH(mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_11) & 0x3F) as u16 }
    fn WAVE_DUTY(mmu: &mut MMU<impl BankController>) -> u8    { mmu.read(ioregs::NR_11) >> 6 }

    // NR 12 - Volume Envelope register
    fn ENVELOPE_SHIFTS(mmu: &mut MMU<impl BankController>) -> u8       { mmu.read(ioregs::NR_12) & 7 }

    // true = increase, true = decrease
    fn ENVELOPE_DIRECTION(mmu: &mut MMU<impl BankController>) -> bool  { mmu.read(ioregs::NR_12) & 8 != 0 }
    fn INITIAL_VOLUME(mmu: &mut MMU<impl BankController>) -> u16       { (mmu.read(ioregs::NR_12) >> 4)  as u16 }

    // NR13 and NR14 - frequency
    fn FREQ(mmu: &mut MMU<impl BankController>) -> u16 {
        (((mmu.read(ioregs::NR_14) & 7) as u16) << 8) + mmu.read(ioregs::NR_13) as u16
    }
    // NR 14 - Counter/Consecutive selection and initial flags
    fn COUNTER_CONSECUTIVE_SELECT(mmu: &mut MMU<impl BankController>) -> bool { mmu.read(ioregs::NR_14) & 0x40 != 0 }
    fn INITIAL(mmu: &mut MMU<impl BankController>) -> bool { mmu.read(ioregs::NR_14) & 0x80 != 0}
    fn _INITIAL(mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_14, 7, value) }

    // NR52 - Sound ON/OFF
    fn ENABLED(mmu: &mut MMU<impl BankController>) -> bool { mmu.read(ioregs::NR_52) & 1 != 0 }
    fn _ENABLED(mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_52, 0, value) }
}

pub struct APU {
    /* If sequencer_cycle % (1MHz/512Hz) == 0 then sequencer_step increments */
    sequencer_cycle: u16,
    /* Number between 0-7. It wraps around. */
    sequencer_step: u16,
    /* Quadrangular wave patterns with sweep and envelope functions. */
    chan1: Chan1,
}

impl <T: BankController>Clocked<T> for APU {
    
    // Can always catchup
    fn next_time(&self, _: &mut MMU<T>) -> u64 { 1 }

    fn step(&mut self, mmu: &mut MMU<T>) { 
        self.chan1.tick(mmu);
        self.sequencer_cycle += 1;
        if self.sequencer_cycle == SEQUENCER_UPDATE_RATE {
            match self.sequencer_step { 0 | 2 | 4 | 6 => {
                    self.chan1.length(mmu);
                }, _ => {},
            };
            match self.sequencer_step { 2 | 6 => {
                    self.chan1.sweep(mmu);
                }, _ => {},
            };
            match self.sequencer_step { 7 => {
                    self.chan1.envelope(mmu);
                }, _ => {},
            };

            self.sequencer_cycle = 0;
            self.sequencer_step += (self.sequencer_step + 1) % SEQUENCER_STEP_COUNT;
        }
    }
}

impl APU {
    pub fn new(mmu: &mut MMU<impl BankController>) -> Self {
        Self {
            sequencer_cycle: 0,
            sequencer_step: 0,
            chan1: Chan1::new(mmu),
        }
    }

    pub fn chan1_samples(&mut self) -> Option<[u16; BUFF_SIZE]> { self.chan1.buffer() }

    pub fn chan1_reset(&mut self, mmu: &mut MMU<impl BankController>) { self.chan1.reset(mmu); }
    pub fn chan2_reset(&mut self, mmu: &mut MMU<impl BankController>) {  }
    pub fn chan3_reset(&mut self, mmu: &mut MMU<impl BankController>) {  }
    pub fn chan4_reset(&mut self, mmu: &mut MMU<impl BankController>) {  }
}