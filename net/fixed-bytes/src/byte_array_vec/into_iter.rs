use bytes::{Buf, Bytes};

pub struct ByteArrayVecIterator<const N: usize>(pub(crate) Bytes);

impl<const N: usize> Iterator for ByteArrayVecIterator<N> {
    type Item = [u8; N];

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        let next = self.0[..N].try_into().unwrap();
        self.0.advance(N);
        Some(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len() / N, Some(self.0.len() / N))
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        if self.0.is_empty() {
            return None;
        }

        Some(self.0[self.0.len() - N..].try_into().unwrap())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let iters_left = self.0.len() / N;

        iters_left.checked_sub(n)?;

        self.0.advance(n * N - N);

        self.next()
    }
}

impl<const N: usize> DoubleEndedIterator for ByteArrayVecIterator<N> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        Some(self.0[self.0.len() - N..].try_into().unwrap())
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let iters_left = self.0.len() / N;

        iters_left.checked_sub(n)?;

        self.0.truncate(self.0.len() - n * N - N);

        self.next_back()
    }
}
