use std::cmp::Ordering;

use crate::config::SUPERSCALAR_MAX_SIZE;
use crate::registers::{RGroupRegisterID, RGroupRegisters};
use crate::superscalar::cpu::{MacroOp, ProgramSchedule, SlotLen};
use crate::superscalar::instructions::ScalarInstruction;
use crate::superscalar::SSProgram;
use crate::{
    blake2_generator::Blake2Generator,
    config::RANDOMX_SUPERSCALAR_LATENCY,
    is_0_or_power_of_2,
    superscalar::instructions::{OpSource, ScalarInstructionID},
};

const LOOK_FORWARD_CYCLES: usize = 4;
const MAX_THROWAWAY_COUNT: usize = 256;

/// Groups of 3 or 4 Macro-op slots that sum to 16
///
/// https://github.com/tevador/RandomX/blob/master/doc/specs.md#631-decoding-stage
/// table 6.3.1
#[derive(Eq, PartialEq, Copy, Clone)]
enum DecoderGroup {
    /// 0: 4-8-4
    D484,
    /// 1: 7-3-3-3
    D7333,
    /// 2: 3-7-3-3
    D3733,
    /// 3: 4-9-3
    D493,

    /// 4: 4-4-4-4
    D4444,
    /// 5: 3-3-10
    D3310,
}

impl DecoderGroup {
    fn slot_len(&self, index: usize) -> Option<SlotLen> {
        match self {
            DecoderGroup::D484 => match index {
                0 | 2 => Some(SlotLen::L4),
                1 => Some(SlotLen::L8),
                _ => None,
            },
            DecoderGroup::D7333 => match index {
                0 => Some(SlotLen::L7),
                1..=3 => Some(SlotLen::L3),
                _ => None,
            },
            DecoderGroup::D3733 => match index {
                0 | 2 | 3 => Some(SlotLen::L3),
                1 => Some(SlotLen::L7),
                _ => None,
            },
            DecoderGroup::D493 => match index {
                0 => Some(SlotLen::L4),
                1 => Some(SlotLen::L9),
                2 => Some(SlotLen::L3),
                _ => None,
            },
            DecoderGroup::D4444 => match index {
                0..=3 => Some(SlotLen::L4),
                _ => None,
            },
            DecoderGroup::D3310 => match index {
                0 | 1 => Some(SlotLen::L3),
                2 => Some(SlotLen::L10),
                _ => None,
            },
        }
    }

    /// Returns an iterator over the lengths with a bool `is_last`
    pub fn iter_slot_len(&self) -> impl Iterator<Item = (SlotLen, bool)> + '_ {
        (0..self.size()).map(|i| (self.slot_len(i).unwrap(), self.size() - 1 == i))
    }

    pub fn size(&self) -> usize {
        match self {
            DecoderGroup::D484 => 3,
            DecoderGroup::D7333 => 4,
            DecoderGroup::D3733 => 4,
            DecoderGroup::D493 => 3,
            DecoderGroup::D4444 => 4,
            DecoderGroup::D3310 => 3,
        }
    }

    fn next_group(
        gen: &mut Blake2Generator,
        instruction: Option<ScalarInstructionID>,
        total_muls_low: bool,
    ) -> DecoderGroup {
        if matches!(
            instruction,
            Some(ScalarInstructionID::IMULH_R) | Some(ScalarInstructionID::ISMULH_R)
        ) {
            return DecoderGroup::D3310;
        }

        if total_muls_low {
            return DecoderGroup::D4444;
        }

        if instruction == Some(ScalarInstructionID::IMUL_RCP) {
            return match (gen.next_u8() & 1).cmp(&1) {
                Ordering::Equal => DecoderGroup::D484,
                Ordering::Less => DecoderGroup::D493,
                Ordering::Greater => unreachable!(),
            };
        }

        match gen.next_u8() & 3 {
            0 => DecoderGroup::D484,
            1 => DecoderGroup::D7333,
            2 => DecoderGroup::D3733,
            3 => DecoderGroup::D493,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct SingleRegisterInfo {
    id: RGroupRegisterID,
    next_ready: usize,
    last_instruction: Option<ScalarInstructionID>,
    last_source: OpSource,
}

impl SingleRegisterInfo {
    pub fn id(&self) -> RGroupRegisterID {
        self.id
    }
    pub fn next_ready(&self) -> usize {
        self.next_ready
    }
    pub fn last_instruction(&self) -> Option<ScalarInstructionID> {
        self.last_instruction
    }
    pub fn last_source(&self) -> OpSource {
        self.last_source
    }
    pub fn set_next_ready(&mut self, next_ready: usize) {
        self.next_ready = next_ready
    }
    pub fn set_last_instruction(&mut self, last_instruction: ScalarInstructionID) {
        self.last_instruction = Some(last_instruction);
    }
    pub fn set_last_source(&mut self, last_source: OpSource) {
        self.last_source = last_source
    }
}

#[derive(Debug)]
pub(crate) struct RegistersInfo {
    registers: [SingleRegisterInfo; 8],
}

impl Default for RegistersInfo {
    fn default() -> Self {
        let default = SingleRegisterInfo {
            id: RGroupRegisterID::R0,
            next_ready: 0,
            last_instruction: None,
            last_source: OpSource::Constant,
        };
        let mut default = [default; 8];
        let reg_ids = [
            RGroupRegisterID::R1,
            RGroupRegisterID::R2,
            RGroupRegisterID::R3,
            RGroupRegisterID::R4,
            RGroupRegisterID::R5,
            RGroupRegisterID::R6,
            RGroupRegisterID::R7,
        ];
        for (reg, id) in default.iter_mut().skip(1).zip(reg_ids) {
            reg.id = id;
        }
        RegistersInfo { registers: default }
    }
}

impl RegistersInfo {
    pub fn iter(&self) -> impl Iterator<Item = &SingleRegisterInfo> {
        self.registers.iter()
    }
    pub fn ready_at_cycle(&self, cycle: usize) -> Vec<&SingleRegisterInfo> {
        self.registers
            .iter()
            .filter(|reg| reg.next_ready <= cycle)
            .collect::<Vec<_>>()
    }
    pub fn get_mut(&mut self, id: RGroupRegisterID) -> &mut SingleRegisterInfo {
        &mut self.registers[id as usize]
    }
}

pub(crate) fn select_register(
    gen: &mut Blake2Generator,
    available: &[&SingleRegisterInfo],
) -> Option<RGroupRegisterID> {
    if available.is_empty() {
        return None;
    }
    let index = if available.len() > 1 {
        // available is <= 8 so as is safe
        (gen.next_u32() % available.len() as u32)
            .try_into()
            .expect("Could not fit u32 into usize")
    } else {
        0
    };

    Some(available[index].id)
}

/// Returns an imm32 if the instruction requires one.
fn get_imm32(gen: &mut Blake2Generator, id: &ScalarInstructionID) -> Option<u32> {
    match id {
        ScalarInstructionID::IADD_C | ScalarInstructionID::IXOR_C => Some(gen.next_u32()),
        ScalarInstructionID::IROR_C => {
            // imm32 % 64 != 0
            Some(
                loop {
                    let imm8 = gen.next_u8() & 63;
                    if imm8 != 0 {
                        break imm8;
                    }
                }
                .into(),
            )
        }
        ScalarInstructionID::IMUL_RCP => {
            // imm32 != 0, imm32 != 2N
            Some(loop {
                let imm32 = gen.next_u32();
                if !is_0_or_power_of_2(imm32.into()) {
                    break imm32;
                }
            })
        }
        _ => None,
    }
}

fn get_mod_shift(gen: &mut Blake2Generator, id: &ScalarInstructionID) -> Option<u8> {
    match id {
        ScalarInstructionID::IADD_RS => Some(gen.next_u8()),
        _ => None,
    }
}

/// Used during [`ScalarInstructionBuilder`] creation. Returns the [`OpSource`] to give the register
/// if this is known otherwise [`None`] is returned and this field will be filled later.
fn get_src_to_give_register(
    gen: &mut Blake2Generator,
    id: &ScalarInstructionID,
) -> Option<OpSource> {
    match id {
        ScalarInstructionID::IADD_C
        | ScalarInstructionID::IXOR_C
        | ScalarInstructionID::IROR_C
        | ScalarInstructionID::IMUL_RCP => Some(OpSource::Constant),
        ScalarInstructionID::IMULH_R | ScalarInstructionID::ISMULH_R => {
            // not actually the source value, the Monero C++ version sets this field to a random
            // value, this has an issue of becoming an actual meaningful value though so we handle
            // those rare cases here:
            Some(OpSource::from_rand_i32(gen.next_u32() as i32))
        }
        _ => None,
    }
}

struct ScalarInstructionBuilder {
    /// The id of the instruction we are building.
    id: ScalarInstructionID,
    /// The true source register - the one we are actually getting the value from will be
    /// None if this instruction doesn't need a register source.
    true_src: Option<RGroupRegisterID>,
    /// The value src we tell the dst register, if this is a register then most of the time this
    /// is the same as [`true_src`] but for `IMULH_R` and `ISMULH_R` it's not.
    ///
    /// `IMULH_R` and `ISMULH_R` generate a random i32 and set it for this slot .
    src_to_give_register: Option<OpSource>,
    /// The destination register for this instruction.
    dst: Option<RGroupRegisterID>,
    /// A constant used in some instructions.
    imm32: Option<u32>,
    /// used in IADD_RS
    mod_shift: Option<u8>,
}

impl ScalarInstructionBuilder {
    /// Creates a new [`ScalarInstructionBuilder`].
    ///
    pub fn new(
        gen: &mut Blake2Generator,
        slot_len: &SlotLen,
        group: &DecoderGroup,
        is_last: bool,
    ) -> Self {
        // https://github.com/tevador/RandomX/blob/master/doc/specs.md#632-instruction-selection
        let id = match slot_len {
            SlotLen::L3 if !is_last => match gen.next_u8() & 1 {
                0 => ScalarInstructionID::ISUB_R,
                _ => ScalarInstructionID::IXOR_R,
            },
            SlotLen::L3 => match gen.next_u8() & 3 {
                0 => ScalarInstructionID::ISUB_R,
                1 => ScalarInstructionID::IXOR_R,
                2 => ScalarInstructionID::IMULH_R,
                _ => ScalarInstructionID::ISMULH_R,
            },
            SlotLen::L4 if group == &DecoderGroup::D4444 && !is_last => ScalarInstructionID::IMUL_R,
            SlotLen::L4 => match gen.next_u8() & 1 {
                0 => ScalarInstructionID::IROR_C,
                _ => ScalarInstructionID::IADD_RS,
            },
            SlotLen::L7 | SlotLen::L8 | SlotLen::L9 => match gen.next_u8() & 1 {
                0 => ScalarInstructionID::IXOR_C,
                _ => ScalarInstructionID::IADD_C,
            },
            SlotLen::L10 => ScalarInstructionID::IMUL_RCP,
        };

        Self {
            id,
            true_src: None,
            src_to_give_register: get_src_to_give_register(gen, &id),
            dst: None,
            imm32: get_imm32(gen, &id),
            mod_shift: get_mod_shift(gen, &id),
        }
    }

    /// Set the source of the operation
    fn set_src(&mut self, src: RGroupRegisterID) {
        self.true_src = Some(src);
        if self.src_to_give_register.is_none() {
            // If the src_to_give_register field hasn't already been set then set it now.
            // The only fields that have true_src as a register with a different src_to_give_register
            // set this field at the start.
            self.src_to_give_register = Some(OpSource::Register(src));
        }
    }

    /// Select the source of this operation from the given registers.
    ///
    /// If no registers are available [`false`] is returned.
    pub fn select_source(
        &mut self,
        gen: &mut Blake2Generator,
        cycle: usize,
        registers_info: &RegistersInfo,
    ) -> bool {
        let available_registers = registers_info.ready_at_cycle(cycle);
        //if there are only 2 available registers for IADD_RS and one of them is r5, select it as the source because it cannot be the destination
        if available_registers.len() == 2
            && self.id == ScalarInstructionID::IADD_RS
            && (available_registers[0].id() == RGroupRegisterID::R5
                || available_registers[1].id() == RGroupRegisterID::R5)
        {
            self.set_src(RGroupRegisterID::R5);
            return true;
        }
        if let Some(reg) = select_register(gen, &available_registers) {
            self.set_src(reg);
            return true;
        };

        false
    }

    /// Selects the destination of this operation from the given registers.
    ///
    /// If no registers are available [`false`] is returned.
    fn select_destination(
        &mut self,
        gen: &mut Blake2Generator,
        cycle: usize,
        allow_chain_mul: bool,
        registers_info: &RegistersInfo,
    ) -> bool {
        let available_registers = registers_info
            .iter()
            .filter(|reg| {
                reg.next_ready() <= cycle
                    && (self.id.can_dst_be_src() || Some(reg.id()) != self.true_src)
                    && (allow_chain_mul
                        || self.id.instruction_group() != ScalarInstructionID::IMUL_R
                        || reg.last_instruction() != Some(ScalarInstructionID::IMUL_R))
                    && (Some(self.id.instruction_group()) != reg.last_instruction()
                        || self.src_to_give_register != Some(reg.last_source()))
                    && (reg.id() != RGroupRegisterID::R5 || self.id != ScalarInstructionID::IADD_RS)
            })
            .collect::<Vec<_>>();
        let Some(reg) = select_register(gen, &available_registers) else {
            return false;
        };
        self.dst = Some(reg);
        true
    }

    fn construct(self) -> ScalarInstruction {
        match self.id {
            ScalarInstructionID::ISUB_R => ScalarInstruction::ISUB_R {
                dst: self.dst.unwrap(),
                src: self.true_src.unwrap(),
            },
            ScalarInstructionID::IXOR_R => ScalarInstruction::IXOR_R {
                dst: self.dst.unwrap(),
                src: self.true_src.unwrap(),
            },
            ScalarInstructionID::IADD_RS => ScalarInstruction::IADD_RS {
                dst: self.dst.unwrap(),
                src: self.true_src.unwrap(),
                mod_shift: self.mod_shift.unwrap(),
            },
            ScalarInstructionID::IMUL_R => ScalarInstruction::IMUL_R {
                dst: self.dst.unwrap(),
                src: self.true_src.unwrap(),
            },
            ScalarInstructionID::IROR_C => ScalarInstruction::IROR_C {
                dst: self.dst.unwrap(),
                imm32: self.imm32.unwrap(),
            },
            ScalarInstructionID::IADD_C => ScalarInstruction::IADD_C {
                dst: self.dst.unwrap(),
                imm32: self.imm32.unwrap(),
            },
            ScalarInstructionID::IXOR_C => ScalarInstruction::IXOR_C {
                dst: self.dst.unwrap(),
                imm32: self.imm32.unwrap(),
            },
            ScalarInstructionID::IMULH_R => ScalarInstruction::IMULH_R {
                dst: self.dst.unwrap(),
                src: self.true_src.unwrap(),
            },
            ScalarInstructionID::ISMULH_R => ScalarInstruction::ISMULH_R {
                dst: self.dst.unwrap(),
                src: self.true_src.unwrap(),
            },
            ScalarInstructionID::IMUL_RCP => ScalarInstruction::IMUL_RCP {
                dst: self.dst.unwrap(),
                imm32: self.imm32.unwrap(),
            },
        }
    }
}

#[derive(Debug, Default)]
struct ProgramState {
    /// The current cycle we are generating for.
    current_cycle: usize,
    /// The cycle the last operation will complete at.
    last_op_completes_at: usize,

    /// The amount of multiplication instructions the program
    /// has generated.
    mul_count: usize,
    /// The amount of instructions in a row the program has thrown
    /// away because they couldn't be completed.
    throw_away_count: usize,
    /// The execution port schedule of the program.
    program_schedule: ProgramSchedule,
    /// Information on the registers state.
    registers_info: RegistersInfo,
    /// The program
    program: Vec<ScalarInstruction>,
}

impl ProgramState {
    fn allow_chain_mul(&self) -> bool {
        self.throw_away_count > 0
    }
}

/// A state machine that controls instruction generation.
enum ScalarInstructionBuilderSM {
    /// The generate instruction state, the next call will
    /// start a new instruction.
    Generate {
        /// The last instruction generated.
        last_instruction: Option<ScalarInstructionID>,
    },
    /// A partially completed instruction, the next call will
    /// push this instruction forward.
    PartiallyComplete {
        /// The instruction currently being generated.
        builder: ScalarInstructionBuilder,
        /// The macro op of the instruction we are going
        /// to do next.
        macro_op_idx: usize,
    },
    /// NULL state, this state will only be finished on is the program is full.
    NULL,
}

impl ScalarInstructionBuilderSM {
    pub fn push_forward(
        &mut self,
        gen: &mut Blake2Generator,
        decoder_group: &DecoderGroup,
        slot_len: &SlotLen,
        is_last_slot: bool,
        program_state: &mut ProgramState,
    ) {
        loop {
            match std::mem::replace(self, ScalarInstructionBuilderSM::NULL) {
                ScalarInstructionBuilderSM::NULL => {
                    return;
                }
                ScalarInstructionBuilderSM::Generate { .. } => {
                    if program_state.program_schedule.is_full()
                        || program_state.program.len() >= SUPERSCALAR_MAX_SIZE
                    {
                        return;
                    }

                    let builder =
                        ScalarInstructionBuilder::new(gen, slot_len, decoder_group, is_last_slot);

                    *self = ScalarInstructionBuilderSM::PartiallyComplete {
                        builder,
                        macro_op_idx: 0,
                    };
                }
                ScalarInstructionBuilderSM::PartiallyComplete {
                    mut builder,
                    mut macro_op_idx,
                } => {
                    let top_cycle = program_state.current_cycle;

                    if macro_op_idx >= builder.id.number_of_macro_ops() {
                        *self = ScalarInstructionBuilderSM::Generate {
                            last_instruction: Some(builder.id),
                        };
                        continue;
                    }

                    let Some(next_macro_op) = builder.id.macro_op(macro_op_idx) else {
                        unreachable!("We just checked if the macro op idx is too high")
                    };

                    let Some(opportunity) =
                        program_state.program_schedule.earliest_cycle_for_macro_op(
                            &next_macro_op,
                            program_state.current_cycle,
                            program_state.last_op_completes_at,
                        )
                    else {
                        program_state.program_schedule.set_full();
                        return;
                    };

                    let mut scheduled_cycle = opportunity.cycle();

                    if !Self::check_set_src(
                        &mut builder,
                        macro_op_idx,
                        gen,
                        &mut scheduled_cycle,
                        &mut program_state.current_cycle,
                        &program_state.registers_info,
                    ) {
                        // If the source couldn't be set throw the instruction away
                        if program_state.throw_away_count < MAX_THROWAWAY_COUNT {
                            program_state.throw_away_count += 1;
                            *self = ScalarInstructionBuilderSM::Generate {
                                last_instruction: Some(builder.id),
                            };
                            continue;
                        }
                        // If too many instructions are thrown away return for the next decoder
                        // idx
                        *self = ScalarInstructionBuilderSM::Generate {
                            last_instruction: None,
                        };
                        return;
                    }

                    let allow_chain_mul = program_state.allow_chain_mul();

                    if !Self::check_set_dst(
                        &mut builder,
                        macro_op_idx,
                        gen,
                        &mut scheduled_cycle,
                        &mut program_state.current_cycle,
                        allow_chain_mul,
                        &program_state.registers_info,
                    ) {
                        // If the source couldn't be set throw the instruction away
                        if program_state.throw_away_count < MAX_THROWAWAY_COUNT {
                            program_state.throw_away_count += 1;
                            *self = ScalarInstructionBuilderSM::Generate {
                                last_instruction: Some(builder.id),
                            };
                            continue;
                        }
                        // If too many instructions are thrown away return for the next decoder
                        // idx
                        *self = ScalarInstructionBuilderSM::Generate {
                            last_instruction: None,
                        };
                        return;
                    }

                    program_state.throw_away_count = 0;

                    let Some(scheduled_cycle) = program_state
                        .program_schedule
                        .schedule_macro_op_at_earliest(
                            &next_macro_op,
                            scheduled_cycle,
                            program_state.last_op_completes_at,
                        )
                    else {
                        program_state.program_schedule.set_full();
                        return;
                    };

                    let completes_at = scheduled_cycle + next_macro_op.cycles_to_complete();
                    program_state.last_op_completes_at = completes_at;

                    if macro_op_idx == builder.id.macro_op_to_store_res() {
                        let reg = program_state.registers_info.get_mut(builder.dst.unwrap());
                        reg.set_next_ready(completes_at);
                        reg.set_last_source(builder.src_to_give_register.unwrap());
                        reg.set_last_instruction(builder.id.instruction_group());
                    }

                    macro_op_idx += 1;
                    program_state.current_cycle = top_cycle;

                    if scheduled_cycle >= RANDOMX_SUPERSCALAR_LATENCY {
                        program_state.program_schedule.set_full();
                    }

                    if macro_op_idx >= builder.id.number_of_macro_ops() {
                        if builder.id.is_multiplication() {
                            program_state.mul_count += 1;
                        }
                        *self = ScalarInstructionBuilderSM::Generate {
                            last_instruction: Some(builder.id),
                        };
                        program_state.program.push(builder.construct());
                    } else {
                        *self = ScalarInstructionBuilderSM::PartiallyComplete {
                            builder,
                            macro_op_idx,
                        };
                    }
                    return;
                }
            }
        }
    }

    /// Try set the instructions source.
    ///
    /// Will return true if the src has been set or if its not the correct macro op to set the dst.
    ///
    /// Will return false if its the correct macro op to set the dst and the src couldn't be set.
    fn check_set_dst(
        builder: &mut ScalarInstructionBuilder,
        macro_op_idx: usize,
        gen: &mut Blake2Generator,
        scheduled_cycle: &mut usize,
        cycle: &mut usize,
        allow_chain_mul: bool,
        registers_info: &RegistersInfo,
    ) -> bool {
        if builder.id.macro_op_to_select_dst() != macro_op_idx {
            // We don't need to set the src at this macro op.
            return true;
        }

        let mut set = false;
        for _ in 0..LOOK_FORWARD_CYCLES {
            if !builder.select_destination(gen, *scheduled_cycle, allow_chain_mul, &registers_info)
            {
                *scheduled_cycle += 1;
                *cycle += 1;
            } else {
                set = true;
                break;
            }
        }

        set
    }

    /// Try set the instructions source.
    ///
    /// Will return true if the src has been set or if its not he correct macro op to set the src.
    ///
    /// Will return false if its the correct macro op to set the src and the src couldn't be set.
    fn check_set_src(
        builder: &mut ScalarInstructionBuilder,
        macro_op_idx: usize,
        gen: &mut Blake2Generator,
        scheduled_cycle: &mut usize,
        cycle: &mut usize,
        registers_info: &RegistersInfo,
    ) -> bool {
        if builder.id.macro_op_to_select_src() != Some(macro_op_idx) {
            // We don't need to set the src at this macro op.
            return true;
        }

        let mut set = false;
        for _ in 0..LOOK_FORWARD_CYCLES {
            if !builder.select_source(gen, *scheduled_cycle, registers_info) {
                *scheduled_cycle += 1;
                *cycle += 1;
            } else {
                set = true;
                break;
            }
        }

        set
    }

    pub fn get_instruction_id(&self) -> Option<ScalarInstructionID> {
        match self {
            ScalarInstructionBuilderSM::Generate { last_instruction } => *last_instruction,
            ScalarInstructionBuilderSM::PartiallyComplete { builder, .. } => Some(builder.id),
            ScalarInstructionBuilderSM::NULL => {
                panic!("Should not be calling this function in this state")
            }
        }
    }
}

pub(crate) fn generate(gen: &mut Blake2Generator) -> SSProgram {
    let mut program_state = ProgramState::default();

    let mut instruction_sm = ScalarInstructionBuilderSM::Generate {
        last_instruction: None,
    };

    for decoder_cycle in 0..RANDOMX_SUPERSCALAR_LATENCY {
        if program_state.program_schedule.is_full()
            || program_state.program.len() >= SUPERSCALAR_MAX_SIZE
        {
            break;
        }
        let current_decode_group = DecoderGroup::next_group(
            gen,
            instruction_sm.get_instruction_id(),
            program_state.mul_count < decoder_cycle + 1,
        );

        for (slot_len, is_last) in current_decode_group.iter_slot_len() {
            instruction_sm.push_forward(
                gen,
                &current_decode_group,
                &slot_len,
                is_last,
                &mut program_state,
            );
        }
        program_state.current_cycle += 1;
    }

    //Calculate ASIC latency:
    //Assumes 1 cycle latency for all operations and unlimited parallelization.
    let mut asic_latencies = RGroupRegisters::default();
    for instr in program_state.program.iter() {
        let mut latency_dst = asic_latencies.get(&instr.dst());
        latency_dst += 1;
        let latency_src = if let Some(src) = instr.src() {
            asic_latencies.get(&src) + 1
        } else {
            0
        };
        asic_latencies.set(&instr.dst(), latency_src.max(latency_dst));
    }

    let mut reg_with_max_latency = RGroupRegisterID::R0;
    for reg in RGroupRegisterID::iter().skip(1) {
        if asic_latencies.get(&reg) > asic_latencies.get(&reg_with_max_latency) {
            reg_with_max_latency = reg
        }
    }

    SSProgram {
        program: program_state.program,
        reg_with_max_latency,
    }
}
