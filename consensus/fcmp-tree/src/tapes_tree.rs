use std::{cmp::min, io, path::PathBuf, sync::Arc};

use bytemuck::{Pod, Zeroable};
use helioselene::{Helios, Selene};
use tapes::{
    CachedBlobTape, CachedTapeOpenOptions, FixedSizedTape, Persistence, Tapes, TapesAppend,
    TapesAppendTransaction, TapesRead, TapesTruncate, WholeBlobTape, WholeTapeOpenOptions,
};

use crate::{
    Branch,
    traits::{ChildReader, FcmpCurve, LayerWriter, TreeStore},
};

const LAYER_NAMES: &[&str] = &[
    "layer1", "layer2", "layer3", "layer4", "layer5", "layer6", "layer7", "layer8", "layer9",
    "layer10", "layer11", "layer12",
];

/// Block information to store with the tree.
#[derive(Pod, Zeroable, Copy, Clone, Default, Debug)]
#[repr(C)]
pub struct TapeBlockInfo {
    pub hash: [u8; 32],
    pub leaves_in_tree: u64,
}

/// Tree store config.
pub struct FcmpTreeStorageConfig {
    pub path: PathBuf,
    pub layer_cache_size: u64,
}

/// A tree store implementation with the tapes database.
pub struct TapesTreeStore {
    pub tapes: Arc<Tapes>,
    layers: Vec<Arc<FixedSizedTape<Branch, CachedBlobTape<WholeBlobTape>>>>,
    pub blocks: Arc<FixedSizedTape<TapeBlockInfo, CachedBlobTape<WholeBlobTape>>>,
}

impl TapesTreeStore {
    /// Open the store or create a new one if one does not exist.
    pub fn open(config: FcmpTreeStorageConfig) -> io::Result<Self> {
        let tapes = Arc::new(Tapes::open(&config.path.join("tree"))?);
        let mut append = tapes.append();

        let layers = LAYER_NAMES
            .iter()
            .map(|name| {
                append
                    .open_fixed_sized_tape(
                        name,
                        CachedTapeOpenOptions {
                            inner: WholeTapeOpenOptions {
                                dir: config.path.join("tree"),
                            },
                            top_cache_size: config.layer_cache_size,
                        },
                    )
                    .map(Arc::new)
            })
            .collect::<io::Result<_>>()?;

        let blocks = Arc::new(append.open_fixed_sized_tape(
            "blocks",
            CachedTapeOpenOptions {
                inner: WholeTapeOpenOptions {
                    dir: config.path.join("tree"),
                },
                top_cache_size: config.layer_cache_size,
            },
        )?);

        append.commit(Persistence::SyncAll)?;

        Ok(Self {
            tapes,
            layers,
            blocks,
        })
    }

    /// Returns if a block at a given height is in the tree.
    pub fn block_in_tree(&self, height: u64, hash: [u8; 32]) -> io::Result<bool> {
        let reader = self.tapes.reader();

        reader
            .read_entry(&self.blocks, height)
            .map(|res| res.is_some_and(|b| b.hash == hash))
    }

    /// Returns the top block in the tree and its height.
    ///
    /// Will be [`None`] if the tree is empty.
    pub fn top_block_in_tree(&self) -> io::Result<Option<(u64, TapeBlockInfo)>> {
        let read = self.tapes.reader();

        let blocks_in_tree = read.fixed_sized_tape_len(&self.blocks);

        if let Some(blocks_in_tree) = blocks_in_tree {
            if blocks_in_tree == 0 {
                return Ok(None);
            }

            let top_block = read.read_entry(&self.blocks, blocks_in_tree - 1)?.unwrap();

            Ok(Some((blocks_in_tree - 1, top_block)))
        } else {
            Ok(None)
        }
    }

    /// Revert the tree to the given chain height.
    ///
    /// # Panics
    ///
    /// This will panic if the genesis block is popped.
    pub fn pop_blocks(&self, to_height: u64) -> io::Result<()> {
        assert_ne!(to_height, 0);

        let mut truncate = self.tapes.truncate();

        let block = truncate.read_entry(&self.blocks, to_height - 1)?;

        let mut new_branches_in_layer = block.map_or(0, |b| b.leaves_in_tree);

        for (layer_idx, layer) in self.layers.iter().enumerate() {
            if layer_idx & 1 == 1 {
                new_branches_in_layer /= Helios::CHUNK_LEN as u64;
            } else {
                new_branches_in_layer /= Selene::CHUNK_LEN as u64;
            }

            truncate.truncate_fixed_sized_tape(layer, new_branches_in_layer)?
        }

        truncate.truncate_fixed_sized_tape(&self.blocks, to_height)?;

        truncate.commit(Persistence::SyncAll)
    }
}

impl TreeStore for TapesTreeStore {
    type LayerReader = BranchReader;
    type LayerWriter = BranchWriter;

    fn layer_reader(&self, layer_idx: usize) -> Self::LayerReader {
        BranchReader {
            layer: self.layers[layer_idx].clone(),
            tapes: self.tapes.clone(),
        }
    }

    fn layer_writer(&self, layer_idx: usize) -> Self::LayerWriter {
        BranchWriter {
            layer: self.layers[layer_idx].clone(),
            blocks: self.blocks.clone(),
            tx: self.tapes.append(),
        }
    }
}

pub struct BranchReader {
    layer: Arc<FixedSizedTape<Branch, CachedBlobTape<WholeBlobTape>>>,
    tapes: Arc<Tapes>,
}

impl ChildReader<Branch> for BranchReader {
    fn get_batch(&mut self, start_index: u64, batch_size: u64) -> Result<Vec<Branch>, io::Error> {
        let tx = self.tapes.reader();

        let len = tx.fixed_sized_tape_len(&self.layer).unwrap_or_default();

        let buf_len = min(len.saturating_sub(start_index), batch_size);

        if buf_len == 0 {
            return Ok(vec![]);
        }

        let mut buf = vec![[0_u8; 32]; buf_len as usize];

        tx.read_entries(&self.layer, start_index, &mut buf)?;

        Ok(buf)
    }

    fn get(&self, index: u64) -> Result<Option<Branch>, io::Error> {
        let tx = self.tapes.reader();

        tx.read_entry(&self.layer, index)
    }
}

pub struct BranchWriter {
    layer: Arc<FixedSizedTape<Branch, CachedBlobTape<WholeBlobTape>>>,
    blocks: Arc<FixedSizedTape<TapeBlockInfo, CachedBlobTape<WholeBlobTape>>>,
    tx: TapesAppendTransaction,
}

impl BranchWriter {
    /// Add block's [`TapeBlockInfo`] to the tree.
    pub fn add_blocks(&mut self, blocks: Vec<TapeBlockInfo>) -> io::Result<()> {
        self.tx.append_entries(&self.blocks, &blocks)?;
        Ok(())
    }
}

impl LayerWriter for BranchWriter {
    fn branch_len(&self) -> Result<u64, io::Error> {
        Ok(self
            .tx
            .fixed_sized_tape_len(&self.layer)
            .expect("We created the tape on open"))
    }

    fn append_branches(&mut self, branches: &[Branch]) -> Result<(), io::Error> {
        self.tx.append_entries(&self.layer, branches)?;

        Ok(())
    }

    fn commit(self) -> Result<(), io::Error> {
        self.tx.commit(Persistence::SyncAll)
    }
}
