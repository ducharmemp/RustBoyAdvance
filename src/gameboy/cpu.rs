/*
Interrupt Vector Table
Address		Exception	Mode on entry
0x00000000	Reset		Supervisor
0x00000004	Undefined	Undefined
0x00000008	Software	Supervisor
0x0000000C	Abort pref	Abort
0x00000010	Abort data	Abort
0x00000014	Reserved
0x00000018	IRQ			IRQ
0x0000001C	FIQ			FIQ

If multiple exceptions occur simultaneously, prioritize:
    - Reset
    - Data Abort
    - FIQ
    - IRQ
    - Prefetch Abort
    - Undefined Instruction, Software Interrupt
*/

use std::io;

// TODO: Set the carry flag on the barrel shifter if there are bits that are shifted out (page 14 of http://bear.ces.cwru.edu/eecs_382/ARM7-TDMI-manual-pt2.pdf)
use super::common::{CPUByteOrder, CPUExecutionMode, CPUInstructionMode};
use super::mmu::RandomAccessMemory;

use bit_vec::BitVec;

use super::cpsr::{C_FLAG, FIQ_DISABLE, IRQ_DISABLE, N_FLAG, STATE_BIT, V_FLAG, Z_FLAG};
use super::utils::{bitvec_from_u32, u32_from_bitvec};
/*
Memory map thanks to http://problemkaputt.de/gbatek.htm#gbamemorymap
*/
// Internal memory
const SYS_MAX_MEMORY: u32 = 0x0E00FFFF;

fn bios_memory(x: u32) -> bool {
    0x00000000 <= x && x <= 0x00003FFF
}
fn wramb_memory(x: u32) -> bool {
    0x02000000 <= x && x <= 0x0203FFFF
}
fn wramc_memory(x: u32) -> bool {
    0x03000000 <= x && x <= 0x03007FFF
}
fn io_memory(x: u32) -> bool {
    0x04000000 <= x && x <= 0x040003FE
}

// Display Memory
fn obj_pallet_memory(x: u32) -> bool {
    0x05000000 <= x && x <= 0x050003FF
}
fn vram_memory(x: u32) -> bool {
    0x06000000 <= x && x <= 0x06017FFF
}
fn obj_attr_memory(x: u32) -> bool {
    0x07000000 <= x && x <= 0x070003FF
}

// Game Pak Memory
fn wait_0_memory(x: u32) -> bool {
    0x08000000 <= x && x <= 0x09FFFFFF
}
fn wait_1_memory(x: u32) -> bool {
    0x0A000000 <= x && x <= 0x0BFFFFFF
}
fn wait_2_memory(x: u32) -> bool {
    0x0C000000 <= x && x <= 0x0DFFFFFF
}
fn gp_sram_memory(x: u32) -> bool {
    0x0E000000 <= x && x <= SYS_MAX_MEMORY
}

pub struct CPU {
    // TODO: refactor to include getters and setters?
    pub pc: u32,
    pub cpsr: BitVec<u32>,
    pub spsr_fiq: BitVec<u32>,
    pub spsr_svc: BitVec<u32>,
    pub spsr_abt: BitVec<u32>,
    pub spsr_irq: BitVec<u32>,
    pub spsr_und: BitVec<u32>,

    // unbanked registers r0-r7
    gen_registers: [u32; 8],

    // Banked registers, depend on the current mode
    // r8-r14
    hi_registers_user: [u32; 7],
    hi_registers_supervisor: [u32; 7],
    hi_registers_irq: [u32; 7],
    hi_registers_fiq: [u32; 7],

    status: u32,

    // CPU Configurations
    execution_mode: CPUExecutionMode,
    instruction_mode: CPUInstructionMode,

    mmu: RandomAccessMemory,

    word_size: u32,
}

impl CPU {
    pub fn new(bios: Vec<u8>, game: Vec<u8>) -> CPU {
        CPU {
            mmu: RandomAccessMemory::new(bios, game, SYS_MAX_MEMORY, CPUByteOrder::LittleEndian),

            gen_registers: [0; 8],
            hi_registers_user: [0; 7],
            hi_registers_supervisor: [0; 7],
            hi_registers_irq: [0; 7],
            hi_registers_fiq: [0; 7],
            pc: 0,
            cpsr: BitVec::from_elem(32, false),
            spsr_fiq: BitVec::from_elem(32, false),
            spsr_svc: BitVec::from_elem(32, false),
            spsr_abt: BitVec::from_elem(32, false),
            spsr_irq: BitVec::from_elem(32, false),
            spsr_und: BitVec::from_elem(32, false),
            status: 0,
            word_size: 4,

            // CPU Configurations
            execution_mode: CPUExecutionMode::Supervisor,
            instruction_mode: CPUInstructionMode::ARM32,
        }
    }

    fn barrel_shift(&mut self, shift: u32, val: u32) -> u32 {
        println!("{:?} {:?}", shift, val);
        let shift_register = 0x1 & shift != 0;
        let shift_type = (0b110 & shift) >> 1;
        let shift_amount = if shift_register {
            self.get_register_arm((shift & 0b11110000) >> 4) as u8
        } else {
            ((shift & 0b11111000) >> 3) as u8
        };
        if shift_register {
            match shift_type {
                0b00 => {
                    println!("{:?}", "Logical left");
                    val << shift
                }
                0b01 => {
                    println!("{:?}", "Logical right");
                    match val.checked_shr(shift) {
                        Some(value) => value,
                        None => 0,
                    }
                }
                0b10 => {
                    println!("{:?}", "Arithmetic right");
                    let top_val = val & (1 << 31);
                    match val.checked_shr(shift) {
                        Some(value) => {
                            let mut v = value;
                            for i in 0..shift {
                                v &= top_val >> i;
                            }
                            v
                        }
                        None => 0,
                    }
                }
                0b11 => {
                    println!("{:?}", "Rotate right");
                    val.rotate_right(shift_amount as u32)
                }
                _ => {
                    println!("{:#b}", shift_type);
                    panic!();
                }
            }
        } else {
            val << shift_amount
        }
    }

    fn run_arm(&mut self) {
        let ins = self.mmu.read_32(self.pc);

        let condition = ins >> 28;
        self.pc += self.word_size;

        // Can probably trim this down a bit to make it less verbose
        if !match condition {
            0x0 => self.cpsr[Z_FLAG],                       // Z set
            0x1 => !self.cpsr[Z_FLAG],                      // Z clear
            0x2 => self.cpsr[C_FLAG],                       // C set
            0x3 => !self.cpsr[C_FLAG],                      // C clear
            0x4 => self.cpsr[N_FLAG],                       // N set
            0x5 => !self.cpsr[N_FLAG],                      // N clear
            0x6 => self.cpsr[V_FLAG],                       // V set
            0x7 => !self.cpsr[V_FLAG],                      // V clear
            0x8 => self.cpsr[C_FLAG] && !self.cpsr[Z_FLAG], // C set and Z clear
            0x9 => !self.cpsr[C_FLAG] || self.cpsr[Z_FLAG], // C clear or Z set
            0x10 => self.cpsr[N_FLAG] == self.cpsr[V_FLAG], // N set and V set, or N clear and V clear (N == V)
            0xA => self.cpsr[N_FLAG] != self.cpsr[V_FLAG],  // N != V
            0xB => !self.cpsr[Z_FLAG] && (self.cpsr[N_FLAG] == self.cpsr[V_FLAG]), // Z == 0, N == V
            0xC => self.cpsr[Z_FLAG] || (self.cpsr[N_FLAG] != self.cpsr[V_FLAG]), // Z == 1 or N != V
            0xD => true, // Unconditional ???
            0xE => true, /* Unconditional instruction space */
            0xF => false,
            _ => unreachable!(),
        } {
            return;
        }

        if ins & 0x0FFFFFF0 == 0x012FFF10 {
            // BX
            println!("{:?}", "BX");
            self.handle_branch_exchange(ins);
        } else if ins & 0x0E000000 == 0x0A000000 {
            // B/BL
            println!("{:?}", "Branch");
            self.handle_branch(ins);
        } else if ins & 0x0E000010 == 0x06000010 {
            // Unknown
            println!("{:?}", "Unknown");
        } else if ins & 0x0FB00FF0 == 0x01000090 {
            // SWP
            println!("{:?}", "SWP");
            self.handle_swap(ins);
        } else if ins & 0x0FC000F0 == 0x00000090 {
            // MUL_MLA
            println!("{:?}", "MUL_MLA");
            self.handle_mulmla(ins);
        } else if ins & 0x0F8000F0 == 0x00800090 {
            // MUL_MLAL
            println!("{:?}", "MUL_MLAL");
            self.handle_mulmlal(ins);
        } else if ins & 0x0FBF0FFF == 0x010F0000 {
            // MRS
            println!("{:?}", "MRS");
            self.handle_mrs(ins);
        } else if ins & 0x0FBFFFF0 == 0x0129F000 {
            // MSR_Reg
            println!("{:?}", "MSR");
            self.handle_msr(ins);
        } else if ins & 0x0DBFF000 == 0x0128F000 {
            // MSR_Flags
            println!("{:?}", "MSR FLAGS");
            self.handle_msr_flags(ins);
        } else if ins & 0x0C000000 == 0x04000000 {
            // LDR_STR
            println!("{:?}", "LDR/STR");
            self.handle_load_store(ins);
        } else if ins & 0x0E400F90 == 0x00000090 {
            println!("{:?}", "LDRH_STRG_Imm");
        } else if ins & 0x0E400090 == 0x00400090 {
            // LDRH_STRG_Imm
            println!("{:?}", "LDRH_STRG_Imm");
        } else if ins & 0x0E000000 == 0x08000000 {
            // LDM_STM
            println!("{:?}", "LDM_STM");
            self.handle_load_store_multiple(ins);
        } else if ins & 0x0F000000 == 0x0F000000 {
            // SWI, software interrupt, go into supervisor mode and set the PC to 0x8
            // Note, since this is the GBA, this will probably never happen
            println!("{:?}", "SWI");
            self.execution_mode = CPUExecutionMode::Supervisor;
            self.spsr_svc = self.cpsr.clone();
            let adj_val = self.pc + 4;
            self.set_register_arm(14, adj_val);
            self.pc = 0x8;
            return;
        } else if ins & 0x0F000010 == 0x0E000000 {
            // CDP, No coprocessors on the GBA (thank god)
            println!("Coprocessor instruction, this should never happen");
        } else if ins & 0x0F000010 == 0x0E000010 {
            println!("Coprocessor register transfer, this should never happen");
        } else if ins & 0x0E000000 == 0x0C000000 {
            // LDC_STC
            println!("Coprocessor data transfer, this should never happen");
        } else if ins & 0x0C000000 == 0x00000000 {
            // Data processing (MUL, ADD, MOV, etc.)
            println!("{:?}", "DATA");
            self.handle_data(ins);
        } else {
            panic!("Unrecognised instruction!")
        };
    }

    fn run_thumb(&mut self) {
        let ins = self.mmu.read_16(self.pc);
        // Don't have to deal with conditionals here, only for branching
        if (ins & 0xF800) == 0x1800 {
            self.thumb_handle_addsub(ins)
        } else if (ins & 0xE000) == 0x0000 {
            self.thumb_handle_movshift(ins)
        } else if (ins & 0xE000) == 0x2000 {
            self.thumb_handle_dataprocessing_f(ins)
        } else if (ins & 0xFFC0) == 0x4340 {
            self.thumb_handle_mul(ins)
        }
        // Decoded separately as MUL is no ARM state data processing.
        else if (ins & 0xFC00) == 0x4000 {
            self.thumb_handle_op(ins)
        } else if (ins & 0xFC00) == 0x4400 {
            self.thumb_handle_hiregopbx(ins)
        } else if (ins & 0xF800) == 0x4800 {
            self.thumb_handle_ldrpcimm(ins)
        } else if (ins & 0xF200) == 0x5000 {
            self.thumb_handle_ldrstrreg(ins)
        } else if (ins & 0xF200) == 0x5200 {
            self.thumb_handle_ldrhstrhreg(ins)
        } else if (ins & 0xE000) == 0x6000 {
            self.thumb_handle_ldrstrimm(ins)
        } else if (ins & 0xF000) == 0x8000 {
            self.thumb_handle_ldrhstrhimm(ins)
        } else if (ins & 0xF000) == 0x9000 {
            self.thumb_handle_ldrstrspimm(ins)
        } else if (ins & 0xF000) == 0xA000 {
            self.thumb_handle_addrimm(ins)
        } else if (ins & 0xFF00) == 0xB000 {
            self.thumb_handle_addspoffset(ins)
        } else if (ins & 0xF600) == 0xB400 {
            self.thumb_pushpop(ins)
        } else if (ins & 0xF000) == 0xC000 {
            self.thumb_handle_ldmstmregs(ins)
        } else if (ins & 0xFF00) == 0xDF00 {
            self.thumb_handle_swi(ins)
        } else if (ins & 0xFF00) == 0xDE00 {
            /* BAL is undefined.*/
        } else if (ins & 0xF000) == 0xD000 {
            self.thumb_handle_branchcondoffs(ins)
        } else if (ins & 0xF800) == 0xE000 {
            self.thumb_handle_branchoffs(ins)
        } else if (ins & 0xF000) == 0xF000 {
            self.thumb_handle_branchlongoffs(ins)
        } else {
            panic!(
                "I'm in Thumb mode and I don't know what to do! {:#x}, {:#b}",
                self.pc, ins
            )
        };
        self.pc += self.word_size;
    }

    /*
    THUMB INSTRUCTIONS
    */
    fn thumb_handle_addsub(&mut self, instruction: u16) {
        let immediate = (1 << 10) & instruction != 0;
        let rs = ((0x7 << 3) & instruction) >> 3;
        let rd = 0x7 & instruction;
        let add = (1 << 9) & instruction == 0;
        let val = ((0x7 << 6) & instruction) >> 6;

        let final_val = if !immediate {
            let reg_val = self.get_register_thumb(val as u32);
            if add {
                self.get_register_thumb(rs as u32)
                    .wrapping_add(reg_val as u32)
            } else {
                self.get_register_thumb(rs as u32)
                    .wrapping_sub(reg_val as u32)
            }
        } else {
            if add {
                self.get_register_thumb(rs as u32).wrapping_add(val as u32)
            } else {
                self.get_register_thumb(rs as u32).wrapping_sub(val as u32)
            }
        };
        self.set_register_thumb(rd as u32, final_val as u16);
    }

    fn thumb_handle_movshift(&mut self, instruction: u16) {}

    fn thumb_handle_dataprocessing_f(&mut self, instruction: u16) {
        let immediate = 0xff & instruction;
        let rd = ((0x7 << 8) & instruction) >> 8;
        let op = ((0b11 << 11) & instruction) >> 11;

        let reg_val = self.get_register_thumb(rd as u32) as u16;
        let write_val = match op {
            0 => immediate,
            1 => reg_val - immediate,
            2 => reg_val + immediate,
            3 => reg_val - immediate,
            _ => unreachable!(),
        };

        self.cpsr.set(N_FLAG, false);
        self.cpsr.set(Z_FLAG, false);
        self.cpsr.set(C_FLAG, false);
        self.cpsr.set(V_FLAG, false);

        if (write_val as u32) == 0 {
            self.cpsr.set(Z_FLAG, true);
        }
        if (write_val as i32) < 0 {
            self.cpsr.set(N_FLAG, true);
        }

        self.set_register_thumb(rd as u32, write_val);
    }

    fn thumb_handle_mul(&mut self, instruction: u16) {}

    fn thumb_handle_op(&mut self, instruction: u16) {
        let rd = 0x7 & instruction;
        let rs = ((0x7 << 3) & instruction) >> 3;
        let op = ((0xf << 6) & instruction) >> 6;

        let rd_val = self.get_register_thumb(rd as u32) as i16;
        let rs_val = self.get_register_thumb(rs as u32) as i16;

        let write_val: i16 = match op {
            0x0 => rd_val & rs_val,
            0x1 => rd_val ^ rs_val,
            0x2 => rd_val.wrapping_shl(rs_val as u32),
            0x3 => rd_val.wrapping_shr(rs_val as u32),
            0x4 => match rd_val.checked_shr(rs_val as u32) {
                Some(value) => {
                    let mut v = value;
                    let top_val = (1 << 15) & value;
                    for i in 0..rs_val {
                        v &= top_val >> i;
                    }
                    v
                }
                None => 0,
            },
            0x5 => {
                let c = if self.cpsr[C_FLAG] { 1 } else { 0 };
                rd_val + rs_val + c
            }
            0x6 => {
                let c = if self.cpsr[C_FLAG] { 1 } else { 0 };
                rd_val - rs_val - !c
            }
            0x7 => rd_val.rotate_right(rs_val as u32),
            0x8 => {
                let val = rd_val & rs_val;
                self.cpsr.set(Z_FLAG, val == 0);
                return; // early exit because we only want to do a comparison here
            }
            0x9 => -(rs_val as i16),
            0xa => {
                let val = (rd_val as i16) - (rs_val as i16);
                self.cpsr.set(Z_FLAG, val == 0);
                self.cpsr.set(N_FLAG, val < 0);
                return; // early exit because we only want to do a comparison here
            }
            0xb => {
                let val = (rd_val as i16) + (rs_val as i16);
                self.cpsr.set(Z_FLAG, val == 0);
                self.cpsr.set(N_FLAG, val < 0);
                return; // early exit because we only want to do a comparison here
            }
            0xc => rd_val | rs_val,
            0xd => rd_val * rs_val,
            0xe => rd_val & !rs_val,
            0xf => !rs_val,
            _ => unreachable!(),
        };

        self.cpsr.set(N_FLAG, false);
        self.cpsr.set(Z_FLAG, false);
        self.cpsr.set(C_FLAG, false);
        self.cpsr.set(V_FLAG, false);

        if (write_val as u32) == 0 {
            self.cpsr.set(Z_FLAG, true);
        }
        if (write_val as i32) < 0 {
            self.cpsr.set(N_FLAG, true);
        }

        self.set_register_thumb(rd as u32, write_val as u16);
    }

    fn thumb_handle_hiregopbx(&mut self, instruction: u16) {
        let op = ((0x3 << 8) & instruction) >> 8;
        let h1 = (0x1 << 7) & instruction != 0;
        let h2 = (0x1 << 6) & instruction != 0;
        let rs = ((0x7 << 3) & instruction) >> 3;
        let rd = 0x7 & instruction;
    }

    fn thumb_handle_ldrpcimm(&mut self, instruction: u16) {}

    fn thumb_handle_ldrstrreg(&mut self, instruction: u16) {}

    fn thumb_handle_ldrhstrhreg(&mut self, instruction: u16) {}

    fn thumb_handle_ldrstrimm(&mut self, instruction: u16) {}

    fn thumb_handle_ldrhstrhimm(&mut self, instruction: u16) {}

    fn thumb_handle_ldrstrspimm(&mut self, instruction: u16) {}

    fn thumb_handle_addrimm(&mut self, instruction: u16) {}

    fn thumb_handle_addspoffset(&mut self, instruction: u16) {}

    fn thumb_pushpop(&mut self, instruction: u16) {}

    fn thumb_handle_ldmstmregs(&mut self, instruction: u16) {}

    fn thumb_handle_swi(&mut self, instruction: u16) {}

    fn thumb_handle_branchcondoffs(&mut self, instruction: u16) {}

    fn thumb_handle_branchoffs(&mut self, instruction: u16) {}

    fn thumb_handle_branchlongoffs(&mut self, instruction: u16) {}

    /*
    ARM INSTRUCTIONS
    */
    fn handle_mrs(&mut self, instruction: u32) {
        println!("MRS");
        // Transfer PSR contents to a register. We currently panic if the P bit is set, because reasons
        let p_bit_set = (1 << 22) & instruction != 0;
        let rd = ((0xf << 12) & instruction) >> 12;

        let psr = if p_bit_set {
            u32_from_bitvec(self.get_spsr_register())
        } else {
            u32_from_bitvec(&self.cpsr)
        };

        self.set_register_arm(rd, psr);
    }

    fn handle_msr(&mut self, instruction: u32) {
        // Transfer register contents to PSR. We currently panic if the P bit is set, because reasons
        let p_bit_set = (1 << 22) & instruction != 0;
        let rm = ((0xf << 12) & instruction) >> 12;

        let val = self.get_register_arm(rm);
        if p_bit_set {
            self.set_spsr_register(val);
            return;
        }

        self.cpsr = bitvec_from_u32(val);
    }

    fn handle_msr_flags(&mut self, instruction: u32) {
        // Transfer register contents to PSR flags only. We currently panic if the P bit is set, because reasons
        let p_bit_set = (1 << 22) & instruction != 0;
        let immediate = (1 << 25) & instruction != 0;
        let rm = ((0xf << 12) & instruction) >> 12;

        let val = if immediate {
            let shift = ((0xf << 8) & instruction) >> 8;
            self.barrel_shift(shift, 0xff & instruction)
        } else {
            self.get_register_arm(rm)
        };
        if p_bit_set {
            self.set_spsr_register(val);
            return;
        }

        self.cpsr = bitvec_from_u32(val);
    }

    fn handle_mulmla(&mut self, instruction: u32) {
        let accumulate = ((1 << 21) & instruction) >> 21 != 0;
        let s_bit = ((1 << 20) & instruction) >> 20 != 0;

        let rd = ((0xf << 16) & instruction) >> 16;

        // Grab the next several values as u64 so we don't horribly overflow
        let rn = self.get_register_arm(((0xf << 12) & instruction) >> 12) as u64;
        let rs = self.get_register_arm(((0xf << 8) & instruction) >> 8) as u64;
        let rm = self.get_register_arm(0xf & instruction) as u64;

        let val: u64 = rm * rs + rn;
		// Truncate the 64 bit value, we only care about the bottom 32 bits
        self.set_register_arm(rd, val as u32);

        if !s_bit {
            return;
        }

        self.cpsr.set(N_FLAG, false);
        self.cpsr.set(Z_FLAG, false);
        self.cpsr.set(C_FLAG, false);
        self.cpsr.set(V_FLAG, false);

        if (val as u32) == 0 {
            self.cpsr.set(Z_FLAG, true);
        }
        if (val as i32) < 0 {
            self.cpsr.set(N_FLAG, true);
        }
    }

    fn handle_mulmlal(&mut self, instruction: u32) {
        let is_unsigned = ((1 << 22) & instruction) >> 22 == 0;
        let accumulate = ((1 << 21) & instruction) >> 21 != 0;
        let s_bit = ((1 << 20) & instruction) >> 20 != 0;

        if s_bit {
            self.cpsr.set(N_FLAG, false);
            self.cpsr.set(Z_FLAG, false);
            self.cpsr.set(C_FLAG, false);
            self.cpsr.set(V_FLAG, false);
        }

        let rd_hi = ((0xf << 16) & instruction) >> 16;
        let rd_lo = ((0xf << 12) & instruction) >> 16;

        let rs = self.get_register_arm(((0xf << 8) & instruction) >> 8);
        let rm = self.get_register_arm(0xf & instruction);

        let accum_val: u64 = if accumulate {
            let hi_val = self.get_register_arm(rd_hi) as u64;
            let lo_val = self.get_register_arm(rd_lo) as u64;

            (hi_val << 32) | lo_val
        } else {
            0
        };

        if is_unsigned {
            let u_rs = rs as u64;
            let u_rm = rs as u64;

            let res: u64 = u_rs * u_rm + accum_val;
            self.set_register_arm(rd_hi, (res >> 32) as u32);
            self.set_register_arm(rd_lo, res as u32);

            if s_bit {
                if res == 0 {
                    self.cpsr.set(Z_FLAG, true);
                } else {
                    self.cpsr.set(Z_FLAG, false);
                }
                if res as i32 <= 0 {
                    self.cpsr.set(N_FLAG, true);
                } else {
                    self.cpsr.set(N_FLAG, false);
                }
            }
        } else {
            let s_rs = rs as i64;
            let s_rm = rs as i64;

            let res: i64 = s_rs * s_rm + (accum_val as i64);
            self.set_register_arm(rd_hi, (res >> 32) as u32);
            self.set_register_arm(rd_lo, res as u32);
        }
    }

    fn handle_data(&mut self, instruction: u32) {
        let opcode = ((0xf << 21) & instruction) >> 21;
        let s_bit = (1 << 20) & instruction != 0;
        let rd = ((0xf << 12) & instruction) >> 12;
        let rn = ((0xf << 16) & instruction) >> 16;
        let immediate = (1 << 25) & instruction != 0;

        if s_bit {
            self.cpsr.set(N_FLAG, false);
            self.cpsr.set(Z_FLAG, false);
            self.cpsr.set(C_FLAG, false);
            self.cpsr.set(V_FLAG, false);
        }

        println!("{:#b}", instruction);
        println!("RD {:?}", rd);
        println!("RN {:?}", rn);

        let first_operand = self.get_register_arm(rn);
        let mut second_operand: u32 = 0;
        let bottom_operand = 0xfff & instruction;
        if immediate == true {
            let imm = 0xff & bottom_operand;
            let rotate = (((0xf << 8) & bottom_operand) >> 8) as u32;
            second_operand = imm.rotate_right(2 * rotate);
        } else {
            let shift = ((0xff << 4) & instruction) >> 4;
            let tmp = self.get_register_arm(instruction & 0xf);
            second_operand = self.barrel_shift(shift, tmp);
        }

        println!("OP1 {:?}", first_operand);
        println!("OP2 {:?}", second_operand);
        println!("OPERAND {:#x}", opcode);

        // Dispatch the instruction to the appropriate function or action
        let write_val = match opcode {
            0x0 => first_operand & second_operand,             // AND
            0x1 => first_operand ^ second_operand,             // EOR
            0x2 => first_operand.wrapping_sub(second_operand), // SUB
            0x3 => second_operand.wrapping_sub(first_operand), // RSB
            0x4 => first_operand.wrapping_add(second_operand), // ADD
            0x5 => {
                let c = if self.cpsr[C_FLAG] { 1 } else { 0 };
                first_operand + second_operand + c
            } // ADC
            0x6 => {
                let c = if self.cpsr[C_FLAG] { 1 } else { 0 };
                first_operand + second_operand + c - 1
            } // SBC
            0x7 => {
                let c = if self.cpsr[C_FLAG] { 1 } else { 0 };
                second_operand + first_operand + c - 1
            } // RSC
            0x8 => {
                let val = first_operand & second_operand;
                self.cpsr.set(Z_FLAG, val == 0);
                return; // early exit because we only want to do a comparison here
            } // TST
            0x9 => {
                let val = first_operand ^ second_operand;
                println!("TEQ {:#x}", val);
                self.cpsr.set(Z_FLAG, val == 0);
                return;
            } // TEQ
            0xa => {
                let val = first_operand - second_operand;
                self.cpsr.set(Z_FLAG, val == 0);
                println!("VAL {:?}", val);
                println!("CMP {:?}", self.cpsr);
                return;
            } // CMP
            0xb => {
                // Need to test for overflow somehow
                let val = match first_operand.checked_add(second_operand) {
                    Some(val) => val,
                    None => {
                        self.cpsr.set(V_FLAG, true);
                        0
                    }
                };
                self.cpsr.set(Z_FLAG, val == 0);
                return;
            } // CMN
            0xc => first_operand | second_operand,             // ORR
            0xd => {
                println!("MOV {:?}, {:#x}", rd, second_operand);
                second_operand
            } // MOV
            0xe => first_operand & !second_operand,            // BIC
            0xf => !second_operand,                            // MVN
            _ => unreachable!(),
        };

        if s_bit == true {
            // Need to update the flags in the CPSR
            if write_val == 0 {
                self.cpsr.set(Z_FLAG, false)
            }
            if write_val & 0x80000000 != 0 {
                self.cpsr.set(N_FLAG, false);
                if rd != 15 {
                    // Update the V Flag
                    self.cpsr.set(V_FLAG, true);
                }
            } else {
                self.cpsr.set(N_FLAG, true);

                if rd != 15 {
                    // Update the V Flag
                    self.cpsr.set(V_FLAG, false);
                }
            }
        }
        self.set_register_arm(rd, write_val as u32);
    }

    fn handle_load_store(&mut self, instruction: u32) {
        println!("Load/store");
        println!("{:#b}", instruction);
        let immediate = (1 << 25) & instruction == 0;
        let pre_index = (1 << 24) & instruction != 0;
        let up_down_bit = (1 << 23) & instruction != 0;
        let is_word = (1 << 22) & instruction != 0;
        let write_back = (1 << 21) & instruction != 0;
        let is_load = (1 << 20) & instruction != 0;

        let rn = ((0xf << 16) & instruction) >> 16;
        let rd = ((0xf << 12) & instruction) >> 12;

        println!("GET RN, {:?}", rn);
        println!("GET RD, {:?}", rd);

        let offset = if immediate == true {
            0xfff & instruction
        } else {
            let shift = (0xff0 & instruction) >> 4;
            let tmp = self.get_register_arm(0xf & instruction);
            println!("GET REG {:#x}, VALUE {:#x}", 0xf & instruction, tmp);
            println!(
                "PERFORMING SHIFT {:#x}, AMOUNT {:?}, {:#x}",
                shift, tmp, self.pc
            );
            self.barrel_shift(shift, tmp)
        };

        let mut base = self.get_register_arm(rn);

        if pre_index == true {
            if up_down_bit == true {
                println!("BASE {:#x} ADD OFFSET {:#x}", base, offset);
                base = base.wrapping_add(offset);
            } else {
                println!("BASE {:#x} SUB OFFSET {:#x}", base, offset);
                base = base.wrapping_sub(offset);
            }

            if is_load == true {
                let val = if is_word {
                    self.mmu.read_32(base)
                } else {
                    self.mmu.read_byte(base) as u32
                };
                println!(
                    "{:#x}: Loading {:#x} from {:#x} into {:?}",
                    self.pc, val, base, rd
                );
                self.set_register_arm(rd, val);
            } else {
                // Store
                let mut val = self.get_register_arm(rd);
                if rd == 15 {
                    val += 4;
                }

                println!("{:#x}: Storing {:#x} from {:?}", self.pc, val, rd);
                if is_word {
                    self.mmu.write_32(base, val);
                } else {
                    self.mmu.write_byte(base, val as u8);
                }
            }

            if write_back {
                self.set_register_arm(rn, base);
            }
        } else {
            if is_load == true {
                let val = if is_word {
                    self.mmu.read_32(base)
                } else {
                    self.mmu.read_byte(base) as u32
                };
                println!("Loading {:#x} into {:?}", val, rd);
                self.set_register_arm(rd, val);
            } else {
                // Store
                let mut val = self.get_register_arm(rd);
                if rd == 15 {
                    val += 4;
                }
                println!("Storing {:#x} from {:?}", val, rd);
                if is_word {
                    self.mmu.write_32(base, val);
                } else {
                    self.mmu.write_byte(base, val as u8);
                }
            }

            if up_down_bit == true {
                println!("BASE {:#x} ADD OFFSET {:#x}", base, offset);
                base = base.wrapping_add(offset);
            } else {
                println!("BASE {:#x} SUB OFFSET {:#x}", base, offset);
                base = base.wrapping_sub(offset);
            }

            if write_back {
                self.set_register_arm(rn, base);
            }
        }
    }

    fn handle_load_store_multiple(&mut self, instruction: u32) {
        let pre_index = (1 << 24) & instruction != 0;
        let up_down_bit = (1 << 23) & instruction != 0;
        let psr_force_user = (1 << 22) & instruction != 0;
        let write_back = (1 << 21) & instruction != 0;
        let is_load = (1 << 20) & instruction != 0;

        let rn = ((0xf << 16) & instruction) >> 16;
        let rl = 0xffff & instruction;

        let mut base = self.get_register_arm(rn) as i32;

        for i in 0..16 {
            // println!("{:?} {:?} {:#x} {:#x}, {:#x}", is_load, pre_index, base, rl, (0x1 << i) & rl);
            if ((0x1 << i) & rl) != 0 {
                if pre_index {
                    base += if up_down_bit { 4 } else { -4 }
                }

                // Transfer register number i
                if is_load {
                    let val = self.mmu.read_32(base as u32);
                    println!("Setting {:?} to {:#x}", i, val);
                    self.set_register_arm(i, val);
                } else {
                    let mut val = self.get_register_arm(i);
                    if i == 15 {
                        val += 12;
                    }
                    println!("Loading {:#x} to {:#x}", val, base);
                    self.mmu.write_32(base as u32, val);
                }

                if !pre_index {
                    base += if up_down_bit { 4 } else { -4 }
                }
            }
        }

        if write_back {
            println!("Writing {:#x} to register {:?}", base, rn);
            self.set_register_arm(rn, base as u32);
        }
        println!("LDM/STM {:#x}", self.pc);
    }

    fn handle_branch_exchange(&mut self, instruction: u32) {
        // TODO: This can switch the processor over to THUMB MODE
        let val = self.get_register_arm(instruction & 0xf);
        println!("JUMP TO {:#x}", val);
        self.pc = val;
        if instruction & 1 != 0 {
            // Oh god we switched to THUMB mode
            self.instruction_mode = CPUInstructionMode::Thumb;
            self.word_size = 2;
        } else {
            self.instruction_mode = CPUInstructionMode::ARM32;
            self.word_size = 4;
        }
        self.pc += self.word_size;
    }

    fn handle_branch(&mut self, instruction: u32) {
        let raw_offset: u32 = ((0xffffff & instruction) << 2) as u32;
        let offset: i32 = if (1 << 23) & raw_offset != 0 {
            // Negative
            (!(raw_offset ^ 0xffffff + 1) + 1) as i32
        } else {
            raw_offset as i32
        };

        let link = (1 << 24) & instruction != 0;
        println!("{:#x}", offset);
        println!("{:?}", link);

        if link == true {
            let tmp = self.pc;
            self.set_register_arm(14, tmp);
        }

        self.pc = ((self.pc as i32) + offset) as u32;

        // The program counter gets incremented during a prefetch, so account for this
        self.pc += self.word_size;
        println!("Jump to {:#x}", self.pc);
    }

    fn handle_swap(&mut self, instruction: u32) {
        let is_word = 1 << 22 == 0;
        let rn = (0xf << 16) & instruction >> 16;
        let rd = (0xf << 12) & instruction >> 12;
        let rm = 0xf & instruction;

        let read_addr = self.get_register_arm(rn);
        let read_val = if is_word {
            self.mmu.read_32(read_addr)
        } else {
            self.mmu.read_byte(read_addr) as u32
        };

        let write_val = self.get_register_arm(rm);
        self.set_register_arm(rd, read_val);

        if is_word {
            self.mmu.write_32(read_addr, write_val);
        } else {
            self.mmu.write_byte(read_addr, write_val as u8);
        }
    }

    fn set_register_thumb(&mut self, register_num: u32, val: u16) {
        match register_num {
            0...7 => self.gen_registers[register_num as usize] = val as u32,
            _ => {
                let reg = register_num + 5;
                if reg == 15 {
                    self.pc = val as u32;
                    return;
                }

                match self.execution_mode {
                    CPUExecutionMode::User => self.hi_registers_user[reg as usize] = val as u32,
                    CPUExecutionMode::Supervisor => {
                        self.hi_registers_supervisor[reg as usize] = val as u32
                    }
                    CPUExecutionMode::IRQ => self.hi_registers_irq[reg as usize] = val as u32,
                    CPUExecutionMode::FIQ => self.hi_registers_fiq[reg as usize] = val as u32,
                    _ => unreachable!(),
                }
            }
        }
    }

    fn get_register_thumb(&mut self, register_num: u32) -> u32 {
        match register_num {
            0...7 => self.gen_registers[register_num as usize],
            _ => {
                let reg = register_num + 5;
                if reg == 15 {
                    return self.pc;
                }
                match self.execution_mode {
                    CPUExecutionMode::User => self.hi_registers_user[reg as usize],
                    CPUExecutionMode::Supervisor => self.hi_registers_supervisor[reg as usize],
                    CPUExecutionMode::IRQ => self.hi_registers_irq[reg as usize],
                    CPUExecutionMode::FIQ => self.hi_registers_fiq[reg as usize],
                    _ => unreachable!(),
                }
            }
        }
    }

    fn set_register_arm(&mut self, register_num: u32, val: u32) {
        match register_num {
            0...7 => self.gen_registers[register_num as usize] = val,
            15 => self.pc = val,
            _ => match self.execution_mode {
                CPUExecutionMode::User => self.hi_registers_user[(register_num - 8) as usize] = val,
                CPUExecutionMode::Supervisor => {
                    self.hi_registers_supervisor[(register_num - 8) as usize] = val
                }
                CPUExecutionMode::IRQ => self.hi_registers_irq[(register_num - 8) as usize] = val,
                CPUExecutionMode::FIQ => self.hi_registers_fiq[(register_num - 8) as usize] = val,
                _ => unreachable!(),
            },
        }
    }

    fn get_register_arm(&mut self, register_num: u32) -> u32 {
        match register_num {
            0...7 => self.gen_registers[register_num as usize],
            15 => self.pc,
            _ => match self.execution_mode {
                CPUExecutionMode::User => self.hi_registers_user[(register_num - 8) as usize],
                CPUExecutionMode::Supervisor => {
                    self.hi_registers_supervisor[(register_num - 8) as usize]
                }
                CPUExecutionMode::IRQ => self.hi_registers_irq[(register_num - 8) as usize],
                CPUExecutionMode::FIQ => self.hi_registers_fiq[(register_num - 8) as usize],
                _ => unreachable!(),
            },
        }
    }

    fn set_spsr_register(&mut self, val: u32) {
        match self.execution_mode {
            CPUExecutionMode::User => panic!("Asking for SPSR while in User mode? For shame"),
            CPUExecutionMode::Supervisor => {
                self.spsr_svc = bitvec_from_u32(val);
            }
            CPUExecutionMode::IRQ => {
                self.spsr_irq = bitvec_from_u32(val);
            }
            CPUExecutionMode::FIQ => {
                self.spsr_fiq = bitvec_from_u32(val);
            }
            _ => unreachable!(),
        }
    }

    fn get_spsr_register(&mut self) -> &mut BitVec<u32> {
        match self.execution_mode {
            CPUExecutionMode::User => panic!("Asking for SPSR while in User mode? For shame"),
            CPUExecutionMode::Supervisor => &mut self.spsr_svc,
            CPUExecutionMode::IRQ => &mut self.spsr_irq,
            CPUExecutionMode::FIQ => &mut self.spsr_fiq,
            _ => unreachable!(),
        }
    }

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        self.cpsr.set(0, true);
        self.cpsr.set(1, true);
        self.cpsr.set(4, true);

        self.pc = 0x08000000;
        let mut bp = self.pc;
        let mut pause: bool = false;

        loop {
            if pause || bp == self.pc {
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                match input.trim() {
                    "r" => {
                        println!("LO {:?}", self.gen_registers);
                        println!("HI {:?} {:#x}", self.hi_registers_supervisor, self.pc);
                        println!("CPSR {:?}", self.cpsr);
                        println!("SPSR_svc {:?}", self.spsr_svc);
                        pause = true;
                    }
                    "c" => {
                        pause = false;
                    }
                    value @ _ => {
                        let mut iter = value.split_whitespace();
                        let op = iter.next().unwrap();
                        let loc = iter.next().unwrap();
                        if op == "m" {
                            let words = loc.parse::<u32>().unwrap();
                            for i in 0..words {
                                println!("{:#x}", self.mmu.read_32(self.pc + i));
                            }
                            continue;
                        } else {
                            bp = loc.parse::<u32>().unwrap();
                            pause = false;
                        }
                    }
                }
            }

            println!("ADDRESS {:#x}", self.pc);
            println!("STACK POINTER {:#x}", self.get_register_arm(13));
            match self.instruction_mode {
                CPUInstructionMode::ARM32 => self.run_arm(),
                CPUInstructionMode::Thumb => self.run_thumb(),
            }
            if self.pc >= SYS_MAX_MEMORY {
                println!("{:?}", "BREAKING, OUT OF MEMORY");
                break;
            }
        }
        Ok(())
    }
}
