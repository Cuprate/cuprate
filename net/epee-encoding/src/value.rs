use alloc::{string::String, vec::Vec};
/// This module contains a `sealed` [`EpeeValue`] trait and different impls for
/// the different possible base epee values.
use core::fmt::Debug;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use sealed::sealed;

use fixed_bytes::{ByteArray, ByteArrayVec};

use crate::{
    io::*, varint::*, EpeeObject, Error, InnerMarker, Marker, Result, MAX_STRING_LEN_POSSIBLE,
};

/// A trait for epee values, this trait is sealed as all possible epee values are
/// defined in the lib, to make an [`EpeeValue`] outside the lib you will need to
/// use the trait [`EpeeObject`].
#[sealed(pub(crate))]
pub trait EpeeValue: Sized {
    const MARKER: Marker;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self>;

    fn should_write(&self) -> bool {
        true
    }

    /// This is different than default field values and instead is the default
    /// value of a whole type.
    ///
    /// For example a `Vec` has a default value of a zero length vec as when a
    /// sequence has no entries it is not encoded.
    fn epee_default_value() -> Option<Self> {
        None
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()>;
}

#[sealed]
impl<T: EpeeObject> EpeeValue for T {
    const MARKER: Marker = Marker::new(InnerMarker::Object);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if marker != &Self::MARKER {
            return Err(Error::Format("Marker does not match expected Marker"));
        }

        let mut skipped_objects = 0;
        crate::read_object(r, &mut skipped_objects)
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.number_of_fields(), w)?;
        self.write_fields(w)
    }
}

#[sealed]
impl<T: EpeeObject> EpeeValue for Vec<T> {
    const MARKER: Marker = T::MARKER.into_seq();

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if !marker.is_seq {
            return Err(Error::Format(
                "Marker is not sequence when a sequence was expected",
            ));
        }
        let len = read_varint(r)?;

        let individual_marker = Marker::new(marker.inner_marker);

        let mut res = Vec::with_capacity(len.try_into()?);
        for _ in 0..len {
            res.push(T::read(r, &individual_marker)?);
        }
        Ok(res)
    }

    fn should_write(&self) -> bool {
        !self.is_empty()
    }

    fn epee_default_value() -> Option<Self> {
        Some(Vec::new())
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;
        for item in self.into_iter() {
            item.write(w)?;
        }
        Ok(())
    }
}

#[sealed]
impl<T: EpeeObject + Debug, const N: usize> EpeeValue for [T; N] {
    const MARKER: Marker = <T>::MARKER.into_seq();

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        let vec = Vec::<T>::read(r, marker)?;

        if vec.len() != N {
            return Err(Error::Format("Array has incorrect length"));
        }

        Ok(vec.try_into().unwrap())
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;
        for item in self.into_iter() {
            item.write(w)?;
        }
        Ok(())
    }
}

macro_rules! epee_numb {
    ($numb:ty, $marker:ident, $read_fn:ident, $write_fn:ident) => {
        #[sealed]
        impl EpeeValue for $numb {
            const MARKER: Marker = Marker::new(InnerMarker::$marker);

            fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
                if marker != &Self::MARKER {
                    return Err(Error::Format("Marker does not match expected Marker"));
                }

                checked_read_primitive(r, Buf::$read_fn)
            }

            fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
                checked_write_primitive(w, BufMut::$write_fn, self)
            }
        }
    };
}

epee_numb!(i64, I64, get_i64_le, put_i64_le);
epee_numb!(i32, I32, get_i32_le, put_i32_le);
epee_numb!(i16, I16, get_i16_le, put_i16_le);
epee_numb!(i8, I8, get_i8, put_i8);
epee_numb!(u8, U8, get_u8, put_u8);
epee_numb!(u16, U16, get_u16_le, put_u16_le);
epee_numb!(u32, U32, get_u32_le, put_u32_le);
epee_numb!(u64, U64, get_u64_le, put_u64_le);
epee_numb!(f64, F64, get_f64_le, put_f64_le);

#[sealed]
impl EpeeValue for bool {
    const MARKER: Marker = Marker::new(InnerMarker::Bool);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if marker != &Self::MARKER {
            return Err(Error::Format("Marker does not match expected Marker"));
        }

        Ok(checked_read_primitive(r, Buf::get_u8)? != 0)
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        checked_write_primitive(w, BufMut::put_u8, if self { 1 } else { 0 })
    }
}

#[sealed]
impl EpeeValue for Vec<u8> {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if marker != &Self::MARKER {
            return Err(Error::Format("Marker does not match expected Marker"));
        }

        let len = read_varint(r)?;
        if len > MAX_STRING_LEN_POSSIBLE {
            return Err(Error::Format("Byte array exceeded max length"));
        }

        if r.remaining() < len.try_into()? {
            return Err(Error::IO("Not enough bytes to fill object"));
        }

        let mut res = vec![0; len.try_into()?];
        r.copy_to_slice(&mut res);

        Ok(res)
    }

    fn epee_default_value() -> Option<Self> {
        Some(Vec::new())
    }

    fn should_write(&self) -> bool {
        !self.is_empty()
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;

        if w.remaining_mut() < self.len() {
            return Err(Error::IO("Not enough capacity to write bytes"));
        }

        w.put_slice(&self);
        Ok(())
    }
}

#[sealed::sealed]
impl EpeeValue for Bytes {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if marker != &Self::MARKER {
            return Err(Error::Format("Marker does not match expected Marker"));
        }

        let len = read_varint(r)?;
        if len > MAX_STRING_LEN_POSSIBLE {
            return Err(Error::Format("Byte array exceeded max length"));
        }

        if r.remaining() < len.try_into()? {
            return Err(Error::IO("Not enough bytes to fill object"));
        }

        Ok(r.copy_to_bytes(len.try_into()?))
    }

    fn epee_default_value() -> Option<Self> {
        Some(Bytes::new())
    }

    fn should_write(&self) -> bool {
        !self.is_empty()
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;

        if w.remaining_mut() < self.len() {
            return Err(Error::IO("Not enough capacity to write bytes"));
        }

        w.put(self);
        Ok(())
    }
}

#[sealed::sealed]
impl EpeeValue for BytesMut {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if marker != &Self::MARKER {
            return Err(Error::Format("Marker does not match expected Marker"));
        }

        let len = read_varint(r)?;
        if len > MAX_STRING_LEN_POSSIBLE {
            return Err(Error::Format("Byte array exceeded max length"));
        }

        if r.remaining() < len.try_into()? {
            return Err(Error::IO("Not enough bytes to fill object"));
        }

        let mut bytes = BytesMut::zeroed(len.try_into()?);
        r.copy_to_slice(&mut bytes);

        Ok(bytes)
    }

    fn epee_default_value() -> Option<Self> {
        Some(BytesMut::new())
    }

    fn should_write(&self) -> bool {
        !self.is_empty()
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;

        if w.remaining_mut() < self.len() {
            return Err(Error::IO("Not enough capacity to write bytes"));
        }

        w.put(self);
        Ok(())
    }
}

#[sealed::sealed]
impl<const N: usize> EpeeValue for ByteArrayVec<N> {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if marker != &Self::MARKER {
            return Err(Error::Format("Marker does not match expected Marker"));
        }

        let len = read_varint(r)?;
        if len > MAX_STRING_LEN_POSSIBLE {
            return Err(Error::Format("Byte array exceeded max length"));
        }

        if r.remaining() < usize::try_from(len)? {
            return Err(Error::IO("Not enough bytes to fill object"));
        }

        ByteArrayVec::try_from(r.copy_to_bytes(usize::try_from(len)?))
            .map_err(|_| Error::Format("Field has invalid length"))
    }

    fn epee_default_value() -> Option<Self> {
        Some(ByteArrayVec::try_from(Bytes::new()).unwrap())
    }

    fn should_write(&self) -> bool {
        !self.is_empty()
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        let bytes = self.take_bytes();

        write_varint(bytes.len().try_into()?, w)?;

        if w.remaining_mut() < bytes.len() {
            return Err(Error::IO("Not enough capacity to write bytes"));
        }

        w.put(bytes);
        Ok(())
    }
}

#[sealed::sealed]
impl<const N: usize> EpeeValue for ByteArray<N> {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if marker != &Self::MARKER {
            return Err(Error::Format("Marker does not match expected Marker"));
        }

        let len: usize = read_varint(r)?.try_into()?;
        if len != N {
            return Err(Error::Format("Byte array has incorrect length"));
        }

        if r.remaining() < N {
            return Err(Error::IO("Not enough bytes to fill object"));
        }

        ByteArray::try_from(r.copy_to_bytes(N))
            .map_err(|_| Error::Format("Field has invalid length"))
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        let bytes = self.take_bytes();

        write_varint(N.try_into().unwrap(), w)?;

        if w.remaining_mut() < N {
            return Err(Error::IO("Not enough capacity to write bytes"));
        }

        w.put(bytes);
        Ok(())
    }
}

#[sealed]
impl EpeeValue for String {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        let bytes = Vec::<u8>::read(r, marker)?;
        String::from_utf8(bytes).map_err(|_| Error::Format("Invalid string"))
    }

    fn should_write(&self) -> bool {
        !self.is_empty()
    }

    fn epee_default_value() -> Option<Self> {
        Some(String::new())
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;

        if w.remaining_mut() < self.len() {
            return Err(Error::IO("Not enough capacity to write bytes"));
        }

        w.put_slice(self.as_bytes());
        Ok(())
    }
}

#[sealed]
impl<const N: usize> EpeeValue for [u8; N] {
    const MARKER: Marker = Marker::new(InnerMarker::String);

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        let bytes = Vec::<u8>::read(r, marker)?;

        if bytes.len() != N {
            return Err(Error::Format("Byte array has incorrect length"));
        }

        Ok(bytes.try_into().unwrap())
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;

        if w.remaining_mut() < self.len() {
            return Err(Error::IO("Not enough capacity to write bytes"));
        }

        w.put_slice(&self);
        Ok(())
    }
}

#[sealed]
impl<const N: usize> EpeeValue for Vec<[u8; N]> {
    const MARKER: Marker = <[u8; N]>::MARKER.into_seq();

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        if !marker.is_seq {
            return Err(Error::Format(
                "Marker is not sequence when a sequence was expected",
            ));
        }

        let len = read_varint(r)?;

        let individual_marker = Marker::new(marker.inner_marker);

        let mut res = Vec::with_capacity(len.try_into()?);
        for _ in 0..len {
            res.push(<[u8; N]>::read(r, &individual_marker)?);
        }
        Ok(res)
    }

    fn should_write(&self) -> bool {
        !self.is_empty()
    }

    fn epee_default_value() -> Option<Self> {
        Some(Vec::new())
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        write_varint(self.len().try_into()?, w)?;
        for item in self.into_iter() {
            item.write(w)?;
        }
        Ok(())
    }
}

macro_rules! epee_seq {
    ($val:ty) => {
        #[sealed]
        impl EpeeValue for Vec<$val> {
            const MARKER: Marker = <$val>::MARKER.into_seq();

            fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
                if !marker.is_seq {
                    return Err(Error::Format(
                        "Marker is not sequence when a sequence was expected",
                    ));
                }

                let len = read_varint(r)?;

                let individual_marker = Marker::new(marker.inner_marker.clone());

                let mut res = Vec::with_capacity(len.try_into()?);
                for _ in 0..len {
                    res.push(<$val>::read(r, &individual_marker)?);
                }
                Ok(res)
            }

            fn should_write(&self) -> bool {
                !self.is_empty()
            }

            fn epee_default_value() -> Option<Self> {
                Some(Vec::new())
            }

            fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
                write_varint(self.len().try_into()?, w)?;
                for item in self.into_iter() {
                    item.write(w)?;
                }
                Ok(())
            }
        }

        #[sealed]
        impl<const N: usize> EpeeValue for [$val; N] {
            const MARKER: Marker = <$val>::MARKER.into_seq();

            fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
                let vec = Vec::<$val>::read(r, marker)?;

                if vec.len() != N {
                    return Err(Error::Format("Array has incorrect length"));
                }

                Ok(vec.try_into().unwrap())
            }

            fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
                write_varint(self.len().try_into()?, w)?;
                for item in self.into_iter() {
                    item.write(w)?;
                }
                Ok(())
            }
        }
    };
}

epee_seq!(i64);
epee_seq!(i32);
epee_seq!(i16);
epee_seq!(i8);
epee_seq!(u64);
epee_seq!(u32);
epee_seq!(u16);
epee_seq!(f64);
epee_seq!(bool);
epee_seq!(Vec<u8>);
epee_seq!(String);
epee_seq!(Bytes);
epee_seq!(BytesMut);

#[sealed]
impl<T: EpeeValue> EpeeValue for Option<T> {
    const MARKER: Marker = T::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> Result<Self> {
        Ok(Some(T::read(r, marker)?))
    }

    fn should_write(&self) -> bool {
        match self {
            Some(t) => t.should_write(),
            None => false,
        }
    }

    fn epee_default_value() -> Option<Self> {
        Some(None)
    }

    fn write<B: BufMut>(self, w: &mut B) -> Result<()> {
        match self {
            Some(t) => t.write(w)?,
            None => panic!("Can't write an Option::None value, this should be handled elsewhere"),
        }
        Ok(())
    }
}
