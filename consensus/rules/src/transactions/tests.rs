use std::ops::Range;

use curve25519_dalek::{
    constants::{ED25519_BASEPOINT_POINT, EIGHT_TORSION},
    edwards::CompressedEdwardsY,
    EdwardsPoint,
};
use proptest::{collection::vec, prelude::*};

use monero_serai::transaction::Output;

use super::*;
use crate::decomposed_amount::decomposed_amounts;

#[test]
fn test_check_output_amount_v1() {
    for amount in decomposed_amounts() {
        assert!(check_output_amount_v1(*amount, &HardFork::V2).is_ok())
    }

    proptest!(|(amount in any::<u64>().prop_filter("value_decomposed", |val| !is_decomposed_amount(val)))| {
        prop_assert!(check_output_amount_v1(amount, &HardFork::V2).is_err());
        prop_assert!(check_output_amount_v1(amount, &HardFork::V1).is_ok())
    });
}

#[test]
fn test_sum_outputs() {
    let mut output_10 = Output {
        key: CompressedEdwardsY([0; 32]),
        amount: None,
        view_tag: None,
    };

    output_10.amount = Some(10);

    let mut outputs_20 = output_10.clone();
    outputs_20.amount = Some(20);

    let outs = [output_10, outputs_20];

    let sum = sum_outputs(&outs, &HardFork::V16, &TxVersion::RingSignatures).unwrap();
    assert_eq!(sum, 30);

    assert!(sum_outputs(&outs, &HardFork::V16, &TxVersion::RingCT).is_err())
}

#[test]
fn test_decoy_info() {
    let decoy_info = DecoyInfo {
        mixable: 0,
        not_mixable: 0,
        min_decoys: minimum_decoys(&HardFork::V8),
        max_decoys: minimum_decoys(&HardFork::V8) + 1,
    };

    assert!(check_decoy_info(&decoy_info, &HardFork::V8).is_ok());
    assert!(check_decoy_info(&decoy_info, &HardFork::V16).is_err());

    let mut decoy_info = DecoyInfo {
        mixable: 0,
        not_mixable: 0,
        min_decoys: minimum_decoys(&HardFork::V8) - 1,
        max_decoys: minimum_decoys(&HardFork::V8) + 1,
    };

    assert!(check_decoy_info(&decoy_info, &HardFork::V8).is_err());

    decoy_info.not_mixable = 1;
    assert!(check_decoy_info(&decoy_info, &HardFork::V8).is_ok());

    decoy_info.mixable = 2;
    assert!(check_decoy_info(&decoy_info, &HardFork::V8).is_err());

    let mut decoy_info = DecoyInfo {
        mixable: 0,
        not_mixable: 0,
        min_decoys: minimum_decoys(&HardFork::V12),
        max_decoys: minimum_decoys(&HardFork::V12) + 1,
    };

    assert!(check_decoy_info(&decoy_info, &HardFork::V12).is_err());

    decoy_info.max_decoys = decoy_info.min_decoys;
    assert!(check_decoy_info(&decoy_info, &HardFork::V12).is_ok());
}

#[test]
fn test_torsion_ki() {
    for &key_image in EIGHT_TORSION[1..].iter() {
        assert!(check_key_images(&Input::ToKey {
            key_image,
            amount: None,
            key_offsets: vec![],
        })
        .is_err())
    }
}

/// Returns a strategy that resolves to a [`RctType`] that uses
/// BPs(+).
#[allow(unreachable_code)]
#[allow(clippy::diverging_sub_expression)]
fn bulletproof_rct_type() -> BoxedStrategy<RctType> {
    return prop_oneof![
        Just(RctType::Bulletproofs),
        Just(RctType::BulletproofsCompactAmount),
        Just(RctType::Clsag),
        Just(RctType::BulletproofsPlus),
    ]
    .boxed();

    // Here to make sure this is updated when needed.
    match unreachable!() {
        RctType::Null => {}
        RctType::MlsagAggregate => {}
        RctType::MlsagIndividual => {}
        RctType::Bulletproofs => {}
        RctType::BulletproofsCompactAmount => {}
        RctType::Clsag => {}
        RctType::BulletproofsPlus => {}
    };
}

prop_compose! {
    /// Returns a valid prime-order point.
    fn random_point()(bytes in any::<[u8; 32]>()) -> EdwardsPoint {
        EdwardsPoint::mul_base_clamped(bytes)
    }
}

prop_compose! {
    /// Returns a valid torsioned point.
    fn random_torsioned_point()(point in random_point(), torsion in 1..8_usize ) -> EdwardsPoint {
        point + curve25519_dalek::constants::EIGHT_TORSION[torsion]
    }
}

prop_compose! {
    /// Returns a random [`Output`].
    ///
    /// `key` is always valid.
    fn random_out(rct: bool, view_tagged: bool)(
        point in random_point(),
        amount in any::<u64>(),
        view_tag in any::<u8>(),
    ) -> Output {
        Output {
            amount: if rct { None } else { Some(amount) },
            key: point.compress(),
            view_tag: if view_tagged { Some(view_tag) } else { None },
        }
    }
}

prop_compose! {
    /// Returns a random [`Output`].
    ///
    /// `key` is always valid but torsioned.
    fn random_torsioned_out(rct: bool, view_tagged: bool)(
        point in random_torsioned_point(),
        amount in any::<u64>(),
        view_tag in any::<u8>(),
    ) -> Output {
        Output {
            amount: if rct { None } else { Some(amount) },
            key: point.compress(),
            view_tag: if view_tagged { Some(view_tag) } else { None },
        }
    }
}

prop_compose! {
    /// Returns a [`HardFork`] in a specific range.
    fn hf_in_range(range: Range<u8>)(
        hf in range,
    ) -> HardFork {
        HardFork::from_version(hf).unwrap()
    }
}

prop_compose! {
    /// Returns a [`Timelock`] that is locked given a height and time.
    fn locked_timelock(height: u64, time_for_time_lock: u64)(
        timebased in any::<bool>(),
        lock_height in (height+1)..500_000_001,
        time_for_time_lock in (time_for_time_lock+121)..,
    ) -> Timelock {
        if timebased || lock_height > 500_000_000 {
            Timelock::Time(time_for_time_lock)
        } else {
            Timelock::Block(usize::try_from(lock_height).unwrap())
        }
    }
}

prop_compose! {
    /// Returns a [`Timelock`] that is unlocked given a height and time.
    fn unlocked_timelock(height: u64, time_for_time_lock: u64)(
        ty in 0..3,
        lock_height in 0..(height+1),
        time_for_time_lock in 0..(time_for_time_lock+121),
    ) -> Timelock {
        match ty {
            0 => Timelock::None,
            1 => Timelock::Time(time_for_time_lock),
            _ =>  Timelock::Block(usize::try_from(lock_height).unwrap())
        }
    }
}

proptest! {
    #[test]
    fn test_check_output_keys(
        outs in vec(random_out(true, true), 0..16),
        torsioned_outs in vec(random_torsioned_out(false, true), 0..16)
    ) {
        prop_assert!(check_output_keys(&outs).is_ok());
        prop_assert!(check_output_keys(&torsioned_outs).is_ok());
    }

    #[test]
    fn output_types(
        mut view_tag_outs in vec(random_out(true, true), 1..16),
        mut non_view_tag_outs in vec(random_out(true, false), 1..16),
        hf_no_view_tags in hf_in_range(1..14),
        hf_view_tags in hf_in_range(16..17),
    ) {
        prop_assert!(check_output_types(&view_tag_outs, &hf_view_tags).is_ok());
        prop_assert!(check_output_types(&view_tag_outs, &hf_no_view_tags).is_err());


        prop_assert!(check_output_types(&non_view_tag_outs, &hf_no_view_tags).is_ok());
        prop_assert!(check_output_types(&non_view_tag_outs, &hf_view_tags).is_err());

        prop_assert!(check_output_types(&non_view_tag_outs, &HardFork::V15).is_ok());
        prop_assert!(check_output_types(&view_tag_outs, &HardFork::V15).is_ok());
        view_tag_outs.append(&mut non_view_tag_outs);
        prop_assert!(check_output_types(&view_tag_outs, &HardFork::V15).is_err());
    }

    #[test]
    fn test_valid_number_of_outputs(valid_numb_outs in 2..17_usize, rct_type in bulletproof_rct_type()) {
        prop_assert!(check_number_of_outputs(valid_numb_outs, &HardFork::V16, &TxVersion::RingCT, &rct_type).is_ok());
    }

    #[test]
    fn test_invalid_number_of_outputs(numb_outs in 17..usize::MAX, rct_type in bulletproof_rct_type()) {
        prop_assert!(check_number_of_outputs(numb_outs, &HardFork::V16, &TxVersion::RingCT, &rct_type).is_err());
    }

    #[test]
    fn test_check_output_amount_v2(amt in 1..u64::MAX) {
        prop_assert!(check_output_amount_v2(amt).is_err());
        prop_assert!(check_output_amount_v2(0).is_ok())
    }

    #[test]
    fn test_block_unlock_time(height in 1..u64::MAX) {
        prop_assert!(check_block_time_lock(height, height));
        prop_assert!(!check_block_time_lock(height, height - 1));
        prop_assert!(check_block_time_lock(height, height+1));
    }

    #[test]
    fn test_timestamp_time_lock(timestamp in 500_000_001..u64::MAX) {
        prop_assert!(check_timestamp_time_lock(timestamp, timestamp - 120, &HardFork::V16));
        prop_assert!(!check_timestamp_time_lock(timestamp, timestamp - 121, &HardFork::V16));
        prop_assert!(check_timestamp_time_lock(timestamp, timestamp, &HardFork::V16));
    }

    #[test]
    fn test_time_locks(
        mut locked_locks in vec(locked_timelock(5_000, 100_000_000), 1..50),
        mut unlocked_locks in vec(unlocked_timelock(5_000, 100_000_000), 1..50)
    ) {
        assert!(check_all_time_locks(&locked_locks, 5_000, 100_000_000, &HardFork::V16).is_err());
        assert!(check_all_time_locks(&unlocked_locks, 5_000, 100_000_000, &HardFork::V16).is_ok());

        unlocked_locks.append(&mut locked_locks);
        assert!(check_all_time_locks(&unlocked_locks, 5_000, 100_000_000, &HardFork::V16).is_err());
    }

    #[test]
    fn test_check_input_has_decoys(key_offsets in vec(any::<u64>(), 1..10_000)) {
        assert!(check_input_has_decoys(&Input::ToKey {
            key_image: ED25519_BASEPOINT_POINT,
            amount: None,
            key_offsets,
        }).is_ok());

        assert!(check_input_has_decoys(&Input::ToKey {
            key_image: ED25519_BASEPOINT_POINT,
            amount: None,
            key_offsets: vec![],
        }).is_err());
    }
}
