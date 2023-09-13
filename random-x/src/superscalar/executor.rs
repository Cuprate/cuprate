use crate::registers::RGroupRegisters;
use crate::superscalar::instructions::ScalarInstruction;

const P2EXP63: u64 = 1 << 63;

pub fn execute(program: &[ScalarInstruction], registers: &mut RGroupRegisters) {
    for instruction in program {
        match instruction {
            ScalarInstruction::ISUB_R { dst, src } => {
                let op = |dst_val: u64, src_val| dst_val.wrapping_sub(src_val);
                registers.apply_to_dst_with_src(dst, src, op);
            }
            ScalarInstruction::IXOR_R { dst, src } => {
                let op = |dst_val: u64, src_val| dst_val ^ src_val;
                registers.apply_to_dst_with_src(dst, src, op);
            }
            ScalarInstruction::IADD_RS {
                dst,
                src,
                mod_shift,
            } => {
                let op = |dst_val: u64, src_val| {
                    dst_val.wrapping_add(src_val << clamp_mod_shift(*mod_shift))
                };
                registers.apply_to_dst_with_src(dst, src, op);
            }
            ScalarInstruction::IMUL_R { dst, src } => {
                let op = |dst_val: u64, src_val| dst_val.wrapping_mul(src_val);
                registers.apply_to_dst_with_src(dst, src, op);
            }
            ScalarInstruction::IROR_C { dst, imm32 } => {
                let op = |dst_val: u64| dst_val.rotate_right(*imm32);
                registers.apply_to_dst(dst, op);
            }
            ScalarInstruction::IADD_C { dst, imm32 } => {
                let op = |dst_val: u64| dst_val.wrapping_add(sign_extend_2s_compl(*imm32));
                registers.apply_to_dst(dst, op);
            }
            ScalarInstruction::IXOR_C { dst, imm32 } => {
                let op = |dst_val: u64| dst_val ^ sign_extend_2s_compl(*imm32);
                registers.apply_to_dst(dst, op);
            }
            ScalarInstruction::IMULH_R { dst, src } => {
                registers.apply_to_dst_with_src(dst, src, high_mul);
            }
            ScalarInstruction::ISMULH_R { dst, src } => {
                let op = |dst_val: u64, src_val: u64| {
                    signed_high_mul(dst_val as i64, src_val as i64) as u64
                };
                registers.apply_to_dst_with_src(dst, src, op);
            }
            ScalarInstruction::IMUL_RCP { dst, imm32 } => {
                let op = |dst_val: u64| dst_val.wrapping_mul(randomx_reciprocal(*imm32 as u64));
                registers.apply_to_dst(dst, op);
            }
        }
    }
}

pub fn randomx_reciprocal(divisor: u64) -> u64 {
    assert!(!divisor.is_power_of_two());
    assert_ne!(divisor, 0);

    let mut quotient = P2EXP63 / divisor;
    let mut remainder = P2EXP63 % divisor;
    let mut bsr = 0;

    let mut bit = divisor;

    while bit > 0 {
        bsr += 1;
        bit >>= 1;
    }

    for _ in 0..bsr {
        if remainder >= divisor.wrapping_sub(remainder) {
            quotient = quotient.wrapping_mul(2).wrapping_add(1);
            remainder = remainder.wrapping_mul(2).wrapping_sub(divisor);
        } else {
            quotient = quotient.wrapping_mul(2);
            remainder = remainder.wrapping_mul(2);
        }
    }
    quotient
}

fn high_mul(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) >> 64) as u64
}

fn signed_high_mul(a: i64, b: i64) -> i64 {
    ((a as i128 * b as i128) >> 64) as i64
}

pub fn sign_extend_2s_compl(imm: u32) -> u64 {
    if imm > i32::MAX as u32 {
        imm as u64 | 0xffffffff00000000
    } else {
        imm as u64
    }
}

fn clamp_mod_shift(x: u8) -> u64 {
    (x as u64 >> 2) % 4
}
