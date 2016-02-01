use std::io::prelude::*;
use std::io::{self, Cursor, SeekFrom};
use std::ops::{Range};

use cpu::cpu::CPU;

/*
Memory map thanks to http://problemkaputt.de/gbatek.htm#gbamemorymap
*/
// Internal memory
fn bios_memory (x: u32)->bool {0x00000000 <= x && x <= 0x00003FFF}
fn wramb_memory (x: u32)->bool {0x02000000 <= x && x <= 0x0203FFFF}
fn wramc_memory (x: u32)->bool {0x03000000 <= x && x<= 0x03007FFF}
fn io_memory (x: u32)->bool {0x04000000 <= x && x <= 0x040003FE}

// Display Memory
fn obj_pallet_memory (x: u32)->bool {0x05000000 <= x && x <= 0x050003FF}
fn vram_memory (x: u32)->bool {0x06000000 <= x && x <= 0x06017FFF}
fn obj_attr_memory (x: u32)->bool {0x07000000 <= x && x <= 0x070003FF}

// Game Pak Memory
fn wait_0_memory (x: u32)->bool {0x08000000 <= x && x <= 0x09FFFFFF}
fn wait_1_memory (x: u32)->bool {0x0A000000 <= x && x<= 0x0BFFFFFF}
fn wait_2_memory (x: u32)->bool {0x0C000000 <= x && x <= 0x0DFFFFFF}
fn gp_sram_memory (x: u32)->bool {0x0E000000 <= x && x <= 0x0E00FFFF}

pub struct GameBoyAdvance {
    cpu: CPU,
    ram: Vec<u8>,
}

fn build_system_ram(bios_rom: Vec<u8>, game_rom: Vec<u8>) -> Vec<u8> {
    let mut ram = vec![0; 0x0E00FFFF];
    ram.extend_from_slice(&bios_rom);
    let mut writer = Cursor::new(ram);

    // Write the game into our RAM vector
    writer.seek(SeekFrom::Start(0x08000000));
    writer.write_all(&game_rom);
    writer.into_inner()
}

impl GameBoyAdvance {
    pub fn new(bios_rom: Vec<u8>, game_rom: Vec<u8>) -> GameBoyAdvance {
        GameBoyAdvance {
            cpu: CPU::new(),
            ram: build_system_ram(bios_rom, game_rom),
        }
    }

    pub fn start(&mut self) {

    }

    pub fn fetch(&self, address: u32) {

    }

    pub fn write(&mut self, address: u32, value: u32) {

    }
}
