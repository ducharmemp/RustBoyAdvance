use std::io::prelude::*;
use std::io::{Cursor, SeekFrom};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};

use super::common::CPUByteOrder;

fn build_system_ram(bios_rom: Vec<u8>, game_rom: Vec<u8>, ram_size: usize) -> Vec<u8> {
    let mut ram = vec![0; ram_size];
    let mut writer = Cursor::new(ram);

    // Convert the bios to a mutable slice
    writer.write_all(&bios_rom[..]).unwrap();

    // Write the game into our RAM vector
    writer.seek(SeekFrom::Start(0x08000000));
    writer.write_all(&game_rom[..]).unwrap();
    writer.into_inner()
}

pub struct RandomAccessMemory {
    big_endian: bool,
    cursor: Cursor<Vec<u8>>,
}

impl RandomAccessMemory {
    pub fn new(
        bios: Vec<u8>,
        game: Vec<u8>,
        size: u32,
        endianess: CPUByteOrder,
    ) -> RandomAccessMemory {
        let endian_flag: bool = match endianess {
            CPUByteOrder::BigEndian => true,
            CPUByteOrder::LittleEndian => false,
        };
        let ram = build_system_ram(bios, game, size as usize);

        RandomAccessMemory {
            big_endian: endian_flag,
            cursor: Cursor::new(ram),
        }
    }

    // pub fn set_endianess(&mut self, endianess: CPUByteOrder) {
    //     match endianess {
    //         CPUByteOrder::BigEndian => self.big_endian = true,
    //         CPUByteOrder::LittleEndian => self.big_endian = false,
    //     }
    // }

    pub fn read_byte(&mut self, address: u32) -> u8 {
        self.cursor.set_position(address as u64);
        self.cursor.get_ref()[address as usize]
    }

    pub fn write_byte(&mut self, address: u32, val: u8) {
        self.cursor.set_position(address as u64);
        self.cursor.write(&[val]);
    }

    pub fn read_16(&mut self, address: u32) -> u16 {
        self.cursor.set_position(address as u64);
        if self.big_endian {
            self.cursor.read_u16::<BigEndian>().unwrap()
        } else {
            self.cursor.read_u16::<LittleEndian>().unwrap()
        }
    }

    pub fn write_16(&mut self, address: u32, val: u16) {
        self.cursor.set_position(address as u64);
        if self.big_endian {
            self.cursor.write_u16::<BigEndian>(val).unwrap()
        } else {
            self.cursor.write_u16::<LittleEndian>(val).unwrap()
        }
    }

    pub fn read_32(&mut self, address: u32) -> u32 {
        self.cursor.set_position(address as u64);
        if self.big_endian {
            self.cursor.read_u32::<BigEndian>().unwrap()
        } else {
            self.cursor.read_u32::<LittleEndian>().unwrap()
        }
    }

    pub fn write_32(&mut self, address: u32, val: u32) {
        self.cursor.set_position(address as u64);
        if self.big_endian {
            self.cursor.write_u32::<BigEndian>(val).unwrap()
        } else {
            self.cursor.write_u32::<LittleEndian>(val).unwrap()
        }
    }
}
