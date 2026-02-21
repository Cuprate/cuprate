use std::{collections::HashMap, sync::Mutex};

use fjall::{KeyspaceCreateOptions, KvSeparationOptions};

pub struct TxpoolDatabase {
    pub(crate) fjall_database: fjall::Database,

    pub(crate) tx_blobs: fjall::Keyspace,
    pub(crate) tx_infos: fjall::Keyspace,
    pub(crate) spent_key_images: fjall::Keyspace,
    pub(crate) known_blob_hashes: fjall::Keyspace,
    pub(crate) metadata: fjall::Keyspace,

    pub(crate) in_progress_key_images: Mutex<HashMap<[u8; 32], [u8; 32]>>,
}

impl TxpoolDatabase {
    pub fn open_with_database(fjall_database: fjall::Database) -> fjall::Result<Self> {
        let s = Self {
            tx_blobs: fjall_database.keyspace("tx_blobs", || {
                KeyspaceCreateOptions::default().with_kv_separation(Some(
                    KvSeparationOptions::default().separation_threshold(3_000),
                ))
            })?,
            tx_infos: fjall_database.keyspace("tx_infos", KeyspaceCreateOptions::default)?,
            spent_key_images: fjall_database
                .keyspace("spent_key_images", KeyspaceCreateOptions::default)?,
            known_blob_hashes: fjall_database
                .keyspace("known_blob_hashes", KeyspaceCreateOptions::default)?,
            metadata: fjall_database.keyspace("metadata", KeyspaceCreateOptions::default)?,
            fjall_database,
            in_progress_key_images: Mutex::new(HashMap::new()),
        };

        Ok(s)
    }
}
