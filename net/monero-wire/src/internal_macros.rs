// Rust Levin Library
// Written in 2023 by
//   Cuprate Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//

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
            .ok_or_else(|| serde::de::Error::invalid_type($map.get_value_type_as_unexpected(), &$expected_ty))?
    };
}

macro_rules! get_internal_val {
    ($value:ident, $get_fn:ident, $expected_ty:expr) => {
        $value
            .$get_fn()
            .ok_or_else(|| serde::de::Error::invalid_type($value.get_value_type_as_unexpected(), &$expected_ty))?
    };
}

macro_rules! monero_decode_into_serde_err {
    ($ty:ty, $buf:ident) => {
        monero::consensus::deserialize::<$ty>($buf).map_err(serde::de::Error::custom)?
    };
}

macro_rules! message {
    (
        Admin,
        Name: $name:ident,
        ID: $id:expr,
        Request: $req:ident {
            EncodingError: $req_enc_err:path,
            Encode: $req_enc:path,
            Decode: $req_dec:path,
        },
        Response: $res:ident {
            EncodingError: $res_enc_err:path,
            Encode: $res_enc:path,
            Decode: $res_dec:path,
        },
    ) => {
        impl crate::messages::Message for $req {
            type EncodingError = $req_enc_err;
            fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError> {
                $req_dec(buf)
            }
            fn encode(&self) -> Result<Vec<u8>, Self::EncodingError> {
                $req_enc(self)
            }
        }

        impl crate::messages::Message for $res {
            type EncodingError = $res_enc_err;
            fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError> {
                $res_dec(buf)
            }
            fn encode(&self) -> Result<Vec<u8>, Self::EncodingError> {
                $res_enc(self)
            }
        }

        pub struct $name;

        impl crate::messages::AdminMessage for $name {
            const ID: u32 = $id;

            type Request = $req;
            type Response = $res;
        }
    };
    (
        Protocol,
        Name: $name:ident {
            EncodingError: $enc_err:path,
            Encode: $enc:path,
            Decode: $dec:path,
        },
        ID: $id:expr,
    ) => {
        impl crate::messages::Message for $name {
            type EncodingError = $enc_err;
            fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError> {
                $dec(buf)
            }
            fn encode(&self) -> Result<Vec<u8>, Self::EncodingError> {
                $enc(self)
            }
        }

        impl crate::messages::ProtocolMessage for $name {
            const ID: u32 = $id;

            type Notification = Self;
        }
    };
}
