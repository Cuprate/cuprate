use toml_edit::TableLike;

/// A macro for config structs defined in `cuprated`. This macro generates a function that
/// can insert toml comments created from doc comments on fields.
///
/// # Attributes
/// - `#[child = true]`: writes the doc comments for all fields in the child struct.
/// - `#[inline = true]`: inlines the struct into `{}` instead of having a separate `[]` header.
/// - `#[comment_out = true]`: comments out the field.
///
/// # Invariants
/// Required for this macro to work:
///
/// - struct must implement [`Default`] and `serde`
/// - None of the fields can be [`Option`]
///
/// # Documentation
/// Consider using the following style when adding documentation:
///
/// ```rust
/// struct Config {
///     /// BRIEF DESCRIPTION.
///     ///
///     /// (optional) LONGER DESCRIPTION.
///     ///
///     /// Type         | (optional) FIELD TYPE
///     /// Valid values | EXPRESSION REPRESENTING VALID VALUES
///     /// Examples     | (optional) A FEW EXAMPLE VALUES
///     field: (),
/// }
/// ```
///
/// For example:
/// ```rust
/// struct Config {
///     /// Enable/disable fast sync.
///     ///
///     /// Fast sync skips verification of old blocks by
///     /// comparing block hashes to a built-in hash file,
///     /// disabling this will significantly increase sync time.
///     /// New blocks are still fully validated.
///     ///
///     /// Type         | boolean
///     /// Valid values | true, false
///     fast_sync: bool,
/// }
/// ```
///
/// Language for types:
///
/// | Rust type    | Wording used in user-book |
/// |--------------|---------------------------|
/// | bool         | boolean
/// | u{8-64}      | Number
/// | i{8-64}      | Signed number
/// | f{32,64}     | Floating point number
/// | str, String  | String
/// | enum, struct | `DataStructureName` (e.g. `Duration`) or $DESCRIPTION (e.g. `IP address`)
///
/// If some fields are redundant or unnecessary, do not add them.
///
/// # Field documentation length
/// In order to prevent wrapping/scrollbars in the user book and in editors,
/// add newlines when a documentation line crosses ~70 characters, around this long:
///
/// `----------------------------------------------------------------------`
///
/// # Shared Values
/// Sometimes multiple different configs will overlap in their fields. Manually duplicating the fields
/// between the structs can lead to maintenance issues as the docs need to be kept in sync. Also using
/// serde's `flatten` is not supported with `deny_unknown_fields`: <https://serde.rs/field-attrs.html#flatten>.
///
/// So this macro provides a way to define shared fields between configs, just add a `Shared` object
/// with the wanted fields at the top and all structs afterwards will have those fields inserted.
macro_rules! config_struct {
        (
        $(
        Shared {
             $(
                $(#[child = $s_child:literal])?
                $(#[inline = $s_inline:literal])?
                $(#[comment_out = $s_comment_out:literal])?
                $(#[doc = $s_doc:expr])*
                $(##[$s_field_meta:meta])*
                pub $s_field:ident: $s_field_ty:ty,
            )+
        }
        )?
    ) => {  };
    (
        $(
        Shared {
             $(
                $(#[child = $s_child:literal])?
                $(#[inline = $s_inline:literal])?
                $(#[comment_out = $s_comment_out:literal])?
                $(#[doc = $s_doc:expr])*
                $(##[$s_field_meta:meta])*
                pub $s_field:ident: $s_field_ty:ty,
            )+
        }
        )?

        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[child = $child:literal])?
                $(#[inline = $inline:literal])?
                $(#[comment_out = $comment_out:literal])?
                $(#[doc = $doc:expr])*
                $(##[$field_meta:meta])*
                pub $field:ident: $field_ty:ty,
            )*
        }

        $($tt: tt)*
    ) => {
        $(#[$meta])*
        pub struct $name {
            $(
                $(#[doc = $doc])*
                $(#[$field_meta])*
                pub $field: $field_ty,
            )*
            $($(
                $(#[doc = $s_doc])*
                $(#[$s_field_meta])*
                pub $s_field: $s_field_ty,
            )+)?
        }

        impl $name {
            #[allow(unused_labels, clippy::allow_attributes)]
            pub fn write_docs(doc: &mut dyn ::toml_edit::TableLike) {
                $(
                    crate::config::macros::__write_docs!(
                        $(#[child = $child])?
                        $(#[inline = $inline])?
                        $(#[comment_out = $comment_out])?
                        $(#[doc = $doc])*
                        $(##[$field_meta])*
                        pub $field: $field_ty,
                        doc,
                    );
                )*

                $($(
                    crate::config::macros::__write_docs!(
                        $(#[child = $s_child])?
                        $(#[inline = $s_inline])?
                        $(#[comment_out = $s_comment_out])?
                        $(#[doc = $s_doc])*
                        $(##[$s_field_meta])*
                        pub $s_field: $s_field_ty,
                        doc,
                    );
                )+)?
            }
        }

        crate::config::macros::config_struct!{
            $(
               Shared {
                   $(
                   $(#[child = $s_child])?
                    $(#[inline = $s_inline])?
                    $(#[comment_out = $s_comment_out])?
                    $(#[doc = $s_doc])*
                    $(##[$s_field_meta])*
                    pub $s_field: $s_field_ty,
                   )+
               }
            )?
            $($tt)*
        }
    };
}

macro_rules! __write_docs {
    (
        $(#[child = $child:literal])?
        $(#[inline = $inline:literal])?
        $(#[comment_out = $comment_out:literal])?
        $(#[doc = $doc:expr])*
        $(##[$field_meta:meta])*
        pub $field:ident: $field_ty:ty,

        $document: ident,
    ) => {
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
            if $child {
                <$field_ty>::write_docs($document.get_key_value_mut(&key_str).unwrap().1.as_table_like_mut().unwrap());
            }
            )?

            if let Some(table) = $document.entry(&key_str).or_insert_with(|| panic!()).as_table_mut() {
                $(
                    if $inline {
                        let mut table = table.clone().into_inline_table();
                        $document.insert(&key_str, ::toml_edit::Item::Value(::toml_edit::Value::InlineTable(table)));
                        $document.key_mut(&key_str).unwrap().leaf_decor_mut().set_prefix(field_prefix);
                        break 'write_field;
                    }
                )?
                table.decor_mut().set_prefix(format!("\n{}", field_prefix));
            }else {
                $document.key_mut(&key_str).unwrap().leaf_decor_mut().set_prefix(field_prefix);
            }
        }
    };
}

pub(crate) use {__write_docs, config_struct};
