use cuprate_pruning::{PruningSeed, CRYPTONOTE_PRUNING_LOG_STRIPES};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{BlockchainError, DATABASE_VERSION};

/// Represents the metadata of the blockchain database.
#[derive(Clone, Debug)]
pub struct Metadata {
    inner: MetadataInner,
    path: PathBuf,
}

#[derive(Clone, Debug)]
struct MetadataInner {
    /// The version of the database, defaults to [`crate::constants::DATABASE_VERSION`].
    db_version: u64,
    /// The seed used for pruning, defaults to [`PruningSeed::NotPruned`].
    /// It is compressed to a `u32` value for storage.
    pruning_seed: PruningSeed,
}

impl Metadata {
    const FILE_NAME: &'static str = "metadata";

    /// Opens the metadata file, or creates a new one if it doesn't exist yet.
    pub fn get_or_create(db_dir_path: impl AsRef<Path>) -> Result<Self, BlockchainError> {
        let metadata_path = Path::new(db_dir_path.as_ref()).join(Self::FILE_NAME);
        let exists = metadata_path.exists();

        if exists {
            let mut file = File::open(&metadata_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            let inner = MetadataInner::deserialize(buf.as_slice())?;
            tracing::info!(
                "Opened existing metadata file with DB version = {} and pruning_stripe = {:?}.",
                inner.db_version,
                inner.pruning_seed.get_stripe()
            );
            Ok(Self::from_inner(inner, metadata_path))
        } else {
            let metadata = Self::from_inner(MetadataInner::default(), metadata_path);
            metadata.persist()?;
            Ok(metadata)
        }
    }

    /// Gets the stripe index from the pruning seed.
    ///
    /// This will persist the metadata file to disk.
    pub fn set_stripe_idx(&mut self, stripe_idx: u32) -> Result<(), BlockchainError> {
        self.inner.pruning_seed =
            PruningSeed::new_pruned(stripe_idx, CRYPTONOTE_PRUNING_LOG_STRIPES)?;

        self.persist()
    }

    /// Gets the pruning seed.
    ///
    /// Note that this doesn't re-read the file.
    #[inline]
    pub const fn get_pruning_seed(&self) -> PruningSeed {
        self.inner.pruning_seed
    }

    #[inline]
    pub const fn get_db_version(&self) -> u64 {
        self.inner.db_version
    }

    #[inline]
    const fn from_inner(inner: MetadataInner, path: PathBuf) -> Self {
        Self { inner, path }
    }

    /// Saves the metadata to the file.
    #[inline]
    fn persist(&self) -> Result<(), BlockchainError> {
        let mut file = File::create(&self.path)?;
        let mut buf = bytes::BytesMut::new();
        self.inner.serialize(&mut buf);
        file.write_all(&buf)?;
        Ok(())
    }
}

impl MetadataInner {
    /// serializes [`MetadataInner`] into a buffer like a [`Vec`] or [`bytes::BytesMut`]
    fn serialize(&self, mut buf: impl bytes::BufMut) {
        buf.put_u64_le(self.db_version);
        buf.put_u32_le(self.pruning_seed.compress());
    }

    /// deserializes a [`bytes::Buf`] into a [`MetadataInner`]
    fn deserialize(mut bytes: impl bytes::Buf) -> Result<Self, BlockchainError> {
        let db_version = bytes.try_get_u64_le()?;
        let pruning_seed = PruningSeed::decompress(bytes.try_get_u32_le()?)?;

        Ok(Self {
            db_version,
            pruning_seed,
        })
    }
}

impl Default for MetadataInner {
    /// Creates a new [`MetadataInner`] with the given path. Default `pruning_seed` is [`PruningSeed::NotPruned`].
    #[inline]
    fn default() -> Self {
        Self {
            db_version: DATABASE_VERSION,
            pruning_seed: PruningSeed::NotPruned,
        }
    }
}
