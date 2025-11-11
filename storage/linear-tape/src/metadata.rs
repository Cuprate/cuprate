use std::{
    fs::OpenOptions,
    io::{self, Write},
    ops::Deref,
    path::Path,
};

use cuprate_rcu_ring::{required_len, DataHandle, RcuRing, WriteGuard};
use memmap2::{MmapOptions, MmapRaw};

use crate::Flush;

/// The u32 value which represents an append operation.
pub(crate) const APPEND_OP: u32 = 0;
/// The u32 value which represents a pop operation.
pub(crate) const POP_OP: u32 = 1;

/// A read handle to the metadata.
pub(crate) struct MetadataHandle<'a> {
    data_handle: DataHandle<'a>,
}

impl Deref for MetadataHandle<'_> {
    type Target = [usize];
    fn deref(&self) -> &Self::Target {
        // Safety:
        //    RcuRing ensures the returned slice is 8-byte aligned.
        //    We only support 64-bit targets.
        unsafe {
            std::slice::from_raw_parts(
                self.data_handle.data.as_ptr().cast(),
                self.data_handle.data.len() / 8,
            )
        }
    }
}

/// A write guard for updating the metadata.
///
/// Changes must be flushed with [`MetadataWriteGuard::push_update`] to be seen.
pub(crate) struct MetadataWriteGuard<'a> {
    write_guard: WriteGuard<'a>,
    mmap: &'a MmapRaw,
}

impl MetadataWriteGuard<'_> {
    /// Returns the lengths of all tables.
    pub(crate) fn tables_len_mut(&mut self) -> &mut [usize] {
        let len = self.write_guard.data_mut().len();
        // Safety:
        //    RcuRing ensures the returned slice is 8-byte aligned.
        //    We only support 64-bit targets.
        unsafe {
            std::slice::from_raw_parts_mut(self.write_guard.data_mut().as_mut_ptr().cast(), len / 8)
        }
    }

    pub(crate) fn tables_len(&self) -> &[usize] {
        let len = self.write_guard.data().len();
        // Safety:
        //    RcuRing ensures the returned slice is 8-byte aligned.
        //    We only support 64-bit targets.
        unsafe { std::slice::from_raw_parts(self.write_guard.data().as_ptr().cast(), len / 8) }
    }

    pub(crate) fn current_data_slot_idx(&self) -> usize {
        self.write_guard.current_data_slot_idx()
    }

    /// Push the changes made during this write.
    ///
    /// This guard must not be used after this call.
    pub(crate) fn push_update(&mut self, mode: Flush) -> io::Result<()> {
        self.write_guard.push_update();

        match mode {
            Flush::Sync => self.mmap.flush(),
            Flush::Async => self.mmap.flush_async(),
            Flush::NoSync => Ok(()),
        }
    }
}

/// The metadata of the liner tapes databases.
///
/// Handles tracking their lengths and atomically updating them.
pub(crate) struct Metadata {
    mmap: MmapRaw,
    rcu_ring: RcuRing,
}

impl Metadata {
    /// Opens the metadata file for the tapes.
    pub(crate) unsafe fn open<P: AsRef<Path>>(
        path: P,
        tables: usize,
        metadata_ring_len: usize,
    ) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .create(true)
            .open(path.as_ref())?;

        let len = file.metadata()?.len();
        let expected_len = required_len(tables * 8, metadata_ring_len);

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

        // Safety: we just checked the length of the file, and wrote zeros if there wasn't enough.
        unsafe {
            let rcu_ring = RcuRing::new_from_ptr(ptr, tables * 8, metadata_ring_len);

            Ok(Self { mmap, rcu_ring })
        }
    }

    /// Start a reader for the metadata.
    pub(crate) fn start_read(&self) -> MetadataHandle<'_> {
        let data_handle = self.rcu_ring.start_read();

        MetadataHandle { data_handle }
    }

    pub(crate) fn wait_for_all_readers(&self, current_slot_idx: usize) {
        self.rcu_ring.wait_for_all_readers(current_slot_idx);
    }

    /// Start a writer for the metadata.
    pub(crate) fn start_write(&self, op: u32) -> MetadataWriteGuard<'_> {
        let write_guard = self
            .rcu_ring
            .start_write(op, |last_op| last_op == POP_OP && op != POP_OP);

        MetadataWriteGuard {
            write_guard,
            mmap: &self.mmap,
        }
    }
}
