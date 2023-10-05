use std::fmt::Debug;

pub(crate) fn default_false() -> bool {
    false
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_zero<T: TryFrom<u8>>() -> T {
    0.try_into().map_err(|_ |"Couldn't fit 0 into integer type!").unwrap()
}
