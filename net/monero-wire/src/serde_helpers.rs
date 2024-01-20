pub(crate) fn default_false() -> bool {
    false
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_zero<T: TryFrom<u8>>() -> T {
    0.try_into()
        .map_err(|_| "Couldn't fit 0 into integer type!")
        .unwrap()
}

pub(crate) mod serde_vec_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_bytes::ByteBuf;

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Vec::<ByteBuf>::deserialize(d)?
            .into_iter()
            .map(ByteBuf::into_vec)
            .collect())
    }

    pub fn serialize<S>(t: &[Vec<u8>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.collect_seq(t.iter())
    }
}
