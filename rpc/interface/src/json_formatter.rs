use std::io::Write;

use bytes::{BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use serde_json::{Result, Serializer};

/// Formatter for JSON output from the [`RpcHandler`](crate::RpcHandler).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum JsonFormatter {
    #[default]
    Compact,
    Pretty,
}

impl JsonFormatter {
    /// Serialize to [`Bytes`].
    pub fn to_bytes<T: ?Sized + Serialize>(self, value: &T) -> Result<Bytes> {
        // <https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189>
        let mut buf = BytesMut::with_capacity(128).writer();
        self.to_writer(&mut buf, value)?;
        Ok(buf.into_inner().freeze())
    }

    /// Serialize to a writer.
    pub fn to_writer<W: Write, T: ?Sized + Serialize>(self, writer: W, value: &T) -> Result<()> {
        match self {
            Self::Compact => value.serialize(&mut Serializer::new(writer)),
            Self::Pretty => value.serialize(&mut Serializer::pretty(writer)),
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::to_string;

    use super::*;

    #[test]
    fn serde() {
        assert_eq!(to_string(&JsonFormatter::Compact).unwrap(), r#""Compact""#);
        assert_eq!(to_string(&JsonFormatter::Pretty).unwrap(), r#""Pretty""#);
    }
}
