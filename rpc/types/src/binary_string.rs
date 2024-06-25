//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- BinaryString
/// TODO
///
/// ```rust
/// use serde::Deserialize;
/// use serde_json::from_str;
/// use cuprate_rpc_types::BinaryString;
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
