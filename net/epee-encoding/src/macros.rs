pub use bytes;
pub use paste::paste;

#[macro_export]
macro_rules! field_name {
    ($field: tt, $alt_name: tt) => {
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
macro_rules! try_right_then_left {
    ($a:expr, $b:expr) => {
        $b
    };
    ($a:expr,) => {
        $a
    };
}

#[macro_export]
macro_rules! epee_object {
    (
        $obj:ident,
        $($field: ident $(($alt_name: literal))?: $ty:ty $(as $ty_as:ty )? $(= $default:expr)?  $(=> $read_fn:expr, $write_fn:expr, $should_write_fn:expr)?, )+
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
                                if core::mem::replace(&mut self.$field, Some(
                                    epee_encoding::try_right_then_left!(epee_encoding::read_epee_value(b)?, $($read_fn(b)?)?)
                                )).is_some() {
                                    Err(epee_encoding::error::Error::Value(format!("Duplicate field in data: {}", epee_encoding::field_name!($field, $($alt_name)?))))?;
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
                                  $field: {
                                      let epee_default_value = epee_encoding::try_right_then_left!(epee_encoding::EpeeValue::epee_default_value(), $({
                                            let _ = $should_write_fn;
                                            None
                                      })?);

                                      self.$field
                                            $(.or(Some($default)))?
                                            .or(epee_default_value)
                                            $(.map(<$ty_as>::into))?
                                              .ok_or(epee_encoding::error::Error::Value(format!("Missing field in data: {}", epee_encoding::field_name!($field, $($alt_name)?))))?
                                  },
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
                    let field = epee_encoding::try_right_then_left!(&self.$field, $(<&$ty_as>::from(&self.$field))? );

                      if $((field) != &$default &&)? epee_encoding::try_right_then_left!(epee_encoding::EpeeValue::should_write, $($should_write_fn)?)(field )
                      {
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
                    let field = epee_encoding::try_right_then_left!(self.$field, $(<$ty_as>::from(self.$field))? );

                      if $(field != $default &&)? epee_encoding::try_right_then_left!(epee_encoding::EpeeValue::should_write, $($should_write_fn)?)(&field )
                        {
                         epee_encoding::try_right_then_left!(epee_encoding::write_field, $($write_fn)?)((field), epee_encoding::field_name!($field, $($alt_name)?), w)?;
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
