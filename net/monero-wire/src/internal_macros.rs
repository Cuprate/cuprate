macro_rules! get_field_from_map {
    ($map:ident, $field_name:expr) => {
        $map.get($field_name)
            .ok_or_else(|| serde::de::Error::missing_field($field_name))?
    };
}

macro_rules! get_val_from_map {
    ($map:ident, $field_name:expr, $get_fn:ident, $expected_ty:expr) => {
        $map.get($field_name)
            .ok_or_else(|| serde::de::Error::missing_field($field_name))?
            .$get_fn()
            .ok_or_else(|| {
                serde::de::Error::invalid_type($map.get_value_type_as_unexpected(), &$expected_ty)
            })?
    };
}

macro_rules! get_internal_val {
    ($value:ident, $get_fn:ident, $expected_ty:expr) => {
        $value.$get_fn().ok_or_else(|| {
            serde::de::Error::invalid_type($value.get_value_type_as_unexpected(), &$expected_ty)
        })?
    };
}

macro_rules! monero_decode_into_serde_err {
    ($ty:ty, $buf:ident) => {
        monero::consensus::deserialize::<$ty>($buf).map_err(serde::de::Error::custom)?
    };
}
