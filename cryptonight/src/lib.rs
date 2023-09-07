#[link(name = "cryptonight")]
extern "C" {
    fn cn_slow_hash(
        data: *const u8,
        length: usize,
        hash: *mut u8,
        variant: i32,
        pre_hashed: i32,
        height: u64,
    );
}

/// CryptoNight variants used in Monero, with data needed for specific variants.
pub enum Variant {
    V0,
    V1,
    V2,
    R { height: u64 },
}

impl Variant {
    /// Returns the height of the block we are hashing, if thats relevant for this variant otherwise
    /// `0` is returned.
    fn height(&self) -> u64 {
        if let Variant::R { height } = self {
            *height
        } else {
            0
        }
    }

    fn identifier(&self) -> i32 {
        match self {
            Variant::V0 => 0,
            Variant::V1 => 1,
            Variant::V2 => 2,
            Variant::R { .. } => 4,
        }
    }
}

/// Calculates the CryptoNight variant hash of buf.
pub fn cryptonight_hash(buf: &[u8], variant: &Variant) -> [u8; 32] {
    let mut hash = [0; 32];
    unsafe {
        cn_slow_hash(
            buf.as_ptr(),
            buf.len(),
            hash.as_mut_ptr(),
            variant.identifier(),
            0,
            variant.height(),
        );
    }
    hash
}
