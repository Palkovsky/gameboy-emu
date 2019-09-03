#![allow(non_snake_case, non_camel_case_types)]

use super::*;
use sdl2::audio::AudioQueue;

const CPU_FREQUENCY: u32 = 1 << 20;
const SEQUENCER_FREQUENCY: u32 = 512;
const SEQUENCER_UPDATE_RATE: u16 = (CPU_FREQUENCY/SEQUENCER_FREQUENCY) as u16;
const SEQUENCER_STEP_COUNT: u16 = 8;
const DUTY_CYCLE_COUNT: u16 = 4;
const DUTY_CYCLE_STEPS: u16 = 8;
pub const BUFF_SIZE: usize = 1024;
pub const PLAYBACK_FREQUENCY: u32 = 96000;
const SAMPLE_APPEND_RATE: u16 = (CPU_FREQUENCY/PLAYBACK_FREQUENCY) as u16 + 1;

const DUTY_CYCLES: [[bool; DUTY_CYCLE_STEPS as usize]; DUTY_CYCLE_COUNT as usize] = [ 
    [false, true, true, true, true, true, true, true], // 12.5%
    [false, false, true, true, true, true, true, true], // 25%
    [false, false, false, false, true, true, true, true], // 50%
    [false, false, false, false, false, false, true, true], // 75%
];

trait SquareWaveRegisters {
    fn SWEEP_TIME(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn SWEEP_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8;
    fn SWEEP_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn SOUND_LENGTH(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn WAVE_DUTY(&self, mmu: &mut MMU<impl BankController>) -> u8;
    fn ENVELOPE_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8;
    fn ENVELOPE_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn INITIAL_VOLUME(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn FREQ(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn COUNTER_CONSECUTIVE_SELECT(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn INITIAL(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn _INITIAL(&self, mmu: &mut MMU<impl BankController>, value: bool);
    fn ENABLED(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn _ENABLED(&self, mmu: &mut MMU<impl BankController>, value: bool);
}

struct Channel1Regs;
impl SquareWaveRegisters for Channel1Regs {
    // NR 10 - Sweep register
    fn SWEEP_TIME(&self, mmu: &mut MMU<impl BankController>) -> u16         { (mmu.read(ioregs::NR_10) >> 4) as u16}
    fn SWEEP_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8        { mmu.read(ioregs::NR_10) & 7 }
    fn SWEEP_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool   { mmu.read(ioregs::NR_10) & 8 != 0 }

    // NR 11 - Length and wave duty registers
    fn SOUND_LENGTH(&self, mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_11) & 0x3F) as u16 }
    fn WAVE_DUTY(&self, mmu: &mut MMU<impl BankController>) -> u8    { mmu.read(ioregs::NR_11) >> 6 }

    // NR 12 - Volume Envelope register
    fn ENVELOPE_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8       { mmu.read(ioregs::NR_12) & 7 }
    fn ENVELOPE_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool  { mmu.read(ioregs::NR_12) & 8 != 0 }
    fn INITIAL_VOLUME(&self, mmu: &mut MMU<impl BankController>) -> u16       { (mmu.read(ioregs::NR_12) >> 4)  as u16 }

    // NR13 and NR14 - frequency
    fn FREQ(&self, mmu: &mut MMU<impl BankController>) -> u16 {
        (((mmu.read(ioregs::NR_14) & 7) as u16) << 8) + mmu.read(ioregs::NR_13) as u16
    }
    // NR 14 - Counter/Consecutive selection and initial flags
    fn COUNTER_CONSECUTIVE_SELECT(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_14, 6) }
    fn INITIAL(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_14, 7) }
    fn _INITIAL(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_14, 7, value) }

    // NR52 - Sound ON/OFF
    fn ENABLED(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_52, 0) }
    fn _ENABLED(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_52, 0, value) }
}

struct Channel2Regs;
impl SquareWaveRegisters for Channel2Regs {
    // No sweep in channel2
    fn SWEEP_TIME(&self, mmu: &mut MMU<impl BankController>) -> u16 { 0 }
    fn SWEEP_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8 { 0 }
    fn SWEEP_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool { false }

    // NR 21 - Length and wave duty registers
    fn SOUND_LENGTH(&self, mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_21) & 0x3F) as u16 }
    fn WAVE_DUTY(&self, mmu: &mut MMU<impl BankController>) -> u8 { mmu.read(ioregs::NR_21) >> 6 }

    // NR 22 - Volume Envelope register
    fn ENVELOPE_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8 { mmu.read(ioregs::NR_22) & 7 }
    fn ENVELOPE_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read(ioregs::NR_22) & 8 != 0 }
    fn INITIAL_VOLUME(&self, mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_22) >> 4)  as u16 }

    // NR23 and NR24 - frequency
    fn FREQ(&self, mmu: &mut MMU<impl BankController>) -> u16 {
        (((mmu.read(ioregs::NR_24) & 7) as u16) << 8) + mmu.read(ioregs::NR_23) as u16
    }
    // NR 24 - Counter/Consecutive selection and initial flags
    fn COUNTER_CONSECUTIVE_SELECT(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read(ioregs::NR_24) & 0x40 != 0 }
    fn INITIAL(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read(ioregs::NR_24) & 0x80 != 0}
    fn _INITIAL(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_24, 7, value) }

    // NR52 - Sound ON/OFF
    fn ENABLED(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_52, 1) }
    fn _ENABLED(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_52, 1, value) }
}

struct SquareWave<T: SquareWaveRegisters> {
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
    /* Output buffer */
    buff: Vec<i16>,
    /* Used to fillup buffer for player with PLAYBACK_FREQUENCY sampling rate, not CPU_FREQUENCY */
    sample_counter: u16,
    /* Provides access to memory mapped registers */
    regs: T,
}

impl <T: SquareWaveRegisters>SquareWave<T> {
    fn new(mmu: &mut MMU<impl BankController>, regs: T) -> Self { 
        Self {
            frequency:      regs.FREQ(mmu),
            volume:         regs.INITIAL_VOLUME(mmu),
            length:         regs.SOUND_LENGTH(mmu),
            timer:          2048 - regs.FREQ(mmu),
            duty_cycle:     0,
            sweep_timer:    regs.SWEEP_TIME(mmu),
            envelope_count: regs.ENVELOPE_SHIFTS(mmu),
            buff:           Vec::with_capacity(BUFF_SIZE),
            sample_counter: 0,
            regs:           regs,
        }
    }

    fn reset(&mut self, mmu: &mut MMU<impl BankController>) {
        self.buff.clear();
        self.frequency = self.regs.FREQ(mmu);
        self.volume = self.regs.INITIAL_VOLUME(mmu);
        self.length = self.regs.SOUND_LENGTH(mmu);
        self.timer = 2048 - self.frequency;
        self.duty_cycle = 0;
        self.sweep_timer = self.regs.SWEEP_TIME(mmu);
        self.envelope_count = self.regs.ENVELOPE_SHIFTS(mmu);
        self.sample_counter = 0;
    }

    fn tick(&mut self, mmu: &mut MMU<impl BankController>) {
        // If triggered start.
        if self.regs.INITIAL(mmu) { 
            self.reset(mmu);
            self.regs._INITIAL(mmu, false);
            self.regs._ENABLED(mmu, true);
        }
        if !self.regs.ENABLED(mmu) { return }
        // Update timer and duty cycle
        if self.timer > 0 { self.timer -= 1 };
        if self.timer == 0 {
            self.duty_cycle = (self.duty_cycle + 1) % DUTY_CYCLE_STEPS;
            self.timer = 2048 - self.frequency;
        }
        // Generate sample
        self.sample_counter += 1;
        if self.sample_counter == SAMPLE_APPEND_RATE {
            let is_on  = DUTY_CYCLES[self.regs.WAVE_DUTY(mmu) as usize][self.duty_cycle as usize];
            let sample = if is_on { (i16::max_value()/0xF)*(self.volume as i16) } else { 0 };
            self.buff.push(sample);
            self.sample_counter = 0;
        }
    }

    pub fn buffer(&mut self) -> &mut Vec<i16> { &mut self.buff }

    fn length(&mut self, mmu: &mut MMU<impl BankController>) {
        if !self.regs.ENABLED(mmu) || self.regs.SOUND_LENGTH(mmu) == 0 { return }
        if self.length > 0 { self.length -= 1; }
        if self.length == 0 {
            if self.regs.COUNTER_CONSECUTIVE_SELECT(mmu) {
                self.regs._ENABLED(mmu, false); 
            } else {
                //self.reset(mmu);
            }
        }
    }

    fn sweep(&mut self, mmu: &mut MMU<impl BankController>) {
        if !self.regs.ENABLED(mmu) { return }
        self.sweep_timer -= 1;
        if self.sweep_timer == 0 {
            let delta = self.frequency/(2 as u16).pow(self.regs.SWEEP_SHIFTS(mmu) as u32);
            if self.regs.SWEEP_DIRECTION(mmu) { 
                if self.frequency >= delta { self.frequency -= delta; }
            } else if self.frequency + delta > 0x7FF { 
                self.regs._ENABLED(mmu, false);
            } else {
                self.frequency += delta;
            }
            self.sweep_timer = self.regs.SWEEP_TIME(mmu);
        }
    }

    fn envelope(&mut self, mmu: &mut MMU<impl BankController>) {
        if !self.regs.ENABLED(mmu) || 
            self.volume == 0
            { return }
        if self.regs.ENVELOPE_DIRECTION(mmu) {
            if self.volume < 0xF { self.volume += 1 };
        } else {
            if self.volume > 0   { self.volume -= 1 };
        }
        self.envelope_count -= 1;
    }
}

pub struct APU {
    /* If sequencer_cycle % (1MHz/512Hz) == 0 then sequencer_step increments */
    sequencer_cycle: u16,
    /* Number between 0-7. It wraps around. */
    sequencer_step: u16,
    /* Quadrangular wave patterns with sweep and envelope functions. */
    chan1: SquareWave<Channel1Regs>,
    chan2: SquareWave<Channel2Regs>,
}

impl <T: BankController>Clocked<T> for APU {
    
    // Can always catchup
    fn next_time(&self, _: &mut MMU<T>) -> u64 { 1 }

    fn step(&mut self, mmu: &mut MMU<T>) { 
        self.chan1.tick(mmu);
        self.chan2.tick(mmu);

        self.sequencer_cycle += 1;
        if self.sequencer_cycle == SEQUENCER_UPDATE_RATE {
            match self.sequencer_step { 0 | 2 | 4 | 6 => {
                    self.chan1.length(mmu);
                    self.chan2.length(mmu);
                }, _ => {},
            };
            match self.sequencer_step { 2 | 6 => {
                    self.chan1.sweep(mmu);
                    // No sweep for chan2
                }, _ => {},
            };
            match self.sequencer_step { 7 => {
                    self.chan1.envelope(mmu);
                    self.chan2.envelope(mmu);
                }, _ => {},
            };

            self.sequencer_cycle = 0;
            self.sequencer_step = (self.sequencer_step + 1) % SEQUENCER_STEP_COUNT;
        }
    }
}

impl APU {
    pub fn new(mmu: &mut MMU<impl BankController>) -> Self {
        Self {
            sequencer_cycle: 0,
            sequencer_step: 0,
            chan1: SquareWave::new(mmu, Channel1Regs),
            chan2: SquareWave::new(mmu, Channel2Regs),
        }
    }

    /* Is channel conected to terminal 1? */
    pub fn SO1(mmu: &mut MMU<impl BankController>, chan: u8) -> bool {
        if chan > 4  || chan == 0 { return false }
        let chan = chan - 1;
        let nr_51 = mmu.read(ioregs::NR_51);
        (nr_51 & (1 << chan)) != 0
    }
    
    /* Is channel conected to terminal 2? */
    pub fn SO2(mmu: &mut MMU<impl BankController>, chan: u8) -> bool {
        if chan > 4  || chan == 0 { return false }
        let chan = chan - 1;
        let nr_51 = mmu.read(ioregs::NR_51) >> 4;
        (nr_51 & (1 << chan)) != 0
    }

    pub fn chan1_samples(&mut self) -> &mut Vec<i16> { self.chan1.buffer() }
    pub fn chan2_samples(&mut self) -> &mut Vec<i16> { self.chan2.buffer() }

    pub fn chan1_reset(&mut self, mmu: &mut MMU<impl BankController>) { self.chan1.reset(mmu); }
    pub fn chan2_reset(&mut self, mmu: &mut MMU<impl BankController>) { self.chan2.reset(mmu); }
    pub fn chan3_reset(&mut self, mmu: &mut MMU<impl BankController>) {  }
    pub fn chan4_reset(&mut self, mmu: &mut MMU<impl BankController>) {  }
}