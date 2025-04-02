use toml_edit::TableLike;

/// A macro for config structs defined in `cuprated`. This macro generates a function that
/// can insert toml comments created from doc comments on fields.
///
/// # Attributes
/// - `#[flatten = true]`: lets the writer know that the field is flattened into the parent struct.
/// - `#[child = true]`: writes the doc comments for all fields in the child struct.
/// - `#[inline = true]`: inlines the struct into `{}` instead of having a separate `[]` header.
/// - `#[comment_out = true]`: comments out the field.
macro_rules! config_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[flatten = $flat:literal])?
                $(#[child = $child:literal])?
                $(#[inline = $inline:literal])?
                $(#[comment_out = $comment_out:literal])?
                $(#[doc = $doc:expr])*
                $(##[$field_meta:meta])*
                pub $field:ident: $field_ty:ty,
            )*
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            $(
                $(#[doc = $doc])*
                $(#[$field_meta])*
                pub $field: $field_ty,
            )*
        }

        impl $name {
            #[allow(unused_labels, clippy::allow_attributes)]
            pub fn write_docs(doc: &mut dyn ::toml_edit::TableLike) {
                $(

                    'write_field: {
                        let key_str = &stringify!($field);

                        let mut field_prefix = [ $(
                          format!("##{}\n", $doc),
                        )*].concat();

                        $(
                        if $comment_out {
                            field_prefix.push('#');
                        }
                        )?

                        $(
                        if $flat {
                            <$field_ty>::write_docs(doc);
                            break 'write_field;
                        }
                        )?

                        $(
                        if $child {
                            <$field_ty>::write_docs(doc.get_key_value_mut(&key_str).unwrap().1.as_table_like_mut().unwrap());
                        }
                        )?

                        if let Some(table) = doc.entry(&key_str).or_insert_with(|| panic!()).as_table_mut() {
                            $(
                                if $inline {
                                    let mut table = table.clone().into_inline_table();
                                    doc.insert(&key_str, ::toml_edit::Item::Value(::toml_edit::Value::InlineTable(table)));
                                    doc.key_mut(&key_str).unwrap().leaf_decor_mut().set_prefix(field_prefix);
                                    break 'write_field;
                                }
                            )?
                            table.decor_mut().set_prefix(format!("\n{}", field_prefix));
                        }else {
                            doc.key_mut(&key_str).unwrap().leaf_decor_mut().set_prefix(field_prefix);
                        }
                    }
                )*
            }
        }
    };
}

pub(crate) use config_struct;
