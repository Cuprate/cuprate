use std::cmp::max;

use blake;
use static_assertions::const_assert_eq;

const TOTAL_LATENCY: usize = 15 * 3;
const NUM_INSTRUCTIONS_MIN: usize = 60;
pub const NUM_INSTRUCTIONS_MAX: usize = 70;
const ALU_COUNT_MUL: usize = 1;
const ALU_COUNT: usize = 3;

#[derive(Debug, Clone, Copy)]
pub enum InstructionList {
    Mul, // a*b
    Add, // a+b + C, C is an unsigned 32-bit constant
    Sub, // a-b
    Ror, // rotate right "a" by "b & 31" bits
    Rol, // rotate left "a" by "b & 31" bits
    Xor, // a^b
    Ret, // finish execution
}

const INSTRUCTION_COUNT: usize = InstructionList::Ret as usize;

// These instruction bit constants are used to generate code from random data.
// Every random sequence of bytes is a valid code.
//
// There are 9 registers in total:
// - 4 variable registers
// - 5 constant registers initialized from loop variables
// This is why dst_index is 2 bits

const INSTRUCTION_OPCODE_BITS: usize = 3;
const INSTRUCTION_DST_INDEX_BITS: usize = 2;
const INSTRUCTION_SRC_INDEX_BITS: usize = 3;

const _: () = {
    const_assert_eq!(INSTRUCTION_COUNT, 6);
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Instruction {
    pub(crate) opcode: u8,
    pub(crate) dst_index: u8,
    pub(crate) src_index: u8,
    pub(crate) c: u32,
}

// If we don't have enough data available, generate more
fn check_data(data_index: &mut usize, bytes_needed: usize, data: &mut [i8]) {
    if *data_index + bytes_needed > data.len() {
        // SAFETY: data is a valid slice of i8, which is cast to a slice of u8 to be compatible
        // with the blake hash input. While it requires access to the raw pointer, it is
        // safe and preferable to copying to a new slice of u8.
        let mut data: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, data.len()) };
        let mut output = [0u8; 32];

        blake::hash(256, data, &mut output).expect("blake hash failed");
        data.copy_from_slice(&output);
        *data_index = 0;
    }
}

// Generates as many random math operations as possible with given latency and ALU restrictions
// "code" array must have space for NUM_INSTRUCTIONS_MAX+1 instructions
pub(crate) fn random_math_init(
    code: &mut [Instruction; NUM_INSTRUCTIONS_MAX + 1],
    height: u64,
) -> usize {
    // MUL is 3 cycles, 3-way addition and rotations are 2 cycles, SUB/XOR are 1 cycle
    // These latencies match real-life instruction latencies for Intel CPUs starting from Sandy Bridge and up to Skylake/Coffee lake
    //
    // AMD Ryzen has the same latencies except 1-cycle ROR/ROL, so it'll be a bit faster than Intel Sandy Bridge and newer processors
    // Surprisingly, Intel Nehalem also has 1-cycle ROR/ROL, so it'll also be faster than Intel Sandy Bridge and newer processors
    // AMD Bulldozer has 4 cycles latency for MUL (slower than Intel) and 1 cycle for ROR/ROL (faster than Intel), so average performance will be the same
    // Source: https://www.agner.org/optimize/instruction_tables.pdf
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

    let mut data = [0i8; 32];
    for (i, b) in height.to_le_bytes().iter().enumerate() {
        data[i] = *b as i8;
    }

    data[20] = -38; // change seed

    // Set data_index past the last byte in data
    // to trigger full data update with blake hash
    // before we start using it
    let mut data_index: usize = data.len();

    let mut code_size: usize = 0;

    // There is a small chance (1.8%) that register R8 won't be used in the generated program
    // So we keep track of it and try again if it's not used
    let mut r8_used = false;
    loop {
        let mut latency = [0usize; 9];
        let mut asic_latency = [0usize; 9];

        // Tracks previous instruction and value of the source operand for registers R0-R3 throughout code execution
        // byte 0: current value of the destination register
        // byte 1: instruction opcode
        // byte 2: current value of the source register
        //
        // Registers R4-R8 are constant and are treated as having the same value because when we do
        // the same operation twice with two constant source registers, it can be optimized into a single operation
        let mut inst_data: [usize; 9] =
            [0, 1, 2, 3, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF];

        let mut alu_busy = [[false; ALU_COUNT]; TOTAL_LATENCY + 1];
        let mut is_rotation = [false; INSTRUCTION_COUNT];
        is_rotation[InstructionList::Ror as usize] = true;
        is_rotation[InstructionList::Rol as usize] = true;
        let mut rotated = [false; 4];
        let mut rotate_count = 0usize;

        let mut num_retries = 0usize;
        code_size = 0;

        let mut total_iterations = 0usize;
        r8_used = false;

        // Generate random code to achieve minimal required latency for our abstract CPU
        // Try to get this latency for all 4 registers
        while ((latency[0] < TOTAL_LATENCY)
            || (latency[1] < TOTAL_LATENCY)
            || (latency[2] < TOTAL_LATENCY)
            || (latency[3] < TOTAL_LATENCY))
            && (num_retries < 64)
        {
            // Fail-safe to guarantee loop termination
            total_iterations += 1;
            if total_iterations > 256 {
                break;
            }

            check_data(&mut data_index, 1, &mut data);

            let c = data[data_index as usize] as u8;
            data_index += 1;

            // MUL = opcodes 0-2
            // ADD = opcode 3
            // SUB = opcode 4
            // ROR/ROL = opcode 5, shift direction is selected randomly
            // XOR = opcodes 6-7
            let mut opcode = c & ((1 << INSTRUCTION_OPCODE_BITS) - 1);
            if opcode == 5 {
                check_data(&mut data_index, 1, &mut data);
                opcode = if data[data_index as usize] >= 0 {
                    InstructionList::Ror as u8
                } else {
                    InstructionList::Rol as u8
                };
                data_index += 1;
            } else if opcode >= 6 {
                opcode = InstructionList::Xor as u8;
            } else {
                opcode = if opcode <= 2 {
                    InstructionList::Mul as u8
                } else {
                    opcode - 2
                };
            }

            let dst_index =
                (c >> INSTRUCTION_OPCODE_BITS) & ((1 << INSTRUCTION_DST_INDEX_BITS) - 1);
            let mut src_index = (c >> (INSTRUCTION_OPCODE_BITS + INSTRUCTION_DST_INDEX_BITS))
                & ((1 << INSTRUCTION_SRC_INDEX_BITS) - 1);

            let a = dst_index as usize;
            let mut b = src_index as usize;

            // Don't do ADD/SUB/XOR with the same register
            if ((opcode == InstructionList::Add as u8)
                || (opcode == InstructionList::Sub as u8)
                || (opcode == InstructionList::Xor as u8))
                && (a == b)
            {
                b = 8;
                src_index = 8;
            }

            // Don't do rotation with the same destination twice because it's equal to a single rotation
            if is_rotation[opcode as usize] && rotated[a] {
                continue;
            }

            // Don't do the same instruction (except MUL) with the same source value twice because all other cases can be optimized:
            // 2xADD(a, b, C) = ADD(a, b*2, C1+C2), same for SUB and rotations
            // 2xXOR(a, b) = NOP
            if (opcode != InstructionList::Mul as u8)
                && ((inst_data[a] & 0xFFFF00)
                    == ((opcode as usize) << 8) + ((inst_data[b] & 255) << 16))
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
                    if !alu_busy[next_latency][i] {
                        if (opcode == InstructionList::Add as u8) && alu_busy[next_latency + 1][i] {
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
                }
                if alu_index >= 0 {
                    break;
                }
                next_latency += 1;
            }

            // Don't generate instructions that leave some register unchanged for more than 7 cycles
            if next_latency > latency[a] + 7 {
                continue;
            }

            next_latency += OP_LATENCY[opcode as usize];

            if next_latency <= TOTAL_LATENCY {
                if is_rotation[opcode as usize] {
                    rotate_count += 1;
                }

                // Mark ALU as busy only for the first cycle when it starts executing the instruction because ALUs are fully pipelined
                alu_busy[next_latency - OP_LATENCY[opcode as usize]][alu_index as usize] = true;
                latency[a] = next_latency;

                // ASIC is supposed to have enough ALUs to run as many independent instructions per cycle as possible, so latency calculation for ASIC is simple
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

                if opcode == InstructionList::Add as u8 {
                    alu_busy[next_latency - OP_LATENCY[opcode as usize] + 1][alu_index as usize] =
                        true;

                    check_data(&mut data_index, std::mem::size_of::<u32>(), &mut data);
                    code[code_size].c = u32::from_le_bytes([
                        data[data_index] as u8,
                        data[data_index + 1] as u8,
                        data[data_index + 2] as u8,
                        data[data_index + 3] as u8,
                    ]);
                    data_index += std::mem::size_of::<u32>();
                }
                code_size += 1;
                if code_size >= NUM_INSTRUCTIONS_MIN {
                    break;
                }
            } else {
                num_retries += 1;
            }
        }

        // ASIC has more execution resources and can extract as much parallelism from the code as possible
        // We need to add a few more MUL and ROR instructions to achieve minimal required latency for ASIC
        // Get this latency for at least 1 of the 4 registers
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

            let pattern = [
                InstructionList::Ror as u8,
                InstructionList::Mul as u8,
                InstructionList::Mul as u8,
            ];
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

        //	// There is ~98.15% chance that loop condition is false, so this loop will execute only 1 iteration most of the time
        //	// It never does more than 4 iterations for all block heights < 10,000,000
        if r8_used && (code_size >= NUM_INSTRUCTIONS_MIN) && (code_size <= NUM_INSTRUCTIONS_MAX) {
            break;
        }
    }

    // It's guaranteed that NUM_INSTRUCTIONS_MIN <= code_size <= NUM_INSTRUCTIONS_MAX here
    // Add final instruction to stop the interpreter
    code[code_size].opcode = InstructionList::Ret as u8;
    code[code_size].dst_index = 0;
    code[code_size].src_index = 0;
    code[code_size].c = 0;

    code_size
}
