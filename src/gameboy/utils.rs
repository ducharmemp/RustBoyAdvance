use bit_vec::BitVec;

pub fn bitvec_from_u32(value: u32) -> BitVec<u32> {
    let mut bv = BitVec::from_elem(32, false);
    for i in 0..32 {
        if value & (1 << i) != 0 {
            bv.set(i, true);
        }
    }
    bv
}

pub fn u32_from_bitvec(bv: &BitVec<u32>) -> u32 {
    let mut val = 0;
    let mut index = 0;
    for bit in bv {
        if bit == true {
            val |= 1 << index;
        }
        index += 1;
    }
    val
}
