use atomic_wait::wait;
use memmap2::{MmapOptions, MmapRaw};
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::ops::Add;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

mod mutex;
mod state;

use mutex::{Mutex, MutexGuard};
use state::MetadataState;

const HEADER_LEN: usize = 8 * 2;

pub struct MetadataHandle<'a> {
    metadata_state: MetadataState<'a>,
    pub tables_len: &'a [usize],
}

impl Drop for MetadataHandle<'_> {
    fn drop(&mut self) {
        self.metadata_state.remove_reader();
    }
}

pub struct WriteGuard<'a> {
    _mutex_guard: MutexGuard<'a>,
    current_metadata_state: MetadataState<'a>,
    current_metadata: &'static AtomicU32,
    next_metadata: u32,
    tables_len: &'a mut [usize],
}

impl WriteGuard<'_> {
    pub fn tables_len_mut(&mut self) -> &mut [usize] {
        self.tables_len
    }

    pub fn push_update(&mut self) {
        self.tables_len = &mut [];
        self.current_metadata_state.mark_old();
        self.current_metadata
            .store(self.next_metadata, Ordering::Release);
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum WriteOp {
    Push,
    Pop,
}

impl WriteOp {
    const OPERATION_POP: u32 = 1;

    fn to_u32(self) -> u32 {
        match self {
            WriteOp::Push => 0,
            WriteOp::Pop => Self::OPERATION_POP,
        }
    }
}

pub struct MetadataFile {
    tables: usize,
    mmap: MmapRaw,
    writer_mutex: Mutex<'static>,
    current_metadata: &'static AtomicU32,
    metadata_ring_len: usize,
}

const fn metadata_len(tables: usize) -> usize {
    8 + tables * 8
}

impl MetadataFile {
    pub unsafe fn open<P: AsRef<Path>>(
        path: P,
        tables: usize,
        metadata_ring_len: usize,
    ) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.as_ref())?;

        let len = file.metadata()?.len();
        let expected_len = HEADER_LEN + metadata_len(tables) * metadata_ring_len;

        if len == 0 {
            file.write_all(&vec![0; expected_len])?;
        }

        let len = file.metadata()?.len();

        if len != expected_len as u64 {
            return Err(io::Error::other("metadata file incorrect size"));
        }

        let mmap = MmapOptions::new().map_raw(&file)?;

        mmap.lock()?;

        let ptr = mmap.as_mut_ptr();

        unsafe {
            let writer_mutex_au32 = atomic_u32_from_u8_ptr(ptr);
            let current_metadata = atomic_u32_from_u8_ptr(ptr.add(8));

            Ok(Self {
                tables,
                mmap,
                writer_mutex: Mutex::new(writer_mutex_au32),
                current_metadata,
                metadata_ring_len,
            })
        }
    }

    pub fn start_read(&self) -> MetadataHandle<'_> {
        loop {
            let current_idx = self.current_metadata.load(Ordering::Acquire) as usize;
            let metadata_ptr = self.metadata_ptr(current_idx);

            let metadata_state = unsafe { MetadataState::from_u8_ptr(metadata_ptr) };

            if !metadata_state.check_add_reader() {
                continue;
            }

            let tables_len =
                unsafe { std::slice::from_raw_parts(metadata_ptr.add(8).cast(), self.tables) };

            return MetadataHandle {
                metadata_state,
                tables_len,
            };
        }
    }

    pub fn start_write(&self, op: WriteOp) -> WriteGuard<'_> {
        let _mutex_guard = self.writer_mutex.lock();

        let current_idx = self.current_metadata.load(Ordering::Acquire) as usize;
        let current_metadata_ptr = self.metadata_ptr(current_idx);
        let current_metadata_state = unsafe { MetadataState::from_u8_ptr(current_metadata_ptr) };

        let last_op = unsafe { current_metadata_ptr.add(4).cast::<u32>().read() };

        let current_lens = unsafe {
            std::slice::from_raw_parts(current_metadata_ptr.add(8).cast::<usize>(), self.tables)
        };

        let next_idx = if last_op == WriteOp::OPERATION_POP && op == WriteOp::Push {
            self.wait_for_all_readers(current_idx)
        } else {
            self.wait_for_next_free_slot(current_idx)
        };

        let metadata_ptr = self.metadata_ptr(next_idx);

        unsafe { metadata_ptr.add(4).cast::<u32>().write(op.to_u32()) };
        let tables_len =
            unsafe { std::slice::from_raw_parts_mut(metadata_ptr.add(8).cast(), self.tables) };

        tables_len.copy_from_slice(current_lens);

        WriteGuard {
            _mutex_guard,
            current_metadata_state,
            current_metadata: &self.current_metadata,
            next_metadata: next_idx as u32,
            tables_len,
        }
    }

    fn wait_for_next_free_slot(&self, current_idx: usize) -> usize {
        let next_free_slot = (current_idx + 1) % self.metadata_ring_len;
        let metadata_ptr = self.metadata_ptr(next_free_slot);

        let metadata_state = unsafe { MetadataState::from_u8_ptr(metadata_ptr) };

        metadata_state.wait_for_readers();

        next_free_slot
    }

    fn wait_for_all_readers(&self, current_idx: usize) -> usize {
        for i in 1..self.metadata_ring_len {
            let metadata_ptr = self.metadata_ptr((current_idx + i) % self.metadata_ring_len);

            let metadata_state = unsafe { MetadataState::from_u8_ptr(metadata_ptr) };

            metadata_state.wait_for_readers();
        }

        (current_idx + 1) % self.metadata_ring_len
    }

    fn metadata_ptr(&self, idx: usize) -> *mut u8 {
        let metadata_ptr_offset = HEADER_LEN + metadata_len(self.tables) * idx;

        unsafe { self.mmap.as_mut_ptr().add(metadata_ptr_offset) }
    }
}

unsafe fn atomic_u32_from_u8_ptr(ptr: *mut u8) -> &'static AtomicU32 {
    let ptr = ptr.cast::<u32>();

    assert!(ptr.cast::<AtomicU32>().is_aligned());

    unsafe { AtomicU32::from_ptr(ptr) }
}

#[test]
fn tt() {
    let meta = unsafe { MetadataFile::open("metadata", 8, 2).unwrap() };

    for _ in 0..100000 {
        let mut gaurd = meta.start_write(WriteOp::Push);
        let i = gaurd.tables_len[0];
        gaurd.tables_len.copy_from_slice(&[i + 1; 8]);

        let read1 = meta.start_read();
        let read2 = meta.start_read();

        assert_eq!(read1.tables_len, read2.tables_len);
        assert_eq!(read1.tables_len, &[i; 8]);

        gaurd.push_update();

        assert_eq!(read1.tables_len, read2.tables_len);
        assert_eq!(read1.tables_len, &[i; 8]);

        let read1 = meta.start_read();
        let read2 = meta.start_read();

        assert_eq!(read1.tables_len, read2.tables_len);
        assert_eq!(read1.tables_len, &[i + 1; 8]);

        println!("{i}");
    }

    let mut gaurd = meta.start_write(WriteOp::Pop);
    let i = gaurd.tables_len[0];
    gaurd.tables_len.copy_from_slice(&[i - 1; 8]);

    let read1 = meta.start_read();
    let read2 = meta.start_read();

    assert_eq!(read1.tables_len, read2.tables_len);
    assert_eq!(read1.tables_len, &[i; 8]);

    gaurd.push_update();

    assert_eq!(read1.tables_len, read2.tables_len);
    assert_eq!(read1.tables_len, &[i; 8]);

    drop((read1, read2, gaurd));

    // let mut gaurd = meta.start_write(WriteOp::Push);
}
