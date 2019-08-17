use super::*;

pub struct GPU {
    vram: Vec<Byte>,
    oam: Vec<Byte>,
}

impl GPU {
    pub fn new() -> Self {
        Self { vram: vec![0; VRAM_SIZE], oam: vec![0; OAM_SIZE] }
    }

    pub fn vram(&mut self) -> MutMem { &mut self.vram[..] }

    pub fn oam(&mut self) -> MutMem { &mut self.oam[..] }
}