use bytes::{Buf, BufMut, Bytes, BytesMut};
use ref_cast::RefCast;

use crate::{error::*, EpeeValue, InnerMarker, Marker};

#[derive(RefCast)]
#[repr(transparent)]
pub struct ContainerAsBlob<T: Containerable + EpeeValue>(Vec<T>);

impl<T: Containerable + EpeeValue> From<Vec<T>> for ContainerAsBlob<T> {
    fn from(value: Vec<T>) -> Self {
        ContainerAsBlob(value)
    }
}

impl<T: Containerable + EpeeValue> From<ContainerAsBlob<T>> for Vec<T> {
    fn from(value: ContainerAsBlob<T>) -> Self {
        value.0
    }
}

impl<'a, T: Containerable + EpeeValue> From<&'a Vec<T>> for &'a ContainerAsBlob<T> {
    fn from(value: &'a Vec<T>) -> Self {
        ContainerAsBlob::ref_cast(value)
    }
}

impl<T: Containerable + EpeeValue> EpeeValue for ContainerAsBlob<T> {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        let bytes = Bytes::read(r, marker)?;
        if bytes.len() % T::SIZE != 0 {
            return Err(Error::Value(
                "Can't convert blob container to Vec type.".to_string(),
            ));
        }

        Ok(ContainerAsBlob(
            bytes.chunks(T::SIZE).map(T::from_bytes).collect(),
        ))
    }

    fn should_write(&self) -> bool {
        !self.0.is_empty()
    }

    fn epee_default_value() -> Option<Self> {
        Some(ContainerAsBlob(vec![]))
    }

    fn write<B: BufMut>(self, w: &mut B) -> crate::Result<()> {
        let mut buf = BytesMut::with_capacity(self.0.len() * T::SIZE);
        self.0.iter().for_each(|tt| tt.push_bytes(&mut buf));
        buf.write(w)
    }
}

pub trait Containerable {
    const SIZE: usize;

    /// Returns `Self` from bytes.
    ///
    /// `bytes` is guaranteed to be [`Self::SIZE`] long.
    fn from_bytes(bytes: &[u8]) -> Self;

    fn push_bytes(&self, buf: &mut BytesMut);
}

macro_rules! int_container_able {
    ($int:ty ) => {
        impl Containerable for $int {
            const SIZE: usize = std::mem::size_of::<$int>();

            fn from_bytes(bytes: &[u8]) -> Self {
                <$int>::from_le_bytes(bytes.try_into().unwrap())
            }

            fn push_bytes(&self, buf: &mut BytesMut) {
                buf.put_slice(&self.to_le_bytes())
            }
        }
    };
}

int_container_able!(u16);
int_container_able!(u32);
int_container_able!(u64);
int_container_able!(u128);

int_container_able!(i8);
int_container_able!(i16);
int_container_able!(i32);
int_container_able!(i64);
int_container_able!(i128);
