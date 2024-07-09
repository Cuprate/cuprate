//! JSON string containing binary data.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- BinaryString
/// TODO: we need to figure out a type that (de)serializes correctly, `String` errors with `serde_json`
///
/// ```rust
/// use serde::Deserialize;
/// use serde_json::from_str;
/// use cuprate_rpc_types::misc::BinaryString;
///
/// #[derive(Deserialize)]
/// struct Key {
///     key: BinaryString,
/// }
///
/// let binary = r"�\b����������";
/// let json = format!("{{\"key\":\"{binary}\"}}");
/// let key = from_str::<Key>(&json).unwrap();
/// let binary: BinaryString = key.key;
/// ```
pub type BinaryString = String;

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
