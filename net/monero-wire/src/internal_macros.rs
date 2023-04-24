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
        #[sealed::sealed]
        impl crate::messages::NetworkMessage for $req {
            type EncodingError = $req_enc_err;
            fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError> {
                $req_dec(buf)
            }
            fn encode(&self) -> Result<Vec<u8>, Self::EncodingError> {
                $req_enc(self)
            }
        }
        #[sealed::sealed]
        impl crate::messages::NetworkMessage for $res {
            type EncodingError = $res_enc_err;
            fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError> {
                $res_dec(buf)
            }
            fn encode(&self) -> Result<Vec<u8>, Self::EncodingError> {
                $res_enc(self)
            }
        }

        pub struct $name;

        #[sealed::sealed]
        impl crate::messages::AdminMessage for $name {
            const ID: u32 = $id;
            const NAME: &'static str = stringify!($name);

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
        #[sealed::sealed]
        impl crate::messages::NetworkMessage for $name {
            type EncodingError = $enc_err;
            fn decode(buf: &[u8]) -> Result<Self, Self::EncodingError> {
                $dec(buf)
            }
            fn encode(&self) -> Result<Vec<u8>, Self::EncodingError> {
                $enc(self)
            }
        }

        #[sealed::sealed]
        impl crate::messages::ProtocolMessage for $name {
            const ID: u32 = $id;
            const NAME: &'static str = stringify!($name);

            type Notification = Self;
        }
    };
}
