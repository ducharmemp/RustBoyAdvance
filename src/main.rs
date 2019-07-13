use std::env;
use std::fs::File;
use std::io::{Read, Result};

extern crate bit_vec;
extern crate byteorder;

mod gameboy;

fn read_bin(path: String) -> Result<Vec<u8>> {
    let mut file_buf = Vec::new();
    File::open(path)?.read_to_end(&mut file_buf)?;
    Ok(file_buf)
}

fn main() -> Result<()> {
    let bios_rom = match env::args().nth(1) {
        None => panic!("Missing the BIOS path."),
        Some(path) => path,
    };

    let game_rom = match env::args().nth(2) {
        None => panic!("Missing the ROM path."),
        Some(path) => path,
    };

    println!("Reading {:?}", bios_rom);
    let bios = read_bin(bios_rom)?;
    println!("Reading {:?}", game_rom);
    let rom = read_bin(game_rom)?;

    let mut console = gameboy::cpu::CPU::new(bios, rom);
    console.run()?;
    Ok(())
}
