// CPSR Flags
pub const N_FLAG: usize = 31;
pub const Z_FLAG: usize = 30;
pub const C_FLAG: usize = 29;
pub const V_FLAG: usize = 28;

pub const IRQ_DISABLE: usize = 7;
pub const FIQ_DISABLE: usize = 6;
pub const STATE_BIT: usize = 5;

// TODO: This is a slice
pub const MODE_BITS: u32 = 0xF;
