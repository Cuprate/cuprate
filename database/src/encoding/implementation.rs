//! ### Implementation sub-module
//! This sub-module contains Encode and Decode implementation of all default database types. Sometimes bincode is used to encode types that are slower
//! with a manual implementation. It also happen that we prefer to use bincode because of types complexity. If you wish to add your own storage and database schema, please keep your specific types in **your** implementation folder.

use std::{fmt::Debug, io::Write};

use monero::{Hash, PublicKey, util::ringct::Key, consensus::{Encodable, Decodable}};
use crate::{types::{BlockMetadata, TransactionPruned, OutputMetadata, TxIndex, AltBlock, TxOutputIdx}, BINCODE_CONFIG};
use super::{Encode, Error, Decode, compat::Compat, Buffer};

/// A macro for idiomatic implementation of encode|decode for integer primitives
macro_rules! impl_encode_decode_integer {
	($int:ty, $size:expr) => {
		/// Manual implementation of encoding for $int
		impl Encode for $int {
			type Output = [u8; $size];

			fn encode(&self) -> Result<Self::Output, Error> {
				Ok(self.to_le_bytes())
			}
		}

		/// Manual implementation of decoding for $int
		impl Decode for $int {
	
			fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
				Ok(<$int>::from_le_bytes(src.as_ref()[0..$size].try_into().map_err(Error::TryInto)?))
			}
		}
	};
}

impl_encode_decode_integer!(u8, 1);
impl_encode_decode_integer!(u16, 2);
impl_encode_decode_integer!(u32, 4);
impl_encode_decode_integer!(u64, 8);
impl_encode_decode_integer!(u128, 16);

/// Manual implementation of encoding for (A,B)
impl<A,B> Encode for (A,B) where A: Encode, B: Encode {
    type Output = Vec<u8>;

    fn encode(&self) -> Result<Self::Output, Error> {
		let mut buf = Self::Output::new();
		buf.write_all(self.0.encode()?.as_ref())?;
		buf.write_all(self.1.encode()?.as_ref())?;
        Ok(buf)
    }
}

/// Manual implementation of decoding for (A,B)
/// SAFETY: SubKey must always be of a stable size
impl<A,B> Decode for (A,B) where A: Decode, B: Decode {

    fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
		let split = src.as_ref().split_at(core::mem::size_of::<A>());
		let a = A::decode::<&[u8]>(&split.0)?;
		let b = B::decode::<&[u8]>(&split.1)?;
		Ok((
			a,
			b
		))
    }
}

/// Manual implementation of encoding for ()
impl Encode for () {
	type Output = [u8; 0];

	fn encode(&self) -> Result<Self::Output, Error> {
		Ok([])
	}
}

/// Manual implementation of decoding for ()
impl Decode for () {
	
	fn decode<S: AsRef<[u8]> + Send + Sync>(_src: &S) -> Result<Self, Error> {
		Ok(())
	}
}

/// Manual implementation of encoding for Vec<u8>
impl Encode for Vec<u8> {
	type Output = Vec<u8>;

	fn encode(&self) -> Result<Self::Output, Error> {
		Ok(self.clone())
	}
}

/// Manual implementation of decoding for Vec<u8>
impl Decode for Vec<u8> {
	
	fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
		Ok(src.as_ref().to_vec())
	}
}

// Bincode's implementation of encoding for any Compat type
impl<T: Debug + Encodable + Decodable + Sync + Send> Encode for Compat<T> {
	type Output = Vec<u8>;

	fn encode(&self) -> Result<Self::Output, Error> {
		bincode::encode_to_vec(self, BINCODE_CONFIG).map_err(Into::into)
	}
}

// Bincode's implementation of decoding for any Compat type
impl<T: Debug + Encodable + Decodable + Sync + Send> Decode for Compat<T> {

	fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Compat<T>, Error> {
		bincode::decode_from_slice(src.as_ref(), BINCODE_CONFIG).map(|o| o.0).map_err(Into::into)
	}
}

/// Manual implementation of encoding for monero-rs' Hash
impl Encode for Hash {
	type Output = [u8; 32];

	fn encode(&self) -> Result<Self::Output, Error> {
        Ok(self.0)
    }
}

/// Manual implementation of decoding for monero-rs' Hash
impl Decode for Hash {
	
	fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
		Ok(Hash(src.as_ref()[0..32].try_into().map_err(Error::TryInto)?))
	}
}

/// Manual implementation of encoding for BlockMetadata
impl Encode for BlockMetadata {
	type Output = [u8; 88];

	fn encode(&self) -> Result<Self::Output, Error> {
        let mut buf = Self::Output::new();
		buf[0..8].copy_from_slice(&self.timestamp.to_le_bytes());
		buf[8..16].copy_from_slice(&self.total_coins_generated.to_le_bytes());
		buf[16..24].copy_from_slice(&self.weight.to_le_bytes());
		buf[24..40].copy_from_slice(&self.cumulative_difficulty.to_le_bytes());
		buf[40..72].copy_from_slice(&self.block_hash.0.0); // < When Hash will be implemented pls remove the Compat this
		buf[72..80].copy_from_slice(&self.cum_rct.to_le_bytes());
		buf[80..88].copy_from_slice(&self.long_term_block_weight.to_le_bytes());
		Ok(buf)
    }
}

/// Manual implementation of decoding for BlockMetadata
impl Decode for BlockMetadata {

    fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
        let src = src.as_ref();
		Ok(
			BlockMetadata {
				timestamp: u64::from_le_bytes(src[0..8].try_into().map_err(Error::TryInto)?),
				total_coins_generated: u64::from_le_bytes(src[8..16].try_into().map_err(Error::TryInto)?),
				weight: u64::from_le_bytes(src[16..24].try_into().map_err(Error::TryInto)?),
				cumulative_difficulty: u128::from_le_bytes(src[24..40].try_into().map_err(Error::TryInto)?),
				block_hash: Compat(Hash::from_slice(src[40..72].try_into().map_err(Error::Infallible)?)), // < When Hash will be implemented pls remove the Compat this
				cum_rct: u64::from_le_bytes(src[72..80].try_into().map_err(Error::TryInto)?),
				long_term_block_weight: u64::from_le_bytes(src[80..88].try_into().map_err(Error::TryInto)?),
			}
		)
    }
}

/// Bincode's implementation of encoding for BlockMetadata
impl Encode for AltBlock {
    type Output = Vec<u8>;

    fn encode(&self) -> Result<Self::Output, Error> {
		bincode::encode_to_vec(self, BINCODE_CONFIG).map_err(Into::into)
    }
}

/// Manual implementation of decoding for BlockMetadata
impl Decode for AltBlock {
	
	fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
		bincode::decode_from_slice(src.as_ref(), BINCODE_CONFIG).map(|o| o.0).map_err(Into::into)
	}
}

/// Bincode's implementation of encoding for TransactionPruned
impl Encode for TransactionPruned {
	type Output = Vec<u8>;

	fn encode(&self) -> Result<Self::Output, Error> {
        bincode::encode_to_vec(self, BINCODE_CONFIG).map_err(Into::into)
    }
}

/// Bincode's implementation of decoding for TransactionPruned
impl Decode for TransactionPruned {

    fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
        bincode::decode_from_slice(src.as_ref(), BINCODE_CONFIG).map(|o| o.0).map_err(Into::into)
    }
}

/// Manual implementation of encoding for TxIndex
impl Encode for TxIndex {
	type Output = [u8; 24];

	fn encode(&self) -> Result<Self::Output, Error> {
        let mut buf = Self::Output::new();
		buf[0..8].copy_from_slice(&self.tx_id.to_le_bytes());
		buf[8..16].copy_from_slice(&self.unlock_time.to_le_bytes());
		buf[16..24].copy_from_slice(&self.height.to_le_bytes());
		Ok(buf)
    }
}

/// Manual implementation of decoding for TxIndex
impl Decode for TxIndex {

    fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
		let src = src.as_ref();
		Ok(
			TxIndex {
				tx_id: u64::from_le_bytes(src[0..8].try_into().map_err(Error::TryInto)?),
				unlock_time: u64::from_le_bytes(src[8..16].try_into().map_err(Error::TryInto)?),
				height: u64::from_le_bytes(src[16..24].try_into().map_err(Error::TryInto)?),
			}
		)
    }
}

/// Bincode's implementation of encoding for TxOutputIdx
impl Encode for TxOutputIdx {
	type Output = Vec<u8>;

	fn encode(&self) -> Result<Self::Output, Error> {
		bincode::encode_to_vec(self, BINCODE_CONFIG).map_err(Into::into)
	}
}

/// Bincode's implementation of decoding for TransactionPruned
impl Decode for TxOutputIdx {

    fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
        bincode::decode_from_slice(src.as_ref(), BINCODE_CONFIG).map(|o| o.0).map_err(Into::into)
    }
}

/// Manual implementation of encoding for OutputMetadata
impl Encode for OutputMetadata {
	type Output = [u8; 120];

	fn encode(&self) -> Result<Self::Output, Error> {
		let mut buf = Self::Output::new();
		buf[0..32].copy_from_slice(&self.tx_hash.0.0);
		buf[32..40].copy_from_slice(&self.local_index.to_le_bytes());
		if let Some(pubkey) = &self.pubkey {
			buf[40..72].copy_from_slice(&pubkey.to_bytes());
		} else {
			buf[40..72].copy_from_slice(&[0u8; 32]);
		}
		buf[72..80].copy_from_slice(&self.unlock_time.to_le_bytes());
		buf[80..88].copy_from_slice(&self.height.to_le_bytes());
		if let Some(commitment) = &self.commitment {
			buf[88..120].copy_from_slice(&commitment.key);
		} else {
			buf[88..120].copy_from_slice(&[0u8; 32]);
		}
		Ok(buf)
    }
}

/// Manual implementation of decoding for OutputMetadata
impl Decode for OutputMetadata {

    fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error> {
		let src = src.as_ref();
        let mut out_meta = OutputMetadata {
            tx_hash: Compat(Hash::from_slice(src[0..32].try_into().map_err(Error::Infallible)?)),
            local_index: u64::from_le_bytes(src[32..40].try_into().map_err(Error::TryInto)?),
            pubkey: None,
            unlock_time: u64::from_le_bytes(src[72..80].try_into().map_err(Error::TryInto)?),
            height: u64::from_le_bytes(src[80..88].try_into().map_err(Error::TryInto)?),
            commitment: None,
        };

		let pubkey_data: [u8; 32] = src[40..72].try_into().map_err(Error::TryInto)?;
		if pubkey_data != [0u8; 32] {
			out_meta.pubkey = Some(Compat(PublicKey::from_slice(&pubkey_data).map_err(Error::MoneroKey)?))
		}
		let commitment_data: [u8; 32] = src[88..120].try_into().map_err(Error::TryInto)?;
		if commitment_data != [0u8; 32] {
			out_meta.commitment = Some(Compat(Key{key: commitment_data}))
		}
		Ok(out_meta)
    }
}

//////////////////////////////////////////////////////////////////////
//=/=/=/=/=/=/=/                TEST                =/=/=/=/=/=/=/=/=/
//////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
	#![allow(clippy::expect_used)]
    use monero::{Hash, TransactionPrefix, util::ringct::{RctSig, RctSigBase, RctType, EcdhInfo, CtKey, Key}, VarInt, blockdata::transaction::{RawExtraField, TxOutTarget, KeyImage}, TxOut, Amount, TxIn, cryptonote::hash::Hash8, PublicKey, BlockHeader, Block, Transaction};
    use crate::{types::{BlockMetadata, TransactionPruned, OutputMetadata, TxIndex, AltBlock}, encoding::compat::Compat, encoding::{Encode, Decode}};

	const HASH: Hash = Hash([
		0x5c, 0x5e, 0x69, 0xd8, 0xfc, 0x0d, 0x22, 0x6a, 0x60, 0x91, 0x47, 0xda, 0x98, 0x36,
		0x06, 0x00, 0xf4, 0xea, 0x49, 0xcc, 0x49, 0x45, 0x2c, 0x5e, 0xf8, 0xba, 0x20, 0xf5,
		0x93, 0xd4, 0x80, 0x7d,
	]);

	#[test]
	fn tuple_encode_impl() {
		let height = 2800000u64;
		// block_height
		let encoded = <(Hash,u64) as Encode>::encode(&(HASH,height)).expect("Failed to encode hash, integer tuple");
		let decoded: (Hash, u64) = <(Hash,u64) as Decode>::decode(&encoded).expect("Failed to decode hash, integer tuple");
		assert_eq!(decoded, (HASH,height));

		// spentkeys
		let encoded = <((), Hash) as Encode>::encode(&((), HASH)).expect("Failed to encode None, Hash tuple");
		let decoded: ((), Hash) = <((), Hash) as Decode>::decode(&encoded).expect("Failed to decode None, hash tuple");
		assert_eq!(decoded, ((), HASH))
	}

	#[test]
	fn compatblock_encode_impl() {

		// https://xmrchain.net/block/2869970
		let compat_block = Compat(Block { 
			header: BlockHeader { 
				major_version: VarInt(16), 
				minor_version: VarInt(16), 
				timestamp: VarInt(1682181761), 
				prev_id: Hash([0x48, 0x8a, 0x7b, 0x4a, 0x20, 0xb9, 0x05, 0x47, 0x99, 0x0c, 0x9c, 0x7a, 0x22, 0x2a, 0xa9, 0x10, 0x37, 0xf5, 0x12, 0x12, 0x19, 0x6d, 0x31, 0x99, 0xef, 0xb3, 0xb1, 0xd0, 0x9a, 0x95, 0xd8, 0x21]),
				nonce: 229626 
			}, 
			miner_tx: Transaction { 
				prefix: TransactionPrefix { 
					version: VarInt(2), 
					unlock_time: VarInt(2870030), 
					inputs: [
						TxIn::Gen { height: VarInt(2869970) }
					].to_vec(), 
					outputs: [
						TxOut { 
							amount: VarInt(600208840000), 
							target: TxOutTarget::ToTaggedKey { 
								key: [107, 63, 219, 75, 138, 7, 140, 171, 187, 175, 210, 205, 203, 8, 133, 118, 53, 95, 41, 156, 201, 226, 98, 176, 23, 6, 235, 254, 194, 54, 219, 120], 
								view_tag: 36 
							} 
						}
					].to_vec(), 
					extra: RawExtraField([1, 45, 128, 129, 170, 44, 223, 152, 94, 211, 62, 43, 222, 209, 6, 108, 219, 219, 196, 99, 223, 119, 245, 101, 227, 192, 173, 145, 215, 181, 126, 227, 135, 2, 17, 0, 0, 0, 99, 243, 82, 10, 199, 0, 0, 0, 0, 0, 0, 0, 0, 0].to_vec()) 
				}, 
				signatures: [].to_vec(), 
				rct_signatures: RctSig { 
					sig: Some(RctSigBase { 
						rct_type: RctType::Null, 
						txn_fee: Amount::from_pico(0), 
						pseudo_outs: [].to_vec(), 
						ecdh_info: [].to_vec(), 
						out_pk: [].to_vec() }), 
					p: None 
				} 
			}, 
			tx_hashes: [
				Hash([0x10, 0xd2, 0x1b, 0x56, 0xec, 0xfd, 0x0e, 0x7c, 0x0d, 0x7e, 0xe0, 0x60, 0x46, 0x04, 0x05, 0xe2, 0xf3, 0x24, 0xda, 0x90, 0x92, 0xb1, 0xca, 0xa5, 0xc7, 0x43, 0xc3, 0x29, 0x51, 0x82, 0x4a, 0xc3]), 
				Hash([0x65, 0xf1, 0x14, 0x02, 0xb0, 0x2b, 0x22, 0x2b, 0x42, 0x62, 0xa1, 0x48, 0x97, 0xb3, 0xa0, 0xb3, 0xc0, 0x7b, 0x0c, 0xff, 0x38, 0x2c, 0x06, 0xb2, 0x68, 0xcd, 0x4e, 0x8a, 0xff, 0x9e, 0x43, 0xf1])
				].to_vec()
			}
		);

		let encoded = compat_block.encode().expect("Failed to encode compat block");
		let decoded: Compat<Block> = Compat::decode(&encoded).expect("Failed to decode into compat block");
		assert_eq!(decoded, compat_block)
	}

	#[test]
	fn hash_encode_impl() {

		let encoded = HASH.encode().expect("Failed to encode Hash");
		let decoded = Hash::decode(&encoded).expect("Failed to decode Hash");
		assert_eq!(decoded, HASH);
	}

	#[test]
	fn blockmetadata_encode_impl() {

		const BLK_META: BlockMetadata = BlockMetadata { 
			timestamp: 1, 
			total_coins_generated: 17, 
			weight: 4300, 
			cumulative_difficulty: 965904, 
			block_hash: Compat(Hash([
				0x5c, 0x5e, 0x69, 0xd8, 0xfc, 0x0d, 0x22, 0x6a, 0x60, 0x91, 0x47, 0xda, 0x98, 0x36,
				0x06, 0x00, 0xf4, 0xea, 0x49, 0xcc, 0x49, 0x45, 0x2c, 0x5e, 0xf8, 0xba, 0x20, 0xf5,
				0x93, 0xd4, 0x80, 0x7d,
			])), 
			cum_rct: 7, 
			long_term_block_weight: 30 
		};

		let encoded = BLK_META.encode().expect("Failed to encode block metadata");
		let decoded = BlockMetadata::decode(&encoded).expect("Failed to decode block metadata");
		assert_eq!(decoded, BLK_META)
	}

	#[test]
	fn altblock_encode_impl() {

		// https://xmrchain.net/block/2869970
		let pseudo_alt_block = AltBlock {
			height: 2869970,
			cumulative_difficulty: 320877633531,
			cumulative_weight: 3859,
			already_generated_coins: 18269179874465731653,
			block: Compat(Block { 
				header: BlockHeader { 
					major_version: VarInt(16), 
					minor_version: VarInt(16), 
					timestamp: VarInt(1682181761), 
					prev_id: Hash([0x48, 0x8a, 0x7b, 0x4a, 0x20, 0xb9, 0x05, 0x47, 0x99, 0x0c, 0x9c, 0x7a, 0x22, 0x2a, 0xa9, 0x10, 0x37, 0xf5, 0x12, 0x12, 0x19, 0x6d, 0x31, 0x99, 0xef, 0xb3, 0xb1, 0xd0, 0x9a, 0x95, 0xd8, 0x21]),
					nonce: 229626 
				}, 
				miner_tx: Transaction { 
					prefix: TransactionPrefix { 
						version: VarInt(2), 
						unlock_time: VarInt(2870030), 
						inputs: [
							TxIn::Gen { height: VarInt(2869970) }
						].to_vec(), 
						outputs: [
							TxOut { 
								amount: VarInt(600208840000), 
								target: TxOutTarget::ToTaggedKey { 
									key: [107, 63, 219, 75, 138, 7, 140, 171, 187, 175, 210, 205, 203, 8, 133, 118, 53, 95, 41, 156, 201, 226, 98, 176, 23, 6, 235, 254, 194, 54, 219, 120], 
									view_tag: 36 
								} 
							}
						].to_vec(), 
						extra: RawExtraField([1, 45, 128, 129, 170, 44, 223, 152, 94, 211, 62, 43, 222, 209, 6, 108, 219, 219, 196, 99, 223, 119, 245, 101, 227, 192, 173, 145, 215, 181, 126, 227, 135, 2, 17, 0, 0, 0, 99, 243, 82, 10, 199, 0, 0, 0, 0, 0, 0, 0, 0, 0].to_vec()) 
					}, 
					signatures: [].to_vec(), 
					rct_signatures: RctSig { 
						sig: Some(RctSigBase { 
							rct_type: RctType::Null, 
							txn_fee: Amount::from_pico(0), 
							pseudo_outs: [].to_vec(), 
							ecdh_info: [].to_vec(), 
							out_pk: [].to_vec() }), 
						p: None 
					} 
				}, 
				tx_hashes: [
					Hash([0x10, 0xd2, 0x1b, 0x56, 0xec, 0xfd, 0x0e, 0x7c, 0x0d, 0x7e, 0xe0, 0x60, 0x46, 0x04, 0x05, 0xe2, 0xf3, 0x24, 0xda, 0x90, 0x92, 0xb1, 0xca, 0xa5, 0xc7, 0x43, 0xc3, 0x29, 0x51, 0x82, 0x4a, 0xc3]), 
					Hash([0x65, 0xf1, 0x14, 0x02, 0xb0, 0x2b, 0x22, 0x2b, 0x42, 0x62, 0xa1, 0x48, 0x97, 0xb3, 0xa0, 0xb3, 0xc0, 0x7b, 0x0c, 0xff, 0x38, 0x2c, 0x06, 0xb2, 0x68, 0xcd, 0x4e, 0x8a, 0xff, 0x9e, 0x43, 0xf1])
					].to_vec()
				}
			)
		};

		let encoded = pseudo_alt_block.encode().expect("Failed to encode the alt block");
		let decoded = AltBlock::decode(&encoded).expect("Failed to decode the alt block");
		assert_eq!(decoded, pseudo_alt_block)
	}

	#[test]
	fn txpruned_encode_impl() {

		let tx_pruned: TransactionPruned = TransactionPruned { 
			prefix: TransactionPrefix { 
				version: VarInt(2), 
				unlock_time: VarInt(0), 
				inputs: [
					TxIn::ToKey { 
						amount: VarInt(0), 
						key_offsets: [VarInt(63031988), VarInt(2569197), VarInt(772228), VarInt(2522550), VarInt(1553761), VarInt(377691), VarInt(502110), VarInt(207568), VarInt(85768), VarInt(248351), VarInt(34894), VarInt(8068), VarInt(2823), VarInt(1047), VarInt(1030), VarInt(3378)].to_vec(), 
						k_image: KeyImage { image: Hash([0xc2, 0x6f, 0x65, 0xc0, 0x2d, 0x68, 0x12, 0x73, 0xb2, 0xc2, 0x64, 0xff, 0xe3, 0x0a, 0xfd, 0xa5, 0x35, 0x3e, 0x51, 0x43, 0xca, 0xa9, 0x38, 0x63, 0x2a, 0x92, 0xaf, 0x89, 0x55, 0xc3, 0x1b, 0x97]) } }
					].to_vec(), 
				outputs: [
					TxOut { 
						amount: VarInt(0), 
						target: TxOutTarget::ToTaggedKey { 
							key: [207, 28, 183, 85, 7, 58, 81, 205, 53, 9, 191, 141, 209, 70, 58, 30, 38, 225, 212, 68, 14, 4, 216, 204, 101, 163, 66, 156, 101, 143, 255, 196], 
							view_tag: 134 
						} 
					}, 
					TxOut { 
						amount: VarInt(0), 
						target: TxOutTarget::ToTaggedKey { 
							key: [254, 66, 159, 187, 180, 41, 78, 252, 85, 255, 154, 55, 239, 222, 199, 37, 159, 210, 71, 186, 188, 46, 134, 181, 236, 221, 173, 43, 93, 50, 138, 249], 
							view_tag: 221 
						} 
					}
				].to_vec(), 
				extra: RawExtraField([1, 34, 67, 111, 182, 199, 28, 219, 56, 238, 143, 188, 101, 103, 205, 139, 160, 144, 226, 34, 92, 235, 221, 75, 38, 7, 104, 255, 108, 208, 1, 184, 169, 2, 9, 1, 84, 62, 77, 107, 119, 22, 148, 222].to_vec()) 
			}, 
			rct_signatures: RctSig { 
				sig: Some(RctSigBase {
					rct_type: RctType::BulletproofPlus, 
					txn_fee: Amount::from_pico(30720000), 
					pseudo_outs: [].to_vec(), 
					ecdh_info: [
						EcdhInfo::Bulletproof { amount: Hash8([0xf2, 0xc8, 0x10, 0x89, 0xef, 0xf9, 0x37, 0x3b]) }, 
						EcdhInfo::Bulletproof { amount: Hash8([0x10, 0xc1, 0xc0, 0x8c, 0xf0, 0x99, 0x81, 0xe4]) }
					].to_vec(), 
					out_pk: [
						CtKey { mask: Key { key: [0x73, 0xde, 0xf7, 0x29, 0x80, 0xdb, 0xf1, 0xf9, 0xc6, 0xd6, 0x4b, 0x1f, 0x52, 0xe1, 0x01, 0x9e, 0xb7, 0xe2, 0xdc, 0x7e, 0xe4, 0xbf, 0xd3, 0x4f, 0x2b, 0xdc, 0x5f, 0x7c, 0x6d, 0x0e, 0xa2, 0xaa] } }, 
						CtKey { mask: Key { key: [0x44, 0x25, 0x3e, 0x15, 0x8b, 0xb6, 0xf6, 0x98, 0x24, 0x9c, 0xac, 0xc5, 0x14, 0x91, 0x55, 0x09, 0x08, 0x6a, 0xed, 0x70, 0x3f, 0xbd, 0xac, 0x91, 0x31, 0xea, 0x44, 0x98, 0xc8, 0xf1, 0x00, 0x25] } }
					].to_vec()
				}), 
				p: None 
			} 
		};

		let encoded = tx_pruned.encode().expect("Failed to encode transaction pruned");
		let decoded = TransactionPruned::decode(&encoded).expect("Failed to decode transaction pruned");
		assert_eq!(tx_pruned, decoded)
	}

	#[test]
	fn txindex_encode_impl() {
		const TX_INDEX: TxIndex = TxIndex {
			tx_id: 76767676,
			unlock_time: 0,
			height: 763456,
		};

		let encoded = TX_INDEX.encode().expect("Failed to encode tx index");
		let decoded = TxIndex::decode(&encoded).expect("Failed to decode tx index");
		assert_eq!(decoded, TX_INDEX)
	}

	#[test]
	fn outputmetadata_encode_impl() {
		
		let out_meta: OutputMetadata = OutputMetadata {
			tx_hash: Compat(Hash([0xa8, 0xa4, 0xa0, 0xe4, 0xcc, 0xd9, 0xe3, 0x91, 0x4a, 0x35, 0xa5, 0x43, 0x33, 0x51, 0xbf, 0x94, 0xfe, 0x1a, 0xec, 0x51, 0x71, 0x65, 0xd6, 0x09, 0x89, 0xa6, 0xc2, 0x0d, 0x4f, 0x2e, 0xdb, 0x28])),
			local_index: 0,
			pubkey: Some(Compat(PublicKey::from_slice(&[0xcf, 0x1c, 0xb7, 0x55, 0x07, 0x3a, 0x51, 0xcd, 0x35, 0x09, 0xbf, 0x8d, 0xd1, 0x46, 0x3a, 0x1e, 0x26, 0xe1, 0xd4, 0x44, 0x0e, 0x04, 0xd8, 0xcc, 0x65, 0xa3, 0x42, 0x9c, 0x65, 0x8f, 0xff, 0xc4]).unwrap())),
			unlock_time: 0,
			height: 2867958,
			commitment: Some(Compat(Key { key: [0x73, 0xde, 0xf7, 0x29, 0x80, 0xdb, 0xf1, 0xf9, 0xc6, 0xd6, 0x4b, 0x1f, 0x52, 0xe1, 0x01, 0x9e, 0xb7, 0xe2, 0xdc, 0x7e, 0xe4, 0xbf, 0xd3, 0x4f, 0x2b, 0xdc, 0x5f, 0x7c, 0x6d, 0x0e, 0xa2, 0xaa] })),
		};
		
		let encoded = out_meta.encode().expect("Failed to encode output metadata");
		let decoded = OutputMetadata::decode(&encoded).expect("Failed to decode output metadata");
		assert_eq!(decoded, out_meta)
	}
}