use std::fs;
use std::env;
use std::error::Error;
use std::io::Read;
use std::path::Path;

extern crate bit_vec;
extern crate byteorder;

mod gameboy;

fn read_bin<P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut file = match fs::File::open(path) {
    	Err(why) => panic!("Couldn't open: {}", Error::description(&why)),
    	Ok(file) => file,
    };
    let mut file_buf = Vec::new();
    match file.read_to_end(&mut file_buf) {
    	Err(why) => panic!("Couldn't read: {}", Error::description(&why)),
    	Ok(_) => {}
    };
    file_buf
}

fn main() {
	let bios_rom = match env::args().nth(1) {
		None => panic!("Missing the BIOS path."),
		Some(path) => path,
	};


	let game_rom = match env::args().nth(2) {
		None => panic!("Missing the ROM path."),
		Some(path) => path,
	};

	println!("Reading {:?}", bios_rom);
	let bios = read_bin(bios_rom);
	println!("Reading {:?}", game_rom);
    let rom = read_bin(game_rom);

	let mut console = gameboy::cpu::CPU::new(bios, rom);
    console.run()
}
