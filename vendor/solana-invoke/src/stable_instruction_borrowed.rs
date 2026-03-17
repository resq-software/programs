use std::{marker::PhantomData, mem::ManuallyDrop};

use solana_instruction::Instruction;
use solana_stable_layout::{stable_instruction::StableInstruction, stable_vec::StableVec};

pub(crate) struct StableInstructionBorrowed<'ix> {
    stabilized_instruction: ManuallyDrop<StableInstruction>,
    _marker: PhantomData<&'ix Instruction>,
}

impl<'ix> StableInstructionBorrowed<'ix> {
    #[inline(always)]
    pub(crate) fn new(ix: &'ix Instruction) -> Self {
        let data = StableVecBorrowed::from(&ix.data);
        let accounts = StableVecBorrowed::from(&ix.accounts);
        let fake_stable_ix = unsafe {
            ManuallyDrop::new(StableInstruction {
                accounts: core::mem::transmute::<StableVecBorrowed<_>, StableVec<_>>(accounts),
                data: core::mem::transmute::<StableVecBorrowed<_>, StableVec<_>>(data),
                program_id: ix.program_id,
            })
        };

        Self {
            stabilized_instruction: fake_stable_ix,
            _marker: PhantomData,
        }
    }

    pub(crate) fn instruction_addr(&self) -> *const u8 {
        &self.stabilized_instruction as *const ManuallyDrop<StableInstruction> as *const u8
    }
}

#[repr(C)]
struct StableVecBorrowed<'vec, T> {
    addr: u64,
    cap: u64,
    len: u64,
    _marker: PhantomData<&'vec T>,
}

impl<'a, T> From<&'a Vec<T>> for StableVecBorrowed<'a, T> {
    fn from(value: &'a Vec<T>) -> Self {
        Self {
            addr: value.as_ptr() as u64,
            cap: value.capacity() as u64,
            len: value.len() as u64,
            _marker: PhantomData,
        }
    }
}
