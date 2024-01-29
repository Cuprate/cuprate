pub use bytes;
pub use paste::paste;

#[macro_export]
macro_rules! field_name {
    ($field: ident, $alt_name: literal) => {
        $alt_name
    };
    ($field: ident,) => {
        stringify!($field)
    };
}

#[macro_export]
macro_rules! field_ty {
    ($ty:ty, $ty_as:ty) => {
        $ty_as
    };
    ($ty:ty,) => {
        $ty
    };
}

#[macro_export]
macro_rules! epee_object {
    (
        $obj:ident,
        $($field: ident $(($alt_name: literal))?: $ty:ty $(= $default:literal)? $(as $ty_as:ty)?, )+
        $(!flatten: $($flat_field: ident: $flat_ty:ty ,)+)?

    ) => {
        epee_encoding::macros::paste!(
            #[allow(non_snake_case)]
            mod [<__epee_builder_ $obj>] {
                use super::*;

                #[derive(Default)]
                pub struct [<__Builder $obj>] {
                    $($field: Option<epee_encoding::field_ty!($ty, $($ty_as)?)>,)+
                    $($($flat_field: <$flat_ty as epee_encoding::EpeeObject>::Builder,)+)?
                }

                impl epee_encoding::EpeeObjectBuilder<$obj> for [<__Builder $obj>] {
                    fn add_field<B: epee_encoding::macros::bytes::Buf>(&mut self, name: &str, b: &mut B) -> epee_encoding::error::Result<bool> {
                        match name {
                            $(epee_encoding::field_name!($field, $($alt_name)?) => {
                                if core::mem::replace(&mut self.$field, Some(epee_encoding::read_epee_value(b)?)).is_some() {
                                    Err(epee_encoding::error::Error::Value("Duplicate field in data"))?;
                                }
                                Ok(true)
                            },)+
                            _ => {
                                $(
                                    $( if self.$flat_field.add_field(name, b)? {
                                        return Ok(true);
                                    })+
                                )?

                                Ok(false)
                            }
                        }
                    }

                    fn finish(self) -> epee_encoding::error::Result<$obj> {
                        Ok(
                            $obj {
                                $(
                                  $field: self.$field
                                            $(.or(Some($default)))?
                                              .or(epee_encoding::EpeeValue::epee_default_value())
                                            $(.map(<$ty_as>::into))?
                                              .ok_or(epee_encoding::error::Error::Value("Missing field in data"))?,
                                )+

                                $(
                                  $(
                                    $flat_field: self.$flat_field.finish()?,
                                  )+
                                )?
                            }
                        )
                    }
                }
            }

            impl epee_encoding::EpeeObject for $obj {
                type Builder = [<__epee_builder_ $obj>]::[<__Builder $obj>];

                fn number_of_fields(&self) -> u64 {
                    let mut fields = 0;

                    $(
                      if $(&self.$field != &$default &&)? epee_encoding::EpeeValue::should_write($(<&$ty_as>::from)?( &self.$field) ) {
                          fields += 1;
                      }
                    )+

                    $(
                      $(
                       fields += self.$flat_field.number_of_fields();
                      )+
                    )?

                    fields
                }

                fn write_fields<B: epee_encoding::macros::bytes::BufMut>(self, w: &mut B) -> epee_encoding::error::Result<()> {
                    $(
                      if $(&self.$field != &$default &&)? epee_encoding::EpeeValue::should_write($(<&$ty_as>::from)?( &self.$field) ) {
                         epee_encoding::write_field($(<$ty_as>::from)?(self.$field), epee_encoding::field_name!($field, $($alt_name)?), w)?;
                      }
                    )+

                    $(
                      $(
                        self.$flat_field.write_fields(w)?;
                      )+
                    )?

                    Ok(())
                }
            }
        );
    };
}
