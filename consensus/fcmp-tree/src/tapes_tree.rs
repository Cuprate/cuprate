use std::{cmp::min, io, path::PathBuf, sync::Arc};

use bytemuck::{Pod, Zeroable};
use cuprate_helper::cast::u64_to_usize;
use helioselene::{Helios, Selene};
use tapes::{
    CachedBlobTape, CachedTapeOpenOptions, FixedSizedTape, Persistence, Tapes, TapesAppend,
    TapesAppendTransaction, TapesRead, TapesTruncate, WholeBlobTape, WholeTapeOpenOptions,
};

use crate::{
    Branch,
    traits::{ChildReader, FcmpCurve, TreeStore, TreeWriter},
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
    pub fn open(config: &FcmpTreeStorageConfig) -> io::Result<Self> {
        let dir = config.path.join("tree");
        let tapes = Arc::new(Tapes::open(&dir)?);
        let mut append = tapes.append();

        let options = || CachedTapeOpenOptions {
            inner: WholeTapeOpenOptions { dir: dir.clone() },
            top_cache_size: config.layer_cache_size,
        };

        let layers = LAYER_NAMES
            .iter()
            .map(|name| append.open_fixed_sized_tape(name, options()).map(Arc::new))
            .collect::<io::Result<_>>()?;

        let blocks = Arc::new(append.open_fixed_sized_tape("blocks", options())?);

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

            truncate.truncate_fixed_sized_tape(layer, new_branches_in_layer)?;
        }

        truncate.truncate_fixed_sized_tape(&self.blocks, to_height)?;

        truncate.commit(Persistence::SyncAll)
    }
}

impl TreeStore for TapesTreeStore {
    type LayerReader = BranchReader;
    type Writer = BranchWriter;

    fn layer_reader(&self, layer_idx: usize) -> Self::LayerReader {
        BranchReader {
            layer: Arc::clone(&self.layers[layer_idx]),
            tapes: Arc::clone(&self.tapes),
        }
    }

    fn writer(&self) -> Self::Writer {
        BranchWriter {
            layers: self.layers.clone(),
            blocks: Arc::clone(&self.blocks),
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
        read_branches(&self.tapes.reader(), &self.layer, start_index, batch_size)
    }

    fn get(&self, index: u64) -> Result<Option<Branch>, io::Error> {
        let tx = self.tapes.reader();

        tx.read_entry(&self.layer, index)
    }
}

pub struct BranchWriter {
    layers: Vec<Arc<FixedSizedTape<Branch, CachedBlobTape<WholeBlobTape>>>>,
    blocks: Arc<FixedSizedTape<TapeBlockInfo, CachedBlobTape<WholeBlobTape>>>,
    tx: TapesAppendTransaction,
}

impl BranchWriter {
    /// Add block's [`TapeBlockInfo`] to the tree.
    pub fn add_blocks(&mut self, blocks: &[TapeBlockInfo]) -> io::Result<()> {
        self.tx.append_entries(&self.blocks, blocks)?;
        Ok(())
    }
}

impl TreeWriter for BranchWriter {
    fn branch_len(&self, layer_idx: usize) -> Result<u64, io::Error> {
        Ok(self
            .tx
            .fixed_sized_tape_len(&self.layers[layer_idx])
            .expect("We created the tape on open"))
    }

    fn get_batch(
        &self,
        layer_idx: usize,
        start_index: u64,
        batch_size: u64,
    ) -> Result<Vec<Branch>, io::Error> {
        read_branches(&self.tx, &self.layers[layer_idx], start_index, batch_size)
    }

    fn append_branches(&mut self, layer_idx: usize, branches: &[Branch]) -> Result<(), io::Error> {
        // Appending nothing would still make the commit sync the layer.
        if !branches.is_empty() {
            self.tx.append_entries(&self.layers[layer_idx], branches)?;
        }

        Ok(())
    }

    fn commit(self) -> Result<(), io::Error> {
        self.tx.commit(Persistence::SyncAll)
    }
}

/// Reads up to `batch_size` branches of a layer from `start_index`.
fn read_branches(
    tx: &impl TapesRead,
    layer: &FixedSizedTape<Branch, CachedBlobTape<WholeBlobTape>>,
    start_index: u64,
    batch_size: u64,
) -> Result<Vec<Branch>, io::Error> {
    let len = tx.fixed_sized_tape_len(layer).unwrap_or_default();

    let buf_len = min(len.saturating_sub(start_index), batch_size);

    if buf_len == 0 {
        return Ok(vec![]);
    }

    let mut buf = vec![[0_u8; 32]; u64_to_usize(buf_len)];

    tx.read_entries(layer, start_index, &mut buf)?;

    Ok(buf)
}
