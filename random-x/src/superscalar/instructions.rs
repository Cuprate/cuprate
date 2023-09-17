use crate::registers::RGroupRegisterID;
use crate::superscalar::cpu::MacroOp;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub enum ScalarInstructionID {
    /// dst = dst - src
    ISUB_R,
    /// dst = dst ^ src
    IXOR_R,
    /// dst = dst + (src << mod_shift)
    IADD_RS,
    /// dst = dst * src
    IMUL_R,
    /// dst = dst >>> imm32
    IROR_C,
    /// dst = dst + imm32
    IADD_C,
    /// dst = dst ^ imm32
    IXOR_C,
    /// dst = (dst * src) >> 64
    IMULH_R,
    /// dst = (dst * src) >> 64 (signed)
    ISMULH_R,
    /// dst = 2x / imm32 * dst
    IMUL_RCP,
}

impl ScalarInstructionID {
    pub fn macro_op_to_select_src(&self) -> Option<usize> {
        match self {
            ScalarInstructionID::ISUB_R
            | ScalarInstructionID::IXOR_R
            | ScalarInstructionID::IADD_RS
            | ScalarInstructionID::IMUL_R => Some(0),
            ScalarInstructionID::IROR_C
            | ScalarInstructionID::IADD_C
            | ScalarInstructionID::IXOR_C => None,
            ScalarInstructionID::IMULH_R | ScalarInstructionID::ISMULH_R => Some(1),
            ScalarInstructionID::IMUL_RCP => None,
        }
    }

    pub fn macro_op_to_select_dst(&self) -> usize {
        match self {
            ScalarInstructionID::IMUL_RCP => 1,
            _ => 0,
        }
    }

    pub fn macro_op_to_store_res(&self) -> usize {
        match self {
            ScalarInstructionID::IMULH_R
            | ScalarInstructionID::ISMULH_R
            | ScalarInstructionID::IMUL_RCP => 1,
            _ => 0,
        }
    }

    pub fn is_multiplication(&self) -> bool {
        matches!(
            self,
            ScalarInstructionID::IMUL_R
                | ScalarInstructionID::IMULH_R
                | ScalarInstructionID::ISMULH_R
                | ScalarInstructionID::IMUL_RCP
        )
    }
    /// is the destination allowed to be the same as the source
    pub fn can_dst_be_src(&self) -> bool {
        matches!(
            self,
            ScalarInstructionID::IMULH_R | ScalarInstructionID::ISMULH_R
        )
    }

    /// Returns the group of this operation.
    ///
    /// A group is related instructions that effect register choice during program construction.
    pub fn instruction_group(&self) -> ScalarInstructionID {
        match self {
            // The only 2 instructions in the same group is ISUB_R & IADD_RS
            // We could make group an enum but for just these 2 i don't think
            // it's worth it.
            ScalarInstructionID::ISUB_R => ScalarInstructionID::IADD_RS,
            id => *id,
        }
    }

    pub fn number_of_macro_ops(&self) -> usize {
        match self {
            ScalarInstructionID::ISUB_R
            | ScalarInstructionID::IXOR_R
            | ScalarInstructionID::IADD_RS
            | ScalarInstructionID::IMUL_R
            | ScalarInstructionID::IROR_C
            | ScalarInstructionID::IADD_C
            | ScalarInstructionID::IXOR_C => 1,
            ScalarInstructionID::IMULH_R | ScalarInstructionID::ISMULH_R => 3,
            ScalarInstructionID::IMUL_RCP => 2,
        }
    }

    pub fn macro_op(&self, i: usize) -> Option<MacroOp> {
        Some(match self {
            ScalarInstructionID::ISUB_R => MacroOp::SUB_RR,
            ScalarInstructionID::IXOR_R => MacroOp::XOR_RR,
            ScalarInstructionID::IADD_RS => MacroOp::LEA_SIB,
            ScalarInstructionID::IMUL_R => MacroOp::IMUL_RR { dependant: false },
            ScalarInstructionID::IROR_C => MacroOp::ROR_RI,
            ScalarInstructionID::IADD_C => MacroOp::ADD_RI,
            ScalarInstructionID::IXOR_C => MacroOp::XOR_RI,
            ScalarInstructionID::IMULH_R => match i {
                0 => MacroOp::MOV_RR,
                1 => MacroOp::MUL_R,
                2 => MacroOp::MOV_RR,
                _ => return None,
            },
            ScalarInstructionID::ISMULH_R => match i {
                0 => MacroOp::MOV_RR,
                1 => MacroOp::IMUL_R,
                2 => MacroOp::MOV_RR,
                _ => return None,
            },
            ScalarInstructionID::IMUL_RCP => match i {
                0 => MacroOp::MOV_RI,
                1 => MacroOp::IMUL_RR { dependant: true },
                _ => return None,
            },
        })
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum ScalarInstruction {
    /// dst = dst - src
    ISUB_R {
        dst: RGroupRegisterID,
        src: RGroupRegisterID,
    },
    /// dst = dst ^ src
    IXOR_R {
        dst: RGroupRegisterID,
        src: RGroupRegisterID,
    },
    /// dst = dst + (src << mod_shift)
    IADD_RS {
        dst: RGroupRegisterID,
        src: RGroupRegisterID,
        mod_shift: u8,
    },
    /// dst = dst * src
    IMUL_R {
        dst: RGroupRegisterID,
        src: RGroupRegisterID,
    },
    /// dst = dst >>> imm32
    IROR_C { dst: RGroupRegisterID, imm32: u32 },
    /// dst = dst + imm32
    IADD_C { dst: RGroupRegisterID, imm32: u32 },
    /// dst = dst ^ imm32
    IXOR_C { dst: RGroupRegisterID, imm32: u32 },
    /// dst = (dst * src) >> 64
    IMULH_R {
        dst: RGroupRegisterID,
        src: RGroupRegisterID,
    },
    /// dst = (dst * src) >> 64 (signed)
    ISMULH_R {
        dst: RGroupRegisterID,
        src: RGroupRegisterID,
    },
    /// dst = 2x / imm32 * dst
    IMUL_RCP { dst: RGroupRegisterID, imm32: u32 },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OpSource {
    Constant,
    Register(RGroupRegisterID),
    /// Not actually a source, but the C++ version sets this field to a
    /// random value on some instructions.
    Randi32(i32),
}

impl OpSource {
    pub fn from_rand_i32(x: i32) -> Self {
        match x {
            -1 => OpSource::Constant,
            0 => OpSource::Register(RGroupRegisterID::R0),
            1 => OpSource::Register(RGroupRegisterID::R1),
            2 => OpSource::Register(RGroupRegisterID::R2),
            3 => OpSource::Register(RGroupRegisterID::R3),
            4 => OpSource::Register(RGroupRegisterID::R4),
            5 => OpSource::Register(RGroupRegisterID::R5),
            6 => OpSource::Register(RGroupRegisterID::R6),
            7 => OpSource::Register(RGroupRegisterID::R7),
            rand => OpSource::Randi32(rand),
        }
    }
}

impl ScalarInstruction {
    pub fn dst(&self) -> RGroupRegisterID {
        match self {
            ScalarInstruction::ISUB_R { dst, .. }
            | ScalarInstruction::IXOR_R { dst, .. }
            | ScalarInstruction::IADD_RS { dst, .. }
            | ScalarInstruction::IMUL_R { dst, .. }
            | ScalarInstruction::IROR_C { dst, .. }
            | ScalarInstruction::IADD_C { dst, .. }
            | ScalarInstruction::IXOR_C { dst, .. }
            | ScalarInstruction::IMULH_R { dst, .. }
            | ScalarInstruction::ISMULH_R { dst, .. }
            | ScalarInstruction::IMUL_RCP { dst, .. } => *dst,
        }
    }

    pub fn src(&self) -> Option<RGroupRegisterID> {
        match self {
            ScalarInstruction::ISUB_R { src, .. }
            | ScalarInstruction::IXOR_R { src, .. }
            | ScalarInstruction::IADD_RS { src, .. }
            | ScalarInstruction::IMUL_R { src, .. }
            | ScalarInstruction::IMULH_R { src, .. }
            | ScalarInstruction::ISMULH_R { src, .. } => Some(*src),
            ScalarInstruction::IROR_C { .. }
            | ScalarInstruction::IADD_C { .. }
            | ScalarInstruction::IXOR_C { .. }
            | ScalarInstruction::IMUL_RCP { .. } => None,
        }
    }

    pub fn id(&self) -> ScalarInstructionID {
        match self {
            ScalarInstruction::ISUB_R { .. } => ScalarInstructionID::ISUB_R,
            ScalarInstruction::IXOR_R { .. } => ScalarInstructionID::IXOR_R,
            ScalarInstruction::IADD_RS { .. } => ScalarInstructionID::IADD_RS,
            ScalarInstruction::IMUL_R { .. } => ScalarInstructionID::IMUL_R,
            ScalarInstruction::IROR_C { .. } => ScalarInstructionID::IROR_C,
            ScalarInstruction::IADD_C { .. } => ScalarInstructionID::IADD_C,
            ScalarInstruction::IXOR_C { .. } => ScalarInstructionID::IXOR_C,
            ScalarInstruction::IMULH_R { .. } => ScalarInstructionID::IMULH_R,
            ScalarInstruction::ISMULH_R { .. } => ScalarInstructionID::ISMULH_R,
            ScalarInstruction::IMUL_RCP { .. } => ScalarInstructionID::IMUL_RCP,
        }
    }

    pub fn op_source(&self) -> OpSource {
        match self {
            ScalarInstruction::ISUB_R { src, .. }
            | ScalarInstruction::IXOR_R { src, .. }
            | ScalarInstruction::IADD_RS { src, .. }
            | ScalarInstruction::IMUL_R { src, .. }
            | ScalarInstruction::IMULH_R { src, .. }
            | ScalarInstruction::ISMULH_R { src, .. } => OpSource::Register(*src),
            ScalarInstruction::IROR_C { .. }
            | ScalarInstruction::IADD_C { .. }
            | ScalarInstruction::IXOR_C { .. }
            | ScalarInstruction::IMUL_RCP { .. } => OpSource::Constant,
        }
    }
}
