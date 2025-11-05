use std::sync::atomic::{AtomicU32, Ordering};

use atomic_wait::wait;

pub struct MetadataState<'a>(&'a AtomicU32);

const METADATA_OLD: u32 = 1 << 31;
const WRITER_WAITING: u32 = 1 << 30;

const READER_COUNT_MASK: u32 = !(METADATA_OLD | WRITER_WAITING);

impl MetadataState<'_> {
    pub unsafe fn from_u8_ptr(ptr: *mut u8) -> Self {
        let ptr = ptr.cast::<u32>();

        assert!(ptr.cast::<AtomicU32>().is_aligned());

        unsafe { Self(AtomicU32::from_ptr(ptr)) }
    }

    pub fn check_add_reader(&self) -> bool {
        loop {
            let state = self.0.load(Ordering::Acquire);

            if state & METADATA_OLD == METADATA_OLD {
                return false;
            }

            if state & READER_COUNT_MASK == READER_COUNT_MASK - 1 {
                panic!("Too many readers");
            }

            match self.0.compare_exchange_weak(
                state,
                state + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(_) => continue,
            }
        }
    }

    pub fn mark_old(&self) {
        self.0.fetch_or(METADATA_OLD, Ordering::AcqRel);
    }

    pub fn wait_for_readers(&self) {
        let mut i = 0;
        let mut old = self.0.fetch_or(WRITER_WAITING, Ordering::AcqRel);
        loop {
            if old & READER_COUNT_MASK == 0 {
                self.0.store(0, Ordering::Release);
                return;
            }

            if i > 100 {
                wait(&self.0, old);
            } else {
                std::hint::spin_loop();
            }

            i += 1;
            old = self.0.load(Ordering::Acquire);
        }
    }

    pub fn remove_reader(&self) {
        let old = self.0.fetch_sub(1, Ordering::AcqRel);

        if old & READER_COUNT_MASK == 1 && old & WRITER_WAITING == WRITER_WAITING {
            atomic_wait::wake_all(self.0)
        }
    }
}
