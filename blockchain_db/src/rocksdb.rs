//!
//! RocksDB implementation.
//! 
//! Database Schema:
//! ---------------------------------------
//!     Column       |        Key       |       Data
//! ---------------------------------------
//! *block*------------------------------------------------------
//!     
//!     blocks                  height        {blob}
//!     heights                hash            height
//!     b_metadata       height         {b_metdata}
//! 
//! *transactions*-----------------------------------------------
//! 
//!     tx_prefix             tx ID            {blob}
//!     tx_prunable       tx ID           {blob}
//!     tx_hash               tx ID             hash
//!     tx_opti_h            hash            height
//!     tx_outputs         tx ID             {amount,output,indices}
//! 
//! *outputs*----------------------------------------------------
//! 
//!     ouputs_txs         op ID           {tx hash, l_index}
//!     outputs_am       amount      {amount output index, metdata}
//! 
//! *spent keys*--------------------------------------------------
//! 
//!     spent_keys      hash               well... obvious?
//! 
//! *tx pool*------------------------------------------------------
//! 
//!     txp_meta          hash               {txp_metadata}
//!     txp_blob            hash              {blob}
//! 
//! *alt blocks*----------------------------------------------------
//! 
//!     alt_blocks         hash                {bock data, block blob}