use std::marker::PhantomData;

use rayon::iter::{
    plumbing::{Consumer, Folder, Reducer, UnindexedConsumer},
    IntoParallelIterator, ParallelExtend, ParallelIterator,
};

use super::{entry_byte_range, Entry, FixedSizedTapeAppender};
use crate::unsafe_tape::UnsafeTape;

impl<T: Entry + Send> ParallelExtend<T> for FixedSizedTapeAppender<'_, T> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        let iterator = par_iter.into_par_iter();

        match iterator.opt_len() {
            Some(items) => {
                self.reserve_capacity(items).unwrap();

                let consumer = RayonExtendTapeImpl {
                    tape: self.backing_file,
                    index: self.len(),
                    phantom: PhantomData,
                };

                *self.bytes_added += items * T::SIZE;

                iterator.drive_unindexed(consumer);
            }
            None => {
                let lists = iterator.collect_vec_list();
                for list in lists {
                    self.push_entries(&list).unwrap();
                }
            }
        }
    }
}

struct RayonExtendTapeImpl<'a, T> {
    tape: &'a UnsafeTape,
    index: usize,

    pub(crate) phantom: PhantomData<T>,
}

impl<T: Entry + Send> UnindexedConsumer<T> for RayonExtendTapeImpl<'_, T> {
    fn split_off_left(&self) -> Self {
        unreachable!("This is only called for unindexed iters")
    }

    fn to_reducer(&self) -> Self::Reducer {
        RayonExtendTapeReducer
    }
}

impl<T: Entry + Send> Consumer<T> for RayonExtendTapeImpl<'_, T> {
    type Folder = Self;
    type Reducer = RayonExtendTapeReducer;
    type Result = ();

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        (
            Self {
                tape: &self.tape,
                index: self.index,
                phantom: PhantomData,
            },
            Self {
                tape: &self.tape,
                index: self.index + index,
                phantom: PhantomData,
            },
            RayonExtendTapeReducer,
        )
    }

    fn into_folder(self) -> Self::Folder {
        self
    }

    fn full(&self) -> bool {
        false
    }
}

impl<T: Entry + Send> Folder<T> for RayonExtendTapeImpl<'_, T> {
    type Result = ();

    fn consume(mut self, item: T) -> Self {
        let bytes = unsafe { self.tape.range_mut(entry_byte_range::<T>(self.index)) };
        item.write(bytes);
        self.index += 1;
        self
    }

    fn complete(self) -> Self::Result {
        ()
    }

    fn full(&self) -> bool {
        false
    }
}

struct RayonExtendTapeReducer;

impl Reducer<()> for RayonExtendTapeReducer {
    fn reduce(self, _: (), _: ()) -> () {
        ()
    }
}
