pub use bytes;
pub use paste::paste;

/// Macro to derive [`EpeeObject`](crate::EpeeObject) for structs.
///
/// ### Basic Usage:
///
/// ```rust
/// // mod visibility is here because of Rust visibility weirdness, you shouldn't need this unless defined in a function.
/// // see: <https://github.com/rust-lang/rust/issues/64079>
/// mod visibility {
///
///     use cuprate_epee_encoding::epee_object;
///
///     struct Example {
///         a: u8
///     }    
///
///     epee_object!(
///         Example,
///         a: u8,
///     );
/// }
/// ```
///
/// ### Advanced Usage:
///
/// ```rust
/// // mod visibility is here because of Rust visibility weirdness, you shouldn't need this unless defined in a function.
/// // see: <https://github.com/rust-lang/rust/issues/64079>
/// mod visibility {
///
///     use cuprate_epee_encoding::epee_object;
///
///     struct Example {
///         a: u8,
///         b: u8,
///         c: u8,
///         d: u8,
///         e_f: Example2
///     }
///
///     struct Example2 {
///         e: u8
///     }
///
///     epee_object!(
///         Example2,
///         e: u8,
///     );
///
///     epee_object!(
///         Example,
///         // `("ALT-NAME")` changes the name of the field in the encoded data.
///         a("A"): u8,
///         // `= VALUE` sets a default value that this field will be set to if not in the data
///         // when encoding this field will be skipped if equal to the default.
///         b: u8 = 0,
///         // `as ALT-TYPE` encodes the data using the alt type, the alt type must impl Into<Type> and From<&Type>
///         c: u8 as u8,
///         // `=> read_fn, write_fn, should_write_fn,` allows you to specify alt field encoding functions.
///         //  for the required args see the default functions, which are used here:
///         d: u8 => cuprate_epee_encoding::read_epee_value, cuprate_epee_encoding::write_field, <u8 as cuprate_epee_encoding::EpeeValue>::should_write,
///         // `!flatten` can be used on fields which are epee objects, and it flattens the fields of that object into this object.
///         // So for this example `e_f` will not appear in the data but e will.
///         // You can't use the other options with this.
///         !flatten: e_f: Example2,
///     );
/// }
/// ```
///
///
#[macro_export]
macro_rules! epee_object {
    // ------------------------------------------------------------------------ internal_try_right_then_left
    // All this does is return the second (right) arg if present otherwise the left is returned.
    (
        @internal_try_right_then_left
        $a:expr_2021, $b:expr_2021
    ) => {
        $b
    };

    (
        @internal_try_right_then_left
        $a:expr_2021,
    ) => {
        $a
    };

    // ------------------------------------------------------------------------ internal_field_name
    // Returns the alt_name if present otherwise stringifies the field ident.
    (
        @internal_field_name
        $field: tt, $alt_name: tt
    ) => {
        $alt_name
    };

    (
        @internal_field_name
        $field: ident,
    ) => {
        stringify!($field)
    };

    // ------------------------------------------------------------------------ internal_field_type
    // All this does is return the second (right) arg if present otherwise the left is returned.
    (
        @internal_field_type
        $ty:ty, $ty_as:ty
    ) => {
        $ty_as
    };
    (
        @internal_field_type
        $ty:ty,
    ) => {
        $ty
    };

    // ------------------------------------------------------------------------ Entry Point
    (
        $obj:ident,
        $($field: ident $(($alt_name: literal))?: $ty:ty $(as $ty_as:ty )? $(= $default:expr_2021)?  $(=> $read_fn:expr_2021, $write_fn:expr_2021, $should_write_fn:expr_2021)?, )*
        $(!flatten: $flat_field: ident: $flat_ty:ty ,)*

    ) => {
        cuprate_epee_encoding::macros::paste!(
            #[allow(non_snake_case)]
            mod [<__epee_builder_ $obj>] {
                use super::*;

                #[derive(Default)]
                pub struct [<__Builder $obj>] {
                    $($field: Option<cuprate_epee_encoding::epee_object!(@internal_field_type $ty, $($ty_as)?)>,)*
                    $($flat_field: <$flat_ty as cuprate_epee_encoding::EpeeObject>::Builder,)*
                }

                impl cuprate_epee_encoding::EpeeObjectBuilder<$obj> for [<__Builder $obj>] {
                    fn add_field<B: cuprate_epee_encoding::macros::bytes::Buf>(&mut self, name: &str, b: &mut B) -> cuprate_epee_encoding::error::Result<bool> {
                        match name {
                            $(cuprate_epee_encoding::epee_object!(@internal_field_name $field, $($alt_name)?) => {
                                if core::mem::replace(&mut self.$field, Some(
                                    cuprate_epee_encoding::epee_object!(@internal_try_right_then_left cuprate_epee_encoding::read_epee_value(b)?, $($read_fn(b)?)?)
                                )).is_some() {
                                    Err(cuprate_epee_encoding::error::Error::Value(format!("Duplicate field in data: {}", cuprate_epee_encoding::epee_object!(@internal_field_name$field, $($alt_name)?))))?;
                                }
                                Ok(true)
                            },)*
                            _ => {

                                $(if self.$flat_field.add_field(name, b)? {
                                    return Ok(true);
                                })*

                                Ok(false)
                            }
                        }
                    }

                    fn finish(self) -> cuprate_epee_encoding::error::Result<$obj> {
                        Ok(
                            $obj {
                                $(
                                  $field: {
                                      let epee_default_value = cuprate_epee_encoding::epee_object!(@internal_try_right_then_left cuprate_epee_encoding::EpeeValue::epee_default_value(), $({
                                            let _ = $should_write_fn;
                                            None
                                      })?);

                                      self.$field
                                            $(.or(Some($default)))?
                                            .or(epee_default_value)
                                            $(.map(<$ty_as>::into))?
                                              .ok_or(cuprate_epee_encoding::error::Error::Value(format!("Missing field in data: {}", cuprate_epee_encoding::epee_object!(@internal_field_name$field, $($alt_name)?))))?
                                  },
                                )*

                                $(
                                    $flat_field: self.$flat_field.finish()?,
                                )*

                            }
                        )
                    }
                }
            }

            impl cuprate_epee_encoding::EpeeObject for $obj {
                type Builder = [<__epee_builder_ $obj>]::[<__Builder $obj>];

                fn number_of_fields(&self) -> u64 {
                    let mut fields = 0;

                    $(
                    let field = cuprate_epee_encoding::epee_object!(@internal_try_right_then_left &self.$field, $(<&$ty_as>::from(&self.$field))? );

                      if $((field) != &$default &&)? cuprate_epee_encoding::epee_object!(@internal_try_right_then_left cuprate_epee_encoding::EpeeValue::should_write, $($should_write_fn)?)(field )
                      {
                          fields += 1;
                      }
                    )*

                    $(
                        fields += self.$flat_field.number_of_fields();
                    )*

                    fields
                }

                fn write_fields<B: cuprate_epee_encoding::macros::bytes::BufMut>(self, w: &mut B) -> cuprate_epee_encoding::error::Result<()> {
                    $(
                    let field = cuprate_epee_encoding::epee_object!(@internal_try_right_then_left self.$field, $(<$ty_as>::from(self.$field))? );

                      if $(field != $default &&)? cuprate_epee_encoding::epee_object!(@internal_try_right_then_left cuprate_epee_encoding::EpeeValue::should_write, $($should_write_fn)?)(&field )
                        {
                         cuprate_epee_encoding::epee_object!(@internal_try_right_then_left cuprate_epee_encoding::write_field, $($write_fn)?)((field), cuprate_epee_encoding::epee_object!(@internal_field_name$field, $($alt_name)?), w)?;
                      }
                    )*

                    $(
                        self.$flat_field.write_fields(w)?;
                    )*

                    Ok(())
                }
            }
        );
    };
}
