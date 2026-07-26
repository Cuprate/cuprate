use cuprate_pruning::{PruningSeed, CRYPTONOTE_PRUNING_LOG_STRIPES};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::{BlockchainError, DATABASE_VERSION};

/// Represents the metadata of the blockchain database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// The version of the database, defaults to [`crate::constants::DATABASE_VERSION`].
    db_version: u64,
    /// The seed used for pruning, defaults to [`PruningSeed::NotPruned`].
    /// It is compressed to a `u32` value for storage.
    #[serde(with = "serde_pruning_seed")]
    pruning_seed: PruningSeed,
}

mod serde_pruning_seed {
    //! Serialization and deserialization of [`PruningSeed`] values.
    use cuprate_pruning::PruningSeed;

    pub(super) fn serialize<S>(seed: &PruningSeed, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(seed.compress())
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<PruningSeed, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let seed_value: u32 = serde::Deserialize::deserialize(deserializer)?;
        PruningSeed::decompress(seed_value).map_err(|e| serde::de::Error::custom(e))
    }
}

impl Metadata {
    const FILE_NAME: &'static str = "metadata.json";

    /// Opens the metadata file, or creates a new one if it doesn't exist yet.
    pub fn get_or_create(db_dir_path: impl AsRef<Path>) -> Result<Self, BlockchainError> {
        let metadata_path = Path::new(db_dir_path.as_ref()).join(Self::FILE_NAME);
        let exists = metadata_path.exists();

        if exists {
            let mut file = File::open(&metadata_path)?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            let metadata: Metadata = serde_json::from_str(&buf)?;
            Ok(metadata)
        } else {
            let metadata = Metadata::new();
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

    /// Gets the stripe index from the pruning seed.
    ///
    /// Note that this doesn't re-read the file.
    ///
    /// Returns `None` if the seed is not pruned (e.g. the seed is `0`).
    #[inline]
    pub fn get_stripe_idx(&self) -> Option<u32> {
        self.pruning_seed.get_stripe()
    }

    /// Gets the pruning seed.
    ///
    /// Note that this doesn't re-read the file.
    #[inline]
    pub fn get_pruning_seed(&self) -> PruningSeed {
        self.pruning_seed
    }

    #[inline]
    pub fn get_db_version(&self) -> u64 {
        self.db_version
    }

    #[inline]
    pub fn is_pruned(&self) -> bool {
        self.pruning_seed != PruningSeed::NotPruned
    }

    /// Saves the metadata to the file.
    #[inline]
    fn persist(&self, path: impl AsRef<Path>) -> Result<(), BlockchainError> {
        let metadata_path = Path::new(path.as_ref()).join(Self::FILE_NAME);
        self.write_to_file(metadata_path)?;
        Ok(())
    }

    /// Creates a new [`Metadata`] with the given path. Default pruning_seed is [`PruningSeed::NotPruned`].
    #[inline]
    fn new() -> Self {
        Self {
            db_version: DATABASE_VERSION,
            pruning_seed: PruningSeed::NotPruned,
        }
    }

    /// Writes the metadata to the given file.
    #[inline]
    fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), BlockchainError> {
        let mut file = File::create(path)?;
        let json = serde_json::to_string(self)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}
