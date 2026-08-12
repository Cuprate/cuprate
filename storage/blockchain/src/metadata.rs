use cuprate_pruning::{PruningSeed, CRYPTONOTE_PRUNING_LOG_STRIPES};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::{BlockchainError, DATABASE_VERSION};

/// Represents the metadata of the blockchain database.
#[derive(Clone, Debug)]
pub struct Metadata {
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
            let metadata = Self::deserialize(buf.as_slice())?;
            tracing::info!(
                "Opened existing metadata file with DB version = {} and pruning_stripe = {:?}.",
                metadata.db_version,
                metadata.get_pruning_seed().get_stripe()
            );
            Ok(metadata)
        } else {
            let metadata = Self::new();
            metadata.write_to_file(&metadata_path)?;
            Ok(metadata)
        }
    }

    /// Gets the stripe index from the pruning seed.
    ///
    /// This will persist the metadata file to disk.
    pub fn set_stripe_idx(
        &mut self,
        stripe_idx: u32,
        path: impl AsRef<Path>,
    ) -> Result<(), BlockchainError> {
        self.pruning_seed = PruningSeed::new_pruned(stripe_idx, CRYPTONOTE_PRUNING_LOG_STRIPES)?;

        self.persist(path)
    }

    /// Gets the pruning seed.
    ///
    /// Note that this doesn't re-read the file.
    #[inline]
    pub const fn get_pruning_seed(&self) -> PruningSeed {
        self.pruning_seed
    }

    #[inline]
    pub const fn get_db_version(&self) -> u64 {
        self.db_version
    }

    /// deserializes a [`bytes::Buf`] into a [`Metadata`]
    fn deserialize(mut bytes: impl bytes::Buf) -> Result<Self, BlockchainError> {
        let db_version = bytes.try_get_u64_le()?;
        let pruning_seed = PruningSeed::decompress(bytes.try_get_u32_le()?)?;

        Ok(Self {
            db_version,
            pruning_seed,
        })
    }

    /// serializes [`self`] into a buffer like a [`Vec`] or [`bytes::BytesMut`]
    fn serialize(&self, mut buf: impl bytes::BufMut) {
        buf.put_u64_le(self.db_version);
        buf.put_u32_le(self.pruning_seed.compress());
    }

    /// Saves the metadata to the file.
    #[inline]
    fn persist(&self, path: impl AsRef<Path>) -> Result<(), BlockchainError> {
        let metadata_path = Path::new(path.as_ref()).join(Self::FILE_NAME);
        self.write_to_file(metadata_path)?;
        Ok(())
    }

    /// Creates a new [`Metadata`] with the given path. Default `pruning_seed` is [`PruningSeed::NotPruned`].
    #[inline]
    const fn new() -> Self {
        Self {
            db_version: DATABASE_VERSION,
            pruning_seed: PruningSeed::NotPruned,
        }
    }

    /// Writes the metadata to the given file.
    #[inline]
    fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), BlockchainError> {
        let mut file = File::create(path)?;
        let mut buf = bytes::BytesMut::new();
        self.serialize(&mut buf);
        file.write_all(&buf)?;
        Ok(())
    }
}
