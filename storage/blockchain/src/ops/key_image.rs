//! Key image functions.

//---------------------------------------------------------------------------------------------------- Import
use cuprate_database::{DatabaseRo, DatabaseRw, RuntimeError};

use crate::{
    ops::macros::{doc_add_block_inner_invariant, doc_error},
    tables::KeyImages,
    types::KeyImage,
};

//---------------------------------------------------------------------------------------------------- Key image functions
/// Add a [`KeyImage`] to the "spent" set in the database.
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn add_key_image(
    key_image: &KeyImage,
    table_key_images: &mut impl DatabaseRw<KeyImages>,
) -> Result<(), RuntimeError> {
    table_key_images.put(key_image, &())
}

/// Remove a [`KeyImage`] from the "spent" set in the database.
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn remove_key_image(
    key_image: &KeyImage,
    table_key_images: &mut impl DatabaseRw<KeyImages>,
) -> Result<(), RuntimeError> {
    table_key_images.delete(key_image)
}

/// Check if a [`KeyImage`] exists - i.e. if it is "spent".
#[doc = doc_error!()]
#[inline]
pub fn key_image_exists(
    key_image: &KeyImage,
    table_key_images: &impl DatabaseRo<KeyImages>,
) -> Result<bool, RuntimeError> {
    table_key_images.contains(key_image)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use hex_literal::hex;

    use cuprate_database::{Env, EnvInner, TxRw};

    use super::*;

    use crate::{
        tables::{OpenTables, Tables, TablesMut},
        tests::{assert_all_tables_are_empty, tmp_concrete_env, AssertTableLen},
    };

    /// Tests all above key-image functions.
    ///
    /// Note that this doesn't test the correctness of values added, as the
    /// functions have a pre-condition that the caller handles this.
    ///
    /// It simply tests if the proper tables are mutated, and if the data
    /// stored and retrieved is the same.
    #[test]
    fn all_key_image_functions() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let key_images = [
            hex!("be1c87fc8f958f68fbe346a18dfb314204dca7573f61aae14840b8037da5c286"),
            hex!("c5e4a592c11f34a12e13516ab2883b7c580d47b286b8fe8b15d57d2a18ade275"),
            hex!("93288b646f858edfb0997ae08d7c76f4599b04c127f108e8e69a0696ae7ba334"),
            hex!("726e9e3d8f826d24811183f94ff53aeba766c9efe6274eb80806f69b06bfa3fc"),
        ];

        // Add.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for key_image in &key_images {
                println!("add_key_image(): {}", hex::encode(key_image));
                add_key_image(key_image, tables.key_images_mut()).unwrap();
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        // Assert all reads are OK.
        {
            let tx_ro = env_inner.tx_ro().unwrap();
            let tables = env_inner.open_tables(&tx_ro).unwrap();

            // Assert only the proper tables were added to.
            AssertTableLen {
                key_images: tables.key_images().len().unwrap(),
                ..Default::default()
            }
            .assert(&tables);

            for key_image in &key_images {
                println!("key_image_exists(): {}", hex::encode(key_image));
                key_image_exists(key_image, tables.key_images()).unwrap();
            }
        }

        // Remove.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for key_image in key_images {
                println!("remove_key_image(): {}", hex::encode(key_image));
                remove_key_image(&key_image, tables.key_images_mut()).unwrap();
                assert!(!key_image_exists(&key_image, tables.key_images()).unwrap());
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        assert_all_tables_are_empty(&env);
    }
}
