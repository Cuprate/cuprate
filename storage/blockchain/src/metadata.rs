use cuprate_pruning::{PruningSeed, CRYPTONOTE_PRUNING_LOG_STRIPES};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{BlockchainError, DATABASE_VERSION};

/// Represents the metadata of the blockchain database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// The version of the database, defaults to `crate::constants::DATABASE_VERSION`.
    db_version: u64,
    /// The seed used for pruning, defaults to `0`.
    pruning_seed: u32,

    #[serde(skip)]
    /// The path to where the metadata file is stored.
    path: PathBuf,
}

impl Metadata {
    const FILE_NAME: &'static str = "metadata.json";

    /// Opens the metadata file, or creates a new one if it doesn't exist yet.
    pub fn get_or_create(db_dir_path: impl AsRef<Path>) -> Result<Self, BlockchainError> {
        let metadata_path = Path::new(db_dir_path.as_ref()).join(Self::FILE_NAME);
        let exists = metadata_path.exists();

        if exists {
            let file = File::open(&metadata_path)?;
            let reader = BufReader::new(file);
            let metadata: Metadata = serde_json::from_reader(reader)?;
            Ok(metadata)
        } else {
            let file = File::create(&metadata_path)?;
            let metadata = Metadata::new(metadata_path);
            metadata.write_to_file(&file)?;
            Ok(metadata)
        }
    }

    /// Gets the stripe index from the pruning seed.
    ///
    /// This will persist the metadata file to disk.
    pub fn set_stripe_idx(&mut self, stripe_idx: u32) -> Result<(), BlockchainError> {
        self.pruning_seed =
            PruningSeed::new_pruned(stripe_idx, CRYPTONOTE_PRUNING_LOG_STRIPES)?.compress();

        self.persist()
    }

    /// Gets the stripe index from the pruning seed.
    ///
    /// Note that this doesn't re-read the file.
    ///
    /// Returns `None` if the seed is not pruned (e.g. the seed is `0`).
    #[inline]
    pub fn get_stripe_idx(&self) -> Result<Option<u32>, BlockchainError> {
        Ok(PruningSeed::decompress(self.pruning_seed)?.get_stripe())
    }

    #[inline]
    pub fn get_db_version(&self) -> u64 {
        self.db_version
    }

    #[inline]
    pub fn is_pruned(&self) -> bool {
        self.pruning_seed != 0
    }

    /// Saves the metadata to the file.
    ///
    /// If the file does not exist, it will error.
    fn persist(&self) -> Result<(), BlockchainError> {
        let file = File::open(&self.path)?;
        self.write_to_file(&file)
    }

    /// Creates a new [`Metadata`] with the given path. Default pruning_seed is `0`.
    #[inline]
    fn new(path: PathBuf) -> Self {
        Self {
            db_version: DATABASE_VERSION,
            pruning_seed: 0,
            path,
        }
    }

    /// Writes the metadata to the given file.
    #[inline]
    fn write_to_file(&self, file: &File) -> Result<(), BlockchainError> {
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, self)?;
        writer.flush()?;
        Ok(())
    }
}
