//! Utils for working with [`Transaction`]

use monero_oxide::transaction::{Input, Transaction};

/// Iterates the spent key images of the [`Transaction`].
pub fn tx_key_images(tx: &Transaction) -> impl Iterator<Item = [u8; 32]> + '_ {
    tx.prefix().inputs.iter().filter_map(|input| match input {
        Input::ToKey { key_image, .. } => Some(key_image.to_bytes()),
        Input::Gen(_) => None,
    })
}

/// Calculates the fee of the [`Transaction`].
///
/// # Panics
/// This will panic if the inputs overflow or the transaction outputs too much, so should only
/// be used on known to be valid txs.
pub fn tx_fee(tx: &Transaction) -> u64 {
    let mut fee = 0_u64;

    match &tx {
        Transaction::V1 { prefix, .. } => {
            for input in &prefix.inputs {
                match input {
                    Input::Gen(_) => return 0,
                    Input::ToKey { amount, .. } => {
                        fee = fee.checked_add(amount.unwrap_or(0)).unwrap();
                    }
                }
            }

            for output in &prefix.outputs {
                fee = fee.checked_sub(output.amount.unwrap_or(0)).unwrap();
            }
        }
        Transaction::V2 { proofs, .. } => {
            fee = proofs.as_ref().unwrap().base.fee;
        }
    }

    fee
}

#[cfg(test)]
mod test {
    use monero_oxide::{
        ed25519::CompressedPoint,
        transaction::{NotPruned, Output, Timelock, TransactionPrefix},
    };

    use super::*;

    #[test]
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    fn tx_fee_panic() {
        let input = Input::ToKey {
            amount: Some(u64::MAX),
            key_offsets: vec![],
            key_image: CompressedPoint::from([0; 32]),
        };

        let output = Output {
            amount: Some(u64::MAX),
            key: CompressedPoint::from([0; 32]),
            view_tag: None,
        };

        let tx = Transaction::<NotPruned>::V1 {
            prefix: TransactionPrefix {
                additional_timelock: Timelock::None,
                inputs: vec![input; 2],
                outputs: vec![output],
                extra: vec![],
            },
            signatures: vec![],
        };

        tx_fee(&tx);
    }
}
