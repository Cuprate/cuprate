use epee_serde::Value;

pub(crate) fn zero_val<T: From<u8>>() -> T {
    T::from(0_u8)
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_false() -> bool {
    false
}

pub(crate) fn get_field_from_map<E: serde::de::Error>(
    value: &mut Value,
    field_name: &'static str,
) -> Result<Value, E> {
    value
        .get_and_remove(field_name)
        .ok_or(serde::de::Error::missing_field(field_name))
}

pub(crate) fn get_internal_val<E, F, T>(value: Value, get_fn: F, expected_ty: &str) -> Result<T, E>
where
    E: serde::de::Error,
    F: Fn(Value) -> Option<T>,
{
    let err = serde::de::Error::invalid_type(value.get_value_type_as_unexpected(), &expected_ty);
    get_fn(value).ok_or(err)
}

pub(crate) fn get_internal_val_from_map<E, F, T>(
    value: &mut Value,
    field_name: &'static str,
    get_fn: F,
    expected_ty: &str,
) -> Result<T, E>
where
    E: serde::de::Error,
    F: Fn(Value) -> Option<T>,
{
    let val = get_field_from_map(value, field_name)?;
    get_internal_val(val, get_fn, expected_ty)
}
