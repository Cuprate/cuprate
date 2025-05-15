use std::cmp::max;

use seq_macro::seq;
use InstructionList::{Add, Mul, Ret, Rol, Ror, Sub, Xor};

use crate::{
    blake256::{Blake256, Digest},
    util::subarray_copy,
};

const TOTAL_LATENCY: usize = 15 * 3;
const NUM_INSTRUCTIONS_MIN: usize = 60;
pub(crate) const NUM_INSTRUCTIONS_MAX: usize = 70;
const ALU_COUNT_MUL: usize = 1;
const ALU_COUNT: usize = 3;

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InstructionList {
    Mul, // a*b
    Add, // a+b + C, C is an unsigned 32-bit constant
    Sub, // a-b
    Ror, // rotate right "a" by "b & 31" bits
    Rol, // rotate left "a" by "b & 31" bits
    Xor, // a^b
    #[default]
    Ret, // finish execution
}

const INSTRUCTION_COUNT: usize = Ret as usize;

/// INSTRUCTION_* constants are used to generate code from random data.
/// Every random sequence of bytes is a valid code.
///
/// There are 9 registers in total:
/// - 4 variable registers
/// - 5 constant registers initialized from loop variables
const INSTRUCTION_OPCODE_BITS: usize = 3;
const INSTRUCTION_DST_INDEX_BITS: usize = 2;
const INSTRUCTION_SRC_INDEX_BITS: usize = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Instruction {
    pub(crate) opcode: InstructionList,
    pub(crate) dst_index: u8,
    pub(crate) src_index: u8,
    pub(crate) c: u32,
}

/// If we don't have enough data available, generate more.
/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/variant4_random_math.h#L171-L178>
fn check_data(data_index: &mut usize, bytes_needed: usize, data: &mut [u8]) {
    if *data_index + bytes_needed > data.len() {
        let output = Blake256::digest(&data);
        data.copy_from_slice(&output);
        *data_index = 0;
    }
}

/// Generates as many random math operations as possible with given latency and
/// ALU restrictions.
///
/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/variant4_random_math.h#L180-L439>
///
#[expect(clippy::cast_sign_loss)]
#[expect(clippy::cast_possible_wrap)]
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn random_math_init(
    code: &mut [Instruction; NUM_INSTRUCTIONS_MAX + 1],
    height: u64,
) -> usize {
    // MUL is 3 cycles, 3-way addition and rotations are 2 cycles, SUB/XOR are 1
    // cycle These latencies match real-life instruction latencies for Intel
    // CPUs starting from Sandy Bridge and up to Skylake/Coffee lake
    //
    // AMD Ryzen has the same latencies except 1-cycle ROR/ROL, so it'll be a bit
    // faster than Intel Sandy Bridge and newer processors Surprisingly, Intel
    // Nehalem also has 1-cycle ROR/ROL, so it'll also be faster than Intel Sandy
    // Bridge and newer processors AMD Bulldozer has 4 cycles latency for MUL
    // (slower than Intel) and 1 cycle for ROR/ROL (faster than Intel), so average
    // performance will be the same Source: https://www.agner.org/optimize/instruction_tables.pdf
    const OP_LATENCY: [usize; INSTRUCTION_COUNT] = [3, 2, 1, 2, 2, 1];

    // Instruction latencies for theoretical ASIC implementation
    const ASIC_OP_LATENCY: [usize; INSTRUCTION_COUNT] = [3, 1, 1, 1, 1, 1];

    // Available ALUs for each instruction
    const OP_ALUS: [usize; INSTRUCTION_COUNT] = [
        ALU_COUNT_MUL,
        ALU_COUNT,
        ALU_COUNT,
        ALU_COUNT,
        ALU_COUNT,
        ALU_COUNT,
    ];

    let mut data = [0_u8; 32];
    data[0..8].copy_from_slice(&height.to_le_bytes());

    data[20] = -38_i8 as u8; // change seed

    // Set data_index past the last byte in data
    // to trigger full data update with blake hash
    // before we start using it
    let mut data_index: usize = data.len();

    let mut code_size: usize;

    // There is a small chance (1.8%) that register R8 won't be used in the
    // generated program, so we keep track of it and try again if it's not used
    loop {
        let mut latency = [0_usize; 9];
        let mut asic_latency = [0_usize; 9];

        // Tracks previous instruction and value of the source operand for
        // registers R0-R3 throughout code execution:
        // byte 0: current value of the destination register
        // byte 1: instruction opcode
        // byte 2: current value of the source register
        //
        // Registers R4-R8 are constant and are treated as having the same
        // value, because when we do the same operation twice with two constant
        // source registers, it can be optimized into a single operation.
        let mut inst_data: [usize; 9] =
            [0, 1, 2, 3, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF];

        let mut alu_busy = [[false; ALU_COUNT]; TOTAL_LATENCY + 1];
        let mut is_rotation = [false; INSTRUCTION_COUNT];
        is_rotation[Ror as usize] = true;
        is_rotation[Rol as usize] = true;
        let mut rotated = [false; 4];
        let mut rotate_count = 0_usize;

        let mut num_retries = 0_usize;
        code_size = 0;

        let mut total_iterations = 0_usize;
        let mut r8_used = false;

        // Generate random code to achieve minimal required latency for our abstract CPU
        // Try to get this latency for all 4 registers
        while (latency[0] < TOTAL_LATENCY
            || latency[1] < TOTAL_LATENCY
            || latency[2] < TOTAL_LATENCY
            || latency[3] < TOTAL_LATENCY)
            && num_retries < 64
        {
            // Fail-safe to guarantee loop termination
            total_iterations += 1;
            if total_iterations > 256 {
                break;
            }

            check_data(&mut data_index, 1, &mut data);

            let c = data[data_index];
            data_index += 1;

            // MUL = opcodes 0-2
            // ADD = opcode 3
            // SUB = opcode 4
            // ROR/ROL = opcode 5, shift direction is selected randomly
            // XOR = opcodes 6-7
            let opcode_bits = c & ((1 << INSTRUCTION_OPCODE_BITS) - 1);
            let opcode: InstructionList;
            if opcode_bits == 5 {
                check_data(&mut data_index, 1, &mut data);
                opcode = if data[data_index] as i8 >= 0 {
                    Ror
                } else {
                    Rol
                };
                data_index += 1;
            } else if opcode_bits >= 6 {
                opcode = Xor;
            } else if opcode_bits <= 2 {
                opcode = Mul;
            } else {
                // remaining values are 3-4
                opcode = match opcode_bits {
                    3 => Add,
                    4 => Sub,
                    _ => unreachable!(),
                };
            }

            let dst_index =
                (c >> INSTRUCTION_OPCODE_BITS) & ((1 << INSTRUCTION_DST_INDEX_BITS) - 1);
            let mut src_index = (c >> (INSTRUCTION_OPCODE_BITS + INSTRUCTION_DST_INDEX_BITS))
                & ((1 << INSTRUCTION_SRC_INDEX_BITS) - 1);

            let a = dst_index as usize;
            let mut b = src_index as usize;

            // Don't do ADD/SUB/XOR with the same register
            if matches!(opcode, Add | Sub | Xor) && a == b {
                b = 8;
                src_index = 8;
            }

            // Don't do rotation with the same destination twice because it's equal to a
            // single rotation
            if is_rotation[opcode as usize] && rotated[a] {
                continue;
            }

            // Don't do the same instruction (except MUL) with the same source value twice,
            // because all other cases can be optimized:
            //      2xADD(a, b, C) = ADD(a,b*2, C1+C2),
            // Same for SUB and rotations:
            //      2xXOR(a, b) = NOP
            if opcode != Mul
                && inst_data[a] & 0xFFFF00
                    == ((opcode as usize) << 8) + ((inst_data[b] & 255) << 16)
            {
                continue;
            }

            // Find which ALU is available (and when) for this instruction
            let mut next_latency = if latency[a] > latency[b] {
                latency[a]
            } else {
                latency[b]
            };
            let mut alu_index = -1;
            while next_latency < TOTAL_LATENCY {
                for i in (0..OP_ALUS[opcode as usize]).rev() {
                    if alu_busy[next_latency][i] {
                        continue;
                    }

                    if opcode == Add && alu_busy[next_latency + 1][i] {
                        continue;
                    }

                    if is_rotation[opcode as usize]
                        && next_latency < (rotate_count * OP_LATENCY[opcode as usize])
                    {
                        continue;
                    }

                    alu_index = i as isize;
                    break;
                }
                if alu_index >= 0 {
                    break;
                }
                next_latency += 1;
            }

            // Don't generate instructions that leave some register unchanged for more than
            // 7 cycles
            if next_latency > latency[a] + 7 {
                continue;
            }

            next_latency += OP_LATENCY[opcode as usize];

            if next_latency <= TOTAL_LATENCY {
                if is_rotation[opcode as usize] {
                    rotate_count += 1;
                }

                // Mark ALU as busy only for the first cycle when it starts executing the
                // instruction because ALUs are fully pipelined.
                alu_busy[next_latency - OP_LATENCY[opcode as usize]][alu_index as usize] = true;
                latency[a] = next_latency;

                // ASIC is supposed to have enough ALUs to run as many independent instructions
                // per cycle as possible, so latency calculation for ASIC is straightforward.
                asic_latency[a] =
                    max(asic_latency[a], asic_latency[b]) + ASIC_OP_LATENCY[opcode as usize];

                rotated[a] = is_rotation[opcode as usize];

                inst_data[a] = code_size + ((opcode as usize) << 8) + ((inst_data[b] & 255) << 16);

                code[code_size].opcode = opcode;
                code[code_size].dst_index = dst_index;
                code[code_size].src_index = src_index;
                code[code_size].c = 0;

                if src_index == 8 {
                    r8_used = true;
                }

                if opcode == Add {
                    alu_busy[next_latency - OP_LATENCY[opcode as usize] + 1][alu_index as usize] =
                        true;

                    check_data(&mut data_index, size_of::<u32>(), &mut data);
                    code[code_size].c = u32::from_le_bytes(subarray_copy(&data, data_index));
                    data_index += 4;
                }
                code_size += 1;
                if code_size >= NUM_INSTRUCTIONS_MIN {
                    break;
                }
            } else {
                num_retries += 1;
            }
        }

        // ASIC has more execution resources and can extract as much parallelism
        // from the code as possible. We need to add a few more MUL and ROR
        // instructions to achieve minimal required latency for ASIC. Get this
        // latency for at least 1 of the 4 registers.
        let prev_code_size = code_size;
        while code_size < NUM_INSTRUCTIONS_MAX
            && asic_latency.iter().take(4).all(|&lat| lat < TOTAL_LATENCY)
        {
            let mut min_idx: usize = 0;
            let mut max_idx: usize = 0;
            for i in 1..4 {
                if asic_latency[i] < asic_latency[min_idx] {
                    min_idx = i;
                }
                if asic_latency[i] > asic_latency[max_idx] {
                    max_idx = i;
                }
            }

            let pattern = [Ror, Mul, Mul];
            let opcode = pattern[(code_size - prev_code_size) % 3];
            latency[min_idx] = latency[max_idx] + OP_LATENCY[opcode as usize];
            asic_latency[min_idx] = asic_latency[max_idx] + ASIC_OP_LATENCY[opcode as usize];

            code[code_size] = Instruction {
                opcode,
                dst_index: min_idx as u8,
                src_index: max_idx as u8,
                c: 0,
            };
            code_size += 1;
        }

        // There is ~98.15% chance that loop condition is false, so this loop will
        // execute only 1 iteration most of the time. It never does more than 4
        // iterations for all block heights < 10,000,000.

        if r8_used && (NUM_INSTRUCTIONS_MIN..=NUM_INSTRUCTIONS_MAX).contains(&code_size) {
            break;
        }
    }

    // It's guaranteed that NUM_INSTRUCTIONS_MIN <= code_size <=
    // NUM_INSTRUCTIONS_MAX here. Add final instruction to stop the interpreter.
    code[code_size].opcode = Ret;
    code[code_size].dst_index = 0;
    code[code_size].src_index = 0;
    code[code_size].c = 0;

    code_size
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/variant4_random_math.h#L81-L168>
#[expect(clippy::needless_return, reason = "last iteration of unrolled loop")]
#[expect(clippy::unnecessary_semicolon, reason = "macro")]
pub(crate) fn v4_random_math(code: &[Instruction; NUM_INSTRUCTIONS_MAX + 1], r: &mut [u32; 9]) {
    const REG_BITS: u32 = 32;

    debug_assert_eq!(NUM_INSTRUCTIONS_MAX, 70);
    seq!(i in 0..70 {
        let op = &code[i];
        let src = r[op.src_index as usize];
        let dst = &mut r[op.dst_index as usize];
        match op.opcode {
            Mul => *dst = dst.wrapping_mul(src),
            Add => *dst = dst.wrapping_add(src).wrapping_add(op.c),
            Sub => *dst = dst.wrapping_sub(src),
            Ror => *dst = dst.rotate_right(src % REG_BITS),
            Rol => *dst = dst.rotate_left(src % REG_BITS),
            Xor => *dst ^= src,
            Ret => return,
        }
    });
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L336-L370>
/// To match the C code organization, this function would be in `slow_hash.rs`, but
/// the test code for it is so large, that it was moved here.
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn variant4_random_math(
    a1: &mut u128,
    c2: &mut u128,
    r: &mut [u32; 9],
    b: &[u128; 2],
    code: &[Instruction; 71],
) {
    let t64 = u64::from(r[0].wrapping_add(r[1])) | (u64::from(r[2].wrapping_add(r[3])) << 32);
    *c2 ^= u128::from(t64);

    r[4] = *a1 as u32;
    r[5] = (*a1 >> 64) as u32;
    r[6] = b[0] as u32;
    r[7] = b[1] as u32;
    r[8] = (b[1] >> 64) as u32;

    v4_random_math(code, r);

    *a1 ^= u128::from(r[2])
        | (u128::from(r[3]) << 32)
        | (u128::from(r[0]) << 64)
        | (u128::from(r[1]) << 96);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::hex_to_array;

    #[rustfmt::skip]
    const CODE: [Instruction; 71] = [
        Instruction { opcode: Rol, dst_index: 0, src_index: 7, c: 0 },
        Instruction { opcode: Mul, dst_index: 3, src_index: 1, c: 0 },
        Instruction { opcode: Add, dst_index: 2, src_index: 7, c: 3553557725 },
        Instruction { opcode: Sub, dst_index: 0, src_index: 8, c: 0 },
        Instruction { opcode: Add, dst_index: 3, src_index: 4, c: 3590470404 },
        Instruction { opcode: Xor, dst_index: 1, src_index: 0, c: 0 },
        Instruction { opcode: Xor, dst_index: 1, src_index: 5, c: 0 },
        Instruction { opcode: Xor, dst_index: 1, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 7, c: 0 },
        Instruction { opcode: Mul, dst_index: 2, src_index: 1, c: 0 },
        Instruction { opcode: Mul, dst_index: 2, src_index: 4, c: 0 },
        Instruction { opcode: Mul, dst_index: 2, src_index: 7, c: 0 },
        Instruction { opcode: Sub, dst_index: 1, src_index: 8, c: 0 },
        Instruction { opcode: Add, dst_index: 0, src_index: 6, c: 1516169632 },
        Instruction { opcode: Add, dst_index: 2, src_index: 0, c: 1587456779 },
        Instruction { opcode: Mul, dst_index: 3, src_index: 5, c: 0 },
        Instruction { opcode: Mul, dst_index: 1, src_index: 0, c: 0 },
        Instruction { opcode: Xor, dst_index: 2, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Sub, dst_index: 3, src_index: 6, c: 0 },
        Instruction { opcode: Rol, dst_index: 3, src_index: 0, c: 0 },
        Instruction { opcode: Xor, dst_index: 2, src_index: 4, c: 0 },
        Instruction { opcode: Mul, dst_index: 3, src_index: 5, c: 0 },
        Instruction { opcode: Xor, dst_index: 2, src_index: 0, c: 0 },
        Instruction { opcode: Rol, dst_index: 2, src_index: 4, c: 0 },
        Instruction { opcode: Xor, dst_index: 3, src_index: 8, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 4, c: 0 },
        Instruction { opcode: Add, dst_index: 2, src_index: 3, c: 2235486112 },
        Instruction { opcode: Xor, dst_index: 0, src_index: 3, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 2, c: 0 },
        Instruction { opcode: Xor, dst_index: 2, src_index: 7, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 7, c: 0 },
        Instruction { opcode: Ror, dst_index: 0, src_index: 4, c: 0 },
        Instruction { opcode: Mul, dst_index: 3, src_index: 2, c: 0 },
        Instruction { opcode: Add, dst_index: 2, src_index: 3, c: 382729823 },
        Instruction { opcode: Mul, dst_index: 1, src_index: 4, c: 0 },
        Instruction { opcode: Sub, dst_index: 3, src_index: 5, c: 0 },
        Instruction { opcode: Add, dst_index: 3, src_index: 7, c: 446636115 },
        Instruction { opcode: Sub, dst_index: 0, src_index: 5, c: 0 },
        Instruction { opcode: Add, dst_index: 1, src_index: 8, c: 1136500848 },
        Instruction { opcode: Xor, dst_index: 3, src_index: 8, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 4, c: 0 },
        Instruction { opcode: Ror, dst_index: 3, src_index: 5, c: 0 },
        Instruction { opcode: Mul, dst_index: 2, src_index: 0, c: 0 },
        Instruction { opcode: Ror, dst_index: 0, src_index: 1, c: 0 },
        Instruction { opcode: Add, dst_index: 0, src_index: 7, c: 4221005163 },
        Instruction { opcode: Rol, dst_index: 0, src_index: 2, c: 0 },
        Instruction { opcode: Add, dst_index: 0, src_index: 7, c: 1789679560 },
        Instruction { opcode: Xor, dst_index: 0, src_index: 3, c: 0 },
        Instruction { opcode: Add, dst_index: 2, src_index: 8, c: 2725270475 },
        Instruction { opcode: Xor, dst_index: 1, src_index: 4, c: 0 },
        Instruction { opcode: Sub, dst_index: 3, src_index: 8, c: 0 },
        Instruction { opcode: Xor, dst_index: 3, src_index: 5, c: 0 },
        Instruction { opcode: Sub, dst_index: 3, src_index: 2, c: 0 },
        Instruction { opcode: Rol, dst_index: 2, src_index: 2, c: 0 },
        Instruction { opcode: Add, dst_index: 3, src_index: 6, c: 4110965463 },
        Instruction { opcode: Xor, dst_index: 2, src_index: 6, c: 0 },
        Instruction { opcode: Sub, dst_index: 2, src_index: 7, c: 0 },
        Instruction { opcode: Sub, dst_index: 3, src_index: 1, c: 0 },
        Instruction { opcode: Sub, dst_index: 1, src_index: 8, c: 0 },
        Instruction { opcode: Ror, dst_index: 1, src_index: 2, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 1, c: 0 },
        Instruction { opcode: Mul, dst_index: 2, src_index: 0, c: 0 },
        Instruction { opcode: Ret, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
        Instruction { opcode: Mul, dst_index: 0, src_index: 0, c: 0 },
    ];

    #[test]
    fn test1_variant4_random_math() {
        let mut a1 = u128::from_le_bytes(hex_to_array("969ecd223474a6bb3be76c637db7457b"));
        let mut c2 = u128::from_le_bytes(hex_to_array("dbd1f6404d4826c52f209951334e6ea7"));
        let mut r: [u32; 9] = [1336109178, 464004736, 1552145461, 3528897376, 0, 0, 0, 0, 0];
        let b_bytes: [u8; 32] =
            hex_to_array("8dfa6d2c82e1806367b844c15f0439ced99c9a4bae0badfb8a8cf8504b813b7d");
        let b: [u128; 2] = [
            u128::from_le_bytes(subarray_copy(&b_bytes, 0)),
            u128::from_le_bytes(subarray_copy(&b_bytes, 16)),
        ];

        variant4_random_math(&mut a1, &mut c2, &mut r, &b, &CODE);

        assert_eq!(
            hex::encode(a1.to_le_bytes()),
            "1cb6fe7738de9e764dd73ea37c438056"
        );
        assert_eq!(
            hex::encode(c2.to_le_bytes()),
            "215fbd2bd8c7fceb2f209951334e6ea7"
        );
        #[rustfmt::skip]
        assert_eq!(r, [
            3226611830, 767947777, 1429416074, 3443042828, 583900822, 1668081467, 745405069,
            1268423897, 1358466186
        ]);
    }

    #[test]
    fn test2_variant4_random_math() {
        let mut a1 = u128::from_le_bytes(hex_to_array("643955bde578c845e4898703c3ce5eaa"));
        let mut c2 = u128::from_le_bytes(hex_to_array("787e2613b8fd0a2dadad16d4ec189035"));
        let mut r: [u32; 9] = [
            3226611830, 767947777, 1429416074, 3443042828, 583900822, 1668081467, 745405069,
            1268423897, 1358466186,
        ];
        let b_bytes: [u8; 32] =
            hex_to_array("d4d1e70f7da4089ae53b2e7545e4242a8dfa6d2c82e1806367b844c15f0439ce");
        let b: [u128; 2] = [
            u128::from_le_bytes(subarray_copy(&b_bytes, 0)),
            u128::from_le_bytes(subarray_copy(&b_bytes, 16)),
        ];

        variant4_random_math(&mut a1, &mut c2, &mut r, &b, &CODE);

        assert_eq!(
            hex::encode(a1.to_le_bytes()),
            "c40cb4b3a3640a958cc919ccb4ff29e6"
        );
        assert_eq!(
            hex::encode(c2.to_le_bytes()),
            "0f5a3efd2e2f610fadad16d4ec189035"
        );
        #[rustfmt::skip]
        assert_eq!(r, [
            3483254888_u32, 1282879863, 249640352, 3502382150, 3176479076, 59214308, 266850772,
            745405069, 3242506343
        ]);
    }
}
