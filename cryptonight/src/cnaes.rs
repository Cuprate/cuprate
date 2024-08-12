const AES_BLOCK_SIZE: usize = 16;

// 16 bytes just like all AES 128 and 256
const ROUND_KEY_SIZE: usize = 16;

// Cryptonight's hash uses the key size of AES256, but it only does 10 AES rounds
// like AES128.
const CN_AES_KEY_SIZE: usize = 32;

// AES-128 uses 11 round keys and AES-256 uses 15 round keys. Cryptonight's
// version of AES uses one less round key than AES-128 (which does 10 rounds
// like Cryptonight), because it doesn't mix the first round key into the state
// and uses the 0th key in the first round instead of the 1st.
const NUM_AES_ROUND_KEYS: usize = 10;

#[rustfmt::skip]
const AES_SBOX: [[u8; 16]; 16] = [
    [0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76],
    [0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0],
    [0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15],
    [0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75],
    [0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84],
    [0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf],
    [0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8],
    [0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2],
    [0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73],
    [0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb],
    [0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79],
    [0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08],
    [0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a],
    [0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e],
    [0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf],
    [0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16]
];

// Cryptonight extends the AES S-Box to 4096 bytes. The C++ code uses a series
// of nested preprocessor macros to do the extension if you want to see how the
// values below are derived, but unwinding nested macros will hurt your head.
#[rustfmt::skip]
const CRYPTONIGHT_SBOX: [u8; 4096] = [
    0xc6, 0x63, 0x63, 0xa5, 0xf8, 0x7c, 0x7c, 0x84, 0xee, 0x77, 0x77, 0x99, 0xf6, 0x7b, 0x7b, 0x8d,
    0xff, 0xf2, 0xf2, 0x0d, 0xd6, 0x6b, 0x6b, 0xbd, 0xde, 0x6f, 0x6f, 0xb1, 0x91, 0xc5, 0xc5, 0x54,
    0x60, 0x30, 0x30, 0x50, 0x02, 0x01, 0x01, 0x03, 0xce, 0x67, 0x67, 0xa9, 0x56, 0x2b, 0x2b, 0x7d,
    0xe7, 0xfe, 0xfe, 0x19, 0xb5, 0xd7, 0xd7, 0x62, 0x4d, 0xab, 0xab, 0xe6, 0xec, 0x76, 0x76, 0x9a,
    0x8f, 0xca, 0xca, 0x45, 0x1f, 0x82, 0x82, 0x9d, 0x89, 0xc9, 0xc9, 0x40, 0xfa, 0x7d, 0x7d, 0x87,
    0xef, 0xfa, 0xfa, 0x15, 0xb2, 0x59, 0x59, 0xeb, 0x8e, 0x47, 0x47, 0xc9, 0xfb, 0xf0, 0xf0, 0x0b,
    0x41, 0xad, 0xad, 0xec, 0xb3, 0xd4, 0xd4, 0x67, 0x5f, 0xa2, 0xa2, 0xfd, 0x45, 0xaf, 0xaf, 0xea,
    0x23, 0x9c, 0x9c, 0xbf, 0x53, 0xa4, 0xa4, 0xf7, 0xe4, 0x72, 0x72, 0x96, 0x9b, 0xc0, 0xc0, 0x5b,
    0x75, 0xb7, 0xb7, 0xc2, 0xe1, 0xfd, 0xfd, 0x1c, 0x3d, 0x93, 0x93, 0xae, 0x4c, 0x26, 0x26, 0x6a,
    0x6c, 0x36, 0x36, 0x5a, 0x7e, 0x3f, 0x3f, 0x41, 0xf5, 0xf7, 0xf7, 0x02, 0x83, 0xcc, 0xcc, 0x4f,
    0x68, 0x34, 0x34, 0x5c, 0x51, 0xa5, 0xa5, 0xf4, 0xd1, 0xe5, 0xe5, 0x34, 0xf9, 0xf1, 0xf1, 0x08,
    0xe2, 0x71, 0x71, 0x93, 0xab, 0xd8, 0xd8, 0x73, 0x62, 0x31, 0x31, 0x53, 0x2a, 0x15, 0x15, 0x3f,
    0x08, 0x04, 0x04, 0x0c, 0x95, 0xc7, 0xc7, 0x52, 0x46, 0x23, 0x23, 0x65, 0x9d, 0xc3, 0xc3, 0x5e,
    0x30, 0x18, 0x18, 0x28, 0x37, 0x96, 0x96, 0xa1, 0x0a, 0x05, 0x05, 0x0f, 0x2f, 0x9a, 0x9a, 0xb5,
    0x0e, 0x07, 0x07, 0x09, 0x24, 0x12, 0x12, 0x36, 0x1b, 0x80, 0x80, 0x9b, 0xdf, 0xe2, 0xe2, 0x3d,
    0xcd, 0xeb, 0xeb, 0x26, 0x4e, 0x27, 0x27, 0x69, 0x7f, 0xb2, 0xb2, 0xcd, 0xea, 0x75, 0x75, 0x9f,
    0x12, 0x09, 0x09, 0x1b, 0x1d, 0x83, 0x83, 0x9e, 0x58, 0x2c, 0x2c, 0x74, 0x34, 0x1a, 0x1a, 0x2e,
    0x36, 0x1b, 0x1b, 0x2d, 0xdc, 0x6e, 0x6e, 0xb2, 0xb4, 0x5a, 0x5a, 0xee, 0x5b, 0xa0, 0xa0, 0xfb,
    0xa4, 0x52, 0x52, 0xf6, 0x76, 0x3b, 0x3b, 0x4d, 0xb7, 0xd6, 0xd6, 0x61, 0x7d, 0xb3, 0xb3, 0xce,
    0x52, 0x29, 0x29, 0x7b, 0xdd, 0xe3, 0xe3, 0x3e, 0x5e, 0x2f, 0x2f, 0x71, 0x13, 0x84, 0x84, 0x97,
    0xa6, 0x53, 0x53, 0xf5, 0xb9, 0xd1, 0xd1, 0x68, 0x00, 0x00, 0x00, 0x00, 0xc1, 0xed, 0xed, 0x2c,
    0x40, 0x20, 0x20, 0x60, 0xe3, 0xfc, 0xfc, 0x1f, 0x79, 0xb1, 0xb1, 0xc8, 0xb6, 0x5b, 0x5b, 0xed,
    0xd4, 0x6a, 0x6a, 0xbe, 0x8d, 0xcb, 0xcb, 0x46, 0x67, 0xbe, 0xbe, 0xd9, 0x72, 0x39, 0x39, 0x4b,
    0x94, 0x4a, 0x4a, 0xde, 0x98, 0x4c, 0x4c, 0xd4, 0xb0, 0x58, 0x58, 0xe8, 0x85, 0xcf, 0xcf, 0x4a,
    0xbb, 0xd0, 0xd0, 0x6b, 0xc5, 0xef, 0xef, 0x2a, 0x4f, 0xaa, 0xaa, 0xe5, 0xed, 0xfb, 0xfb, 0x16,
    0x86, 0x43, 0x43, 0xc5, 0x9a, 0x4d, 0x4d, 0xd7, 0x66, 0x33, 0x33, 0x55, 0x11, 0x85, 0x85, 0x94,
    0x8a, 0x45, 0x45, 0xcf, 0xe9, 0xf9, 0xf9, 0x10, 0x04, 0x02, 0x02, 0x06, 0xfe, 0x7f, 0x7f, 0x81,
    0xa0, 0x50, 0x50, 0xf0, 0x78, 0x3c, 0x3c, 0x44, 0x25, 0x9f, 0x9f, 0xba, 0x4b, 0xa8, 0xa8, 0xe3,
    0xa2, 0x51, 0x51, 0xf3, 0x5d, 0xa3, 0xa3, 0xfe, 0x80, 0x40, 0x40, 0xc0, 0x05, 0x8f, 0x8f, 0x8a,
    0x3f, 0x92, 0x92, 0xad, 0x21, 0x9d, 0x9d, 0xbc, 0x70, 0x38, 0x38, 0x48, 0xf1, 0xf5, 0xf5, 0x04,
    0x63, 0xbc, 0xbc, 0xdf, 0x77, 0xb6, 0xb6, 0xc1, 0xaf, 0xda, 0xda, 0x75, 0x42, 0x21, 0x21, 0x63,
    0x20, 0x10, 0x10, 0x30, 0xe5, 0xff, 0xff, 0x1a, 0xfd, 0xf3, 0xf3, 0x0e, 0xbf, 0xd2, 0xd2, 0x6d,
    0x81, 0xcd, 0xcd, 0x4c, 0x18, 0x0c, 0x0c, 0x14, 0x26, 0x13, 0x13, 0x35, 0xc3, 0xec, 0xec, 0x2f,
    0xbe, 0x5f, 0x5f, 0xe1, 0x35, 0x97, 0x97, 0xa2, 0x88, 0x44, 0x44, 0xcc, 0x2e, 0x17, 0x17, 0x39,
    0x93, 0xc4, 0xc4, 0x57, 0x55, 0xa7, 0xa7, 0xf2, 0xfc, 0x7e, 0x7e, 0x82, 0x7a, 0x3d, 0x3d, 0x47,
    0xc8, 0x64, 0x64, 0xac, 0xba, 0x5d, 0x5d, 0xe7, 0x32, 0x19, 0x19, 0x2b, 0xe6, 0x73, 0x73, 0x95,
    0xc0, 0x60, 0x60, 0xa0, 0x19, 0x81, 0x81, 0x98, 0x9e, 0x4f, 0x4f, 0xd1, 0xa3, 0xdc, 0xdc, 0x7f,
    0x44, 0x22, 0x22, 0x66, 0x54, 0x2a, 0x2a, 0x7e, 0x3b, 0x90, 0x90, 0xab, 0x0b, 0x88, 0x88, 0x83,
    0x8c, 0x46, 0x46, 0xca, 0xc7, 0xee, 0xee, 0x29, 0x6b, 0xb8, 0xb8, 0xd3, 0x28, 0x14, 0x14, 0x3c,
    0xa7, 0xde, 0xde, 0x79, 0xbc, 0x5e, 0x5e, 0xe2, 0x16, 0x0b, 0x0b, 0x1d, 0xad, 0xdb, 0xdb, 0x76,
    0xdb, 0xe0, 0xe0, 0x3b, 0x64, 0x32, 0x32, 0x56, 0x74, 0x3a, 0x3a, 0x4e, 0x14, 0x0a, 0x0a, 0x1e,
    0x92, 0x49, 0x49, 0xdb, 0x0c, 0x06, 0x06, 0x0a, 0x48, 0x24, 0x24, 0x6c, 0xb8, 0x5c, 0x5c, 0xe4,
    0x9f, 0xc2, 0xc2, 0x5d, 0xbd, 0xd3, 0xd3, 0x6e, 0x43, 0xac, 0xac, 0xef, 0xc4, 0x62, 0x62, 0xa6,
    0x39, 0x91, 0x91, 0xa8, 0x31, 0x95, 0x95, 0xa4, 0xd3, 0xe4, 0xe4, 0x37, 0xf2, 0x79, 0x79, 0x8b,
    0xd5, 0xe7, 0xe7, 0x32, 0x8b, 0xc8, 0xc8, 0x43, 0x6e, 0x37, 0x37, 0x59, 0xda, 0x6d, 0x6d, 0xb7,
    0x01, 0x8d, 0x8d, 0x8c, 0xb1, 0xd5, 0xd5, 0x64, 0x9c, 0x4e, 0x4e, 0xd2, 0x49, 0xa9, 0xa9, 0xe0,
    0xd8, 0x6c, 0x6c, 0xb4, 0xac, 0x56, 0x56, 0xfa, 0xf3, 0xf4, 0xf4, 0x07, 0xcf, 0xea, 0xea, 0x25,
    0xca, 0x65, 0x65, 0xaf, 0xf4, 0x7a, 0x7a, 0x8e, 0x47, 0xae, 0xae, 0xe9, 0x10, 0x08, 0x08, 0x18,
    0x6f, 0xba, 0xba, 0xd5, 0xf0, 0x78, 0x78, 0x88, 0x4a, 0x25, 0x25, 0x6f, 0x5c, 0x2e, 0x2e, 0x72,
    0x38, 0x1c, 0x1c, 0x24, 0x57, 0xa6, 0xa6, 0xf1, 0x73, 0xb4, 0xb4, 0xc7, 0x97, 0xc6, 0xc6, 0x51,
    0xcb, 0xe8, 0xe8, 0x23, 0xa1, 0xdd, 0xdd, 0x7c, 0xe8, 0x74, 0x74, 0x9c, 0x3e, 0x1f, 0x1f, 0x21,
    0x96, 0x4b, 0x4b, 0xdd, 0x61, 0xbd, 0xbd, 0xdc, 0x0d, 0x8b, 0x8b, 0x86, 0x0f, 0x8a, 0x8a, 0x85,
    0xe0, 0x70, 0x70, 0x90, 0x7c, 0x3e, 0x3e, 0x42, 0x71, 0xb5, 0xb5, 0xc4, 0xcc, 0x66, 0x66, 0xaa,
    0x90, 0x48, 0x48, 0xd8, 0x06, 0x03, 0x03, 0x05, 0xf7, 0xf6, 0xf6, 0x01, 0x1c, 0x0e, 0x0e, 0x12,
    0xc2, 0x61, 0x61, 0xa3, 0x6a, 0x35, 0x35, 0x5f, 0xae, 0x57, 0x57, 0xf9, 0x69, 0xb9, 0xb9, 0xd0,
    0x17, 0x86, 0x86, 0x91, 0x99, 0xc1, 0xc1, 0x58, 0x3a, 0x1d, 0x1d, 0x27, 0x27, 0x9e, 0x9e, 0xb9,
    0xd9, 0xe1, 0xe1, 0x38, 0xeb, 0xf8, 0xf8, 0x13, 0x2b, 0x98, 0x98, 0xb3, 0x22, 0x11, 0x11, 0x33,
    0xd2, 0x69, 0x69, 0xbb, 0xa9, 0xd9, 0xd9, 0x70, 0x07, 0x8e, 0x8e, 0x89, 0x33, 0x94, 0x94, 0xa7,
    0x2d, 0x9b, 0x9b, 0xb6, 0x3c, 0x1e, 0x1e, 0x22, 0x15, 0x87, 0x87, 0x92, 0xc9, 0xe9, 0xe9, 0x20,
    0x87, 0xce, 0xce, 0x49, 0xaa, 0x55, 0x55, 0xff, 0x50, 0x28, 0x28, 0x78, 0xa5, 0xdf, 0xdf, 0x7a,
    0x03, 0x8c, 0x8c, 0x8f, 0x59, 0xa1, 0xa1, 0xf8, 0x09, 0x89, 0x89, 0x80, 0x1a, 0x0d, 0x0d, 0x17,
    0x65, 0xbf, 0xbf, 0xda, 0xd7, 0xe6, 0xe6, 0x31, 0x84, 0x42, 0x42, 0xc6, 0xd0, 0x68, 0x68, 0xb8,
    0x82, 0x41, 0x41, 0xc3, 0x29, 0x99, 0x99, 0xb0, 0x5a, 0x2d, 0x2d, 0x77, 0x1e, 0x0f, 0x0f, 0x11,
    0x7b, 0xb0, 0xb0, 0xcb, 0xa8, 0x54, 0x54, 0xfc, 0x6d, 0xbb, 0xbb, 0xd6, 0x2c, 0x16, 0x16, 0x3a,
    0xa5, 0xc6, 0x63, 0x63, 0x84, 0xf8, 0x7c, 0x7c, 0x99, 0xee, 0x77, 0x77, 0x8d, 0xf6, 0x7b, 0x7b,
    0x0d, 0xff, 0xf2, 0xf2, 0xbd, 0xd6, 0x6b, 0x6b, 0xb1, 0xde, 0x6f, 0x6f, 0x54, 0x91, 0xc5, 0xc5,
    0x50, 0x60, 0x30, 0x30, 0x03, 0x02, 0x01, 0x01, 0xa9, 0xce, 0x67, 0x67, 0x7d, 0x56, 0x2b, 0x2b,
    0x19, 0xe7, 0xfe, 0xfe, 0x62, 0xb5, 0xd7, 0xd7, 0xe6, 0x4d, 0xab, 0xab, 0x9a, 0xec, 0x76, 0x76,
    0x45, 0x8f, 0xca, 0xca, 0x9d, 0x1f, 0x82, 0x82, 0x40, 0x89, 0xc9, 0xc9, 0x87, 0xfa, 0x7d, 0x7d,
    0x15, 0xef, 0xfa, 0xfa, 0xeb, 0xb2, 0x59, 0x59, 0xc9, 0x8e, 0x47, 0x47, 0x0b, 0xfb, 0xf0, 0xf0,
    0xec, 0x41, 0xad, 0xad, 0x67, 0xb3, 0xd4, 0xd4, 0xfd, 0x5f, 0xa2, 0xa2, 0xea, 0x45, 0xaf, 0xaf,
    0xbf, 0x23, 0x9c, 0x9c, 0xf7, 0x53, 0xa4, 0xa4, 0x96, 0xe4, 0x72, 0x72, 0x5b, 0x9b, 0xc0, 0xc0,
    0xc2, 0x75, 0xb7, 0xb7, 0x1c, 0xe1, 0xfd, 0xfd, 0xae, 0x3d, 0x93, 0x93, 0x6a, 0x4c, 0x26, 0x26,
    0x5a, 0x6c, 0x36, 0x36, 0x41, 0x7e, 0x3f, 0x3f, 0x02, 0xf5, 0xf7, 0xf7, 0x4f, 0x83, 0xcc, 0xcc,
    0x5c, 0x68, 0x34, 0x34, 0xf4, 0x51, 0xa5, 0xa5, 0x34, 0xd1, 0xe5, 0xe5, 0x08, 0xf9, 0xf1, 0xf1,
    0x93, 0xe2, 0x71, 0x71, 0x73, 0xab, 0xd8, 0xd8, 0x53, 0x62, 0x31, 0x31, 0x3f, 0x2a, 0x15, 0x15,
    0x0c, 0x08, 0x04, 0x04, 0x52, 0x95, 0xc7, 0xc7, 0x65, 0x46, 0x23, 0x23, 0x5e, 0x9d, 0xc3, 0xc3,
    0x28, 0x30, 0x18, 0x18, 0xa1, 0x37, 0x96, 0x96, 0x0f, 0x0a, 0x05, 0x05, 0xb5, 0x2f, 0x9a, 0x9a,
    0x09, 0x0e, 0x07, 0x07, 0x36, 0x24, 0x12, 0x12, 0x9b, 0x1b, 0x80, 0x80, 0x3d, 0xdf, 0xe2, 0xe2,
    0x26, 0xcd, 0xeb, 0xeb, 0x69, 0x4e, 0x27, 0x27, 0xcd, 0x7f, 0xb2, 0xb2, 0x9f, 0xea, 0x75, 0x75,
    0x1b, 0x12, 0x09, 0x09, 0x9e, 0x1d, 0x83, 0x83, 0x74, 0x58, 0x2c, 0x2c, 0x2e, 0x34, 0x1a, 0x1a,
    0x2d, 0x36, 0x1b, 0x1b, 0xb2, 0xdc, 0x6e, 0x6e, 0xee, 0xb4, 0x5a, 0x5a, 0xfb, 0x5b, 0xa0, 0xa0,
    0xf6, 0xa4, 0x52, 0x52, 0x4d, 0x76, 0x3b, 0x3b, 0x61, 0xb7, 0xd6, 0xd6, 0xce, 0x7d, 0xb3, 0xb3,
    0x7b, 0x52, 0x29, 0x29, 0x3e, 0xdd, 0xe3, 0xe3, 0x71, 0x5e, 0x2f, 0x2f, 0x97, 0x13, 0x84, 0x84,
    0xf5, 0xa6, 0x53, 0x53, 0x68, 0xb9, 0xd1, 0xd1, 0x00, 0x00, 0x00, 0x00, 0x2c, 0xc1, 0xed, 0xed,
    0x60, 0x40, 0x20, 0x20, 0x1f, 0xe3, 0xfc, 0xfc, 0xc8, 0x79, 0xb1, 0xb1, 0xed, 0xb6, 0x5b, 0x5b,
    0xbe, 0xd4, 0x6a, 0x6a, 0x46, 0x8d, 0xcb, 0xcb, 0xd9, 0x67, 0xbe, 0xbe, 0x4b, 0x72, 0x39, 0x39,
    0xde, 0x94, 0x4a, 0x4a, 0xd4, 0x98, 0x4c, 0x4c, 0xe8, 0xb0, 0x58, 0x58, 0x4a, 0x85, 0xcf, 0xcf,
    0x6b, 0xbb, 0xd0, 0xd0, 0x2a, 0xc5, 0xef, 0xef, 0xe5, 0x4f, 0xaa, 0xaa, 0x16, 0xed, 0xfb, 0xfb,
    0xc5, 0x86, 0x43, 0x43, 0xd7, 0x9a, 0x4d, 0x4d, 0x55, 0x66, 0x33, 0x33, 0x94, 0x11, 0x85, 0x85,
    0xcf, 0x8a, 0x45, 0x45, 0x10, 0xe9, 0xf9, 0xf9, 0x06, 0x04, 0x02, 0x02, 0x81, 0xfe, 0x7f, 0x7f,
    0xf0, 0xa0, 0x50, 0x50, 0x44, 0x78, 0x3c, 0x3c, 0xba, 0x25, 0x9f, 0x9f, 0xe3, 0x4b, 0xa8, 0xa8,
    0xf3, 0xa2, 0x51, 0x51, 0xfe, 0x5d, 0xa3, 0xa3, 0xc0, 0x80, 0x40, 0x40, 0x8a, 0x05, 0x8f, 0x8f,
    0xad, 0x3f, 0x92, 0x92, 0xbc, 0x21, 0x9d, 0x9d, 0x48, 0x70, 0x38, 0x38, 0x04, 0xf1, 0xf5, 0xf5,
    0xdf, 0x63, 0xbc, 0xbc, 0xc1, 0x77, 0xb6, 0xb6, 0x75, 0xaf, 0xda, 0xda, 0x63, 0x42, 0x21, 0x21,
    0x30, 0x20, 0x10, 0x10, 0x1a, 0xe5, 0xff, 0xff, 0x0e, 0xfd, 0xf3, 0xf3, 0x6d, 0xbf, 0xd2, 0xd2,
    0x4c, 0x81, 0xcd, 0xcd, 0x14, 0x18, 0x0c, 0x0c, 0x35, 0x26, 0x13, 0x13, 0x2f, 0xc3, 0xec, 0xec,
    0xe1, 0xbe, 0x5f, 0x5f, 0xa2, 0x35, 0x97, 0x97, 0xcc, 0x88, 0x44, 0x44, 0x39, 0x2e, 0x17, 0x17,
    0x57, 0x93, 0xc4, 0xc4, 0xf2, 0x55, 0xa7, 0xa7, 0x82, 0xfc, 0x7e, 0x7e, 0x47, 0x7a, 0x3d, 0x3d,
    0xac, 0xc8, 0x64, 0x64, 0xe7, 0xba, 0x5d, 0x5d, 0x2b, 0x32, 0x19, 0x19, 0x95, 0xe6, 0x73, 0x73,
    0xa0, 0xc0, 0x60, 0x60, 0x98, 0x19, 0x81, 0x81, 0xd1, 0x9e, 0x4f, 0x4f, 0x7f, 0xa3, 0xdc, 0xdc,
    0x66, 0x44, 0x22, 0x22, 0x7e, 0x54, 0x2a, 0x2a, 0xab, 0x3b, 0x90, 0x90, 0x83, 0x0b, 0x88, 0x88,
    0xca, 0x8c, 0x46, 0x46, 0x29, 0xc7, 0xee, 0xee, 0xd3, 0x6b, 0xb8, 0xb8, 0x3c, 0x28, 0x14, 0x14,
    0x79, 0xa7, 0xde, 0xde, 0xe2, 0xbc, 0x5e, 0x5e, 0x1d, 0x16, 0x0b, 0x0b, 0x76, 0xad, 0xdb, 0xdb,
    0x3b, 0xdb, 0xe0, 0xe0, 0x56, 0x64, 0x32, 0x32, 0x4e, 0x74, 0x3a, 0x3a, 0x1e, 0x14, 0x0a, 0x0a,
    0xdb, 0x92, 0x49, 0x49, 0x0a, 0x0c, 0x06, 0x06, 0x6c, 0x48, 0x24, 0x24, 0xe4, 0xb8, 0x5c, 0x5c,
    0x5d, 0x9f, 0xc2, 0xc2, 0x6e, 0xbd, 0xd3, 0xd3, 0xef, 0x43, 0xac, 0xac, 0xa6, 0xc4, 0x62, 0x62,
    0xa8, 0x39, 0x91, 0x91, 0xa4, 0x31, 0x95, 0x95, 0x37, 0xd3, 0xe4, 0xe4, 0x8b, 0xf2, 0x79, 0x79,
    0x32, 0xd5, 0xe7, 0xe7, 0x43, 0x8b, 0xc8, 0xc8, 0x59, 0x6e, 0x37, 0x37, 0xb7, 0xda, 0x6d, 0x6d,
    0x8c, 0x01, 0x8d, 0x8d, 0x64, 0xb1, 0xd5, 0xd5, 0xd2, 0x9c, 0x4e, 0x4e, 0xe0, 0x49, 0xa9, 0xa9,
    0xb4, 0xd8, 0x6c, 0x6c, 0xfa, 0xac, 0x56, 0x56, 0x07, 0xf3, 0xf4, 0xf4, 0x25, 0xcf, 0xea, 0xea,
    0xaf, 0xca, 0x65, 0x65, 0x8e, 0xf4, 0x7a, 0x7a, 0xe9, 0x47, 0xae, 0xae, 0x18, 0x10, 0x08, 0x08,
    0xd5, 0x6f, 0xba, 0xba, 0x88, 0xf0, 0x78, 0x78, 0x6f, 0x4a, 0x25, 0x25, 0x72, 0x5c, 0x2e, 0x2e,
    0x24, 0x38, 0x1c, 0x1c, 0xf1, 0x57, 0xa6, 0xa6, 0xc7, 0x73, 0xb4, 0xb4, 0x51, 0x97, 0xc6, 0xc6,
    0x23, 0xcb, 0xe8, 0xe8, 0x7c, 0xa1, 0xdd, 0xdd, 0x9c, 0xe8, 0x74, 0x74, 0x21, 0x3e, 0x1f, 0x1f,
    0xdd, 0x96, 0x4b, 0x4b, 0xdc, 0x61, 0xbd, 0xbd, 0x86, 0x0d, 0x8b, 0x8b, 0x85, 0x0f, 0x8a, 0x8a,
    0x90, 0xe0, 0x70, 0x70, 0x42, 0x7c, 0x3e, 0x3e, 0xc4, 0x71, 0xb5, 0xb5, 0xaa, 0xcc, 0x66, 0x66,
    0xd8, 0x90, 0x48, 0x48, 0x05, 0x06, 0x03, 0x03, 0x01, 0xf7, 0xf6, 0xf6, 0x12, 0x1c, 0x0e, 0x0e,
    0xa3, 0xc2, 0x61, 0x61, 0x5f, 0x6a, 0x35, 0x35, 0xf9, 0xae, 0x57, 0x57, 0xd0, 0x69, 0xb9, 0xb9,
    0x91, 0x17, 0x86, 0x86, 0x58, 0x99, 0xc1, 0xc1, 0x27, 0x3a, 0x1d, 0x1d, 0xb9, 0x27, 0x9e, 0x9e,
    0x38, 0xd9, 0xe1, 0xe1, 0x13, 0xeb, 0xf8, 0xf8, 0xb3, 0x2b, 0x98, 0x98, 0x33, 0x22, 0x11, 0x11,
    0xbb, 0xd2, 0x69, 0x69, 0x70, 0xa9, 0xd9, 0xd9, 0x89, 0x07, 0x8e, 0x8e, 0xa7, 0x33, 0x94, 0x94,
    0xb6, 0x2d, 0x9b, 0x9b, 0x22, 0x3c, 0x1e, 0x1e, 0x92, 0x15, 0x87, 0x87, 0x20, 0xc9, 0xe9, 0xe9,
    0x49, 0x87, 0xce, 0xce, 0xff, 0xaa, 0x55, 0x55, 0x78, 0x50, 0x28, 0x28, 0x7a, 0xa5, 0xdf, 0xdf,
    0x8f, 0x03, 0x8c, 0x8c, 0xf8, 0x59, 0xa1, 0xa1, 0x80, 0x09, 0x89, 0x89, 0x17, 0x1a, 0x0d, 0x0d,
    0xda, 0x65, 0xbf, 0xbf, 0x31, 0xd7, 0xe6, 0xe6, 0xc6, 0x84, 0x42, 0x42, 0xb8, 0xd0, 0x68, 0x68,
    0xc3, 0x82, 0x41, 0x41, 0xb0, 0x29, 0x99, 0x99, 0x77, 0x5a, 0x2d, 0x2d, 0x11, 0x1e, 0x0f, 0x0f,
    0xcb, 0x7b, 0xb0, 0xb0, 0xfc, 0xa8, 0x54, 0x54, 0xd6, 0x6d, 0xbb, 0xbb, 0x3a, 0x2c, 0x16, 0x16,
    0x63, 0xa5, 0xc6, 0x63, 0x7c, 0x84, 0xf8, 0x7c, 0x77, 0x99, 0xee, 0x77, 0x7b, 0x8d, 0xf6, 0x7b,
    0xf2, 0x0d, 0xff, 0xf2, 0x6b, 0xbd, 0xd6, 0x6b, 0x6f, 0xb1, 0xde, 0x6f, 0xc5, 0x54, 0x91, 0xc5,
    0x30, 0x50, 0x60, 0x30, 0x01, 0x03, 0x02, 0x01, 0x67, 0xa9, 0xce, 0x67, 0x2b, 0x7d, 0x56, 0x2b,
    0xfe, 0x19, 0xe7, 0xfe, 0xd7, 0x62, 0xb5, 0xd7, 0xab, 0xe6, 0x4d, 0xab, 0x76, 0x9a, 0xec, 0x76,
    0xca, 0x45, 0x8f, 0xca, 0x82, 0x9d, 0x1f, 0x82, 0xc9, 0x40, 0x89, 0xc9, 0x7d, 0x87, 0xfa, 0x7d,
    0xfa, 0x15, 0xef, 0xfa, 0x59, 0xeb, 0xb2, 0x59, 0x47, 0xc9, 0x8e, 0x47, 0xf0, 0x0b, 0xfb, 0xf0,
    0xad, 0xec, 0x41, 0xad, 0xd4, 0x67, 0xb3, 0xd4, 0xa2, 0xfd, 0x5f, 0xa2, 0xaf, 0xea, 0x45, 0xaf,
    0x9c, 0xbf, 0x23, 0x9c, 0xa4, 0xf7, 0x53, 0xa4, 0x72, 0x96, 0xe4, 0x72, 0xc0, 0x5b, 0x9b, 0xc0,
    0xb7, 0xc2, 0x75, 0xb7, 0xfd, 0x1c, 0xe1, 0xfd, 0x93, 0xae, 0x3d, 0x93, 0x26, 0x6a, 0x4c, 0x26,
    0x36, 0x5a, 0x6c, 0x36, 0x3f, 0x41, 0x7e, 0x3f, 0xf7, 0x02, 0xf5, 0xf7, 0xcc, 0x4f, 0x83, 0xcc,
    0x34, 0x5c, 0x68, 0x34, 0xa5, 0xf4, 0x51, 0xa5, 0xe5, 0x34, 0xd1, 0xe5, 0xf1, 0x08, 0xf9, 0xf1,
    0x71, 0x93, 0xe2, 0x71, 0xd8, 0x73, 0xab, 0xd8, 0x31, 0x53, 0x62, 0x31, 0x15, 0x3f, 0x2a, 0x15,
    0x04, 0x0c, 0x08, 0x04, 0xc7, 0x52, 0x95, 0xc7, 0x23, 0x65, 0x46, 0x23, 0xc3, 0x5e, 0x9d, 0xc3,
    0x18, 0x28, 0x30, 0x18, 0x96, 0xa1, 0x37, 0x96, 0x05, 0x0f, 0x0a, 0x05, 0x9a, 0xb5, 0x2f, 0x9a,
    0x07, 0x09, 0x0e, 0x07, 0x12, 0x36, 0x24, 0x12, 0x80, 0x9b, 0x1b, 0x80, 0xe2, 0x3d, 0xdf, 0xe2,
    0xeb, 0x26, 0xcd, 0xeb, 0x27, 0x69, 0x4e, 0x27, 0xb2, 0xcd, 0x7f, 0xb2, 0x75, 0x9f, 0xea, 0x75,
    0x09, 0x1b, 0x12, 0x09, 0x83, 0x9e, 0x1d, 0x83, 0x2c, 0x74, 0x58, 0x2c, 0x1a, 0x2e, 0x34, 0x1a,
    0x1b, 0x2d, 0x36, 0x1b, 0x6e, 0xb2, 0xdc, 0x6e, 0x5a, 0xee, 0xb4, 0x5a, 0xa0, 0xfb, 0x5b, 0xa0,
    0x52, 0xf6, 0xa4, 0x52, 0x3b, 0x4d, 0x76, 0x3b, 0xd6, 0x61, 0xb7, 0xd6, 0xb3, 0xce, 0x7d, 0xb3,
    0x29, 0x7b, 0x52, 0x29, 0xe3, 0x3e, 0xdd, 0xe3, 0x2f, 0x71, 0x5e, 0x2f, 0x84, 0x97, 0x13, 0x84,
    0x53, 0xf5, 0xa6, 0x53, 0xd1, 0x68, 0xb9, 0xd1, 0x00, 0x00, 0x00, 0x00, 0xed, 0x2c, 0xc1, 0xed,
    0x20, 0x60, 0x40, 0x20, 0xfc, 0x1f, 0xe3, 0xfc, 0xb1, 0xc8, 0x79, 0xb1, 0x5b, 0xed, 0xb6, 0x5b,
    0x6a, 0xbe, 0xd4, 0x6a, 0xcb, 0x46, 0x8d, 0xcb, 0xbe, 0xd9, 0x67, 0xbe, 0x39, 0x4b, 0x72, 0x39,
    0x4a, 0xde, 0x94, 0x4a, 0x4c, 0xd4, 0x98, 0x4c, 0x58, 0xe8, 0xb0, 0x58, 0xcf, 0x4a, 0x85, 0xcf,
    0xd0, 0x6b, 0xbb, 0xd0, 0xef, 0x2a, 0xc5, 0xef, 0xaa, 0xe5, 0x4f, 0xaa, 0xfb, 0x16, 0xed, 0xfb,
    0x43, 0xc5, 0x86, 0x43, 0x4d, 0xd7, 0x9a, 0x4d, 0x33, 0x55, 0x66, 0x33, 0x85, 0x94, 0x11, 0x85,
    0x45, 0xcf, 0x8a, 0x45, 0xf9, 0x10, 0xe9, 0xf9, 0x02, 0x06, 0x04, 0x02, 0x7f, 0x81, 0xfe, 0x7f,
    0x50, 0xf0, 0xa0, 0x50, 0x3c, 0x44, 0x78, 0x3c, 0x9f, 0xba, 0x25, 0x9f, 0xa8, 0xe3, 0x4b, 0xa8,
    0x51, 0xf3, 0xa2, 0x51, 0xa3, 0xfe, 0x5d, 0xa3, 0x40, 0xc0, 0x80, 0x40, 0x8f, 0x8a, 0x05, 0x8f,
    0x92, 0xad, 0x3f, 0x92, 0x9d, 0xbc, 0x21, 0x9d, 0x38, 0x48, 0x70, 0x38, 0xf5, 0x04, 0xf1, 0xf5,
    0xbc, 0xdf, 0x63, 0xbc, 0xb6, 0xc1, 0x77, 0xb6, 0xda, 0x75, 0xaf, 0xda, 0x21, 0x63, 0x42, 0x21,
    0x10, 0x30, 0x20, 0x10, 0xff, 0x1a, 0xe5, 0xff, 0xf3, 0x0e, 0xfd, 0xf3, 0xd2, 0x6d, 0xbf, 0xd2,
    0xcd, 0x4c, 0x81, 0xcd, 0x0c, 0x14, 0x18, 0x0c, 0x13, 0x35, 0x26, 0x13, 0xec, 0x2f, 0xc3, 0xec,
    0x5f, 0xe1, 0xbe, 0x5f, 0x97, 0xa2, 0x35, 0x97, 0x44, 0xcc, 0x88, 0x44, 0x17, 0x39, 0x2e, 0x17,
    0xc4, 0x57, 0x93, 0xc4, 0xa7, 0xf2, 0x55, 0xa7, 0x7e, 0x82, 0xfc, 0x7e, 0x3d, 0x47, 0x7a, 0x3d,
    0x64, 0xac, 0xc8, 0x64, 0x5d, 0xe7, 0xba, 0x5d, 0x19, 0x2b, 0x32, 0x19, 0x73, 0x95, 0xe6, 0x73,
    0x60, 0xa0, 0xc0, 0x60, 0x81, 0x98, 0x19, 0x81, 0x4f, 0xd1, 0x9e, 0x4f, 0xdc, 0x7f, 0xa3, 0xdc,
    0x22, 0x66, 0x44, 0x22, 0x2a, 0x7e, 0x54, 0x2a, 0x90, 0xab, 0x3b, 0x90, 0x88, 0x83, 0x0b, 0x88,
    0x46, 0xca, 0x8c, 0x46, 0xee, 0x29, 0xc7, 0xee, 0xb8, 0xd3, 0x6b, 0xb8, 0x14, 0x3c, 0x28, 0x14,
    0xde, 0x79, 0xa7, 0xde, 0x5e, 0xe2, 0xbc, 0x5e, 0x0b, 0x1d, 0x16, 0x0b, 0xdb, 0x76, 0xad, 0xdb,
    0xe0, 0x3b, 0xdb, 0xe0, 0x32, 0x56, 0x64, 0x32, 0x3a, 0x4e, 0x74, 0x3a, 0x0a, 0x1e, 0x14, 0x0a,
    0x49, 0xdb, 0x92, 0x49, 0x06, 0x0a, 0x0c, 0x06, 0x24, 0x6c, 0x48, 0x24, 0x5c, 0xe4, 0xb8, 0x5c,
    0xc2, 0x5d, 0x9f, 0xc2, 0xd3, 0x6e, 0xbd, 0xd3, 0xac, 0xef, 0x43, 0xac, 0x62, 0xa6, 0xc4, 0x62,
    0x91, 0xa8, 0x39, 0x91, 0x95, 0xa4, 0x31, 0x95, 0xe4, 0x37, 0xd3, 0xe4, 0x79, 0x8b, 0xf2, 0x79,
    0xe7, 0x32, 0xd5, 0xe7, 0xc8, 0x43, 0x8b, 0xc8, 0x37, 0x59, 0x6e, 0x37, 0x6d, 0xb7, 0xda, 0x6d,
    0x8d, 0x8c, 0x01, 0x8d, 0xd5, 0x64, 0xb1, 0xd5, 0x4e, 0xd2, 0x9c, 0x4e, 0xa9, 0xe0, 0x49, 0xa9,
    0x6c, 0xb4, 0xd8, 0x6c, 0x56, 0xfa, 0xac, 0x56, 0xf4, 0x07, 0xf3, 0xf4, 0xea, 0x25, 0xcf, 0xea,
    0x65, 0xaf, 0xca, 0x65, 0x7a, 0x8e, 0xf4, 0x7a, 0xae, 0xe9, 0x47, 0xae, 0x08, 0x18, 0x10, 0x08,
    0xba, 0xd5, 0x6f, 0xba, 0x78, 0x88, 0xf0, 0x78, 0x25, 0x6f, 0x4a, 0x25, 0x2e, 0x72, 0x5c, 0x2e,
    0x1c, 0x24, 0x38, 0x1c, 0xa6, 0xf1, 0x57, 0xa6, 0xb4, 0xc7, 0x73, 0xb4, 0xc6, 0x51, 0x97, 0xc6,
    0xe8, 0x23, 0xcb, 0xe8, 0xdd, 0x7c, 0xa1, 0xdd, 0x74, 0x9c, 0xe8, 0x74, 0x1f, 0x21, 0x3e, 0x1f,
    0x4b, 0xdd, 0x96, 0x4b, 0xbd, 0xdc, 0x61, 0xbd, 0x8b, 0x86, 0x0d, 0x8b, 0x8a, 0x85, 0x0f, 0x8a,
    0x70, 0x90, 0xe0, 0x70, 0x3e, 0x42, 0x7c, 0x3e, 0xb5, 0xc4, 0x71, 0xb5, 0x66, 0xaa, 0xcc, 0x66,
    0x48, 0xd8, 0x90, 0x48, 0x03, 0x05, 0x06, 0x03, 0xf6, 0x01, 0xf7, 0xf6, 0x0e, 0x12, 0x1c, 0x0e,
    0x61, 0xa3, 0xc2, 0x61, 0x35, 0x5f, 0x6a, 0x35, 0x57, 0xf9, 0xae, 0x57, 0xb9, 0xd0, 0x69, 0xb9,
    0x86, 0x91, 0x17, 0x86, 0xc1, 0x58, 0x99, 0xc1, 0x1d, 0x27, 0x3a, 0x1d, 0x9e, 0xb9, 0x27, 0x9e,
    0xe1, 0x38, 0xd9, 0xe1, 0xf8, 0x13, 0xeb, 0xf8, 0x98, 0xb3, 0x2b, 0x98, 0x11, 0x33, 0x22, 0x11,
    0x69, 0xbb, 0xd2, 0x69, 0xd9, 0x70, 0xa9, 0xd9, 0x8e, 0x89, 0x07, 0x8e, 0x94, 0xa7, 0x33, 0x94,
    0x9b, 0xb6, 0x2d, 0x9b, 0x1e, 0x22, 0x3c, 0x1e, 0x87, 0x92, 0x15, 0x87, 0xe9, 0x20, 0xc9, 0xe9,
    0xce, 0x49, 0x87, 0xce, 0x55, 0xff, 0xaa, 0x55, 0x28, 0x78, 0x50, 0x28, 0xdf, 0x7a, 0xa5, 0xdf,
    0x8c, 0x8f, 0x03, 0x8c, 0xa1, 0xf8, 0x59, 0xa1, 0x89, 0x80, 0x09, 0x89, 0x0d, 0x17, 0x1a, 0x0d,
    0xbf, 0xda, 0x65, 0xbf, 0xe6, 0x31, 0xd7, 0xe6, 0x42, 0xc6, 0x84, 0x42, 0x68, 0xb8, 0xd0, 0x68,
    0x41, 0xc3, 0x82, 0x41, 0x99, 0xb0, 0x29, 0x99, 0x2d, 0x77, 0x5a, 0x2d, 0x0f, 0x11, 0x1e, 0x0f,
    0xb0, 0xcb, 0x7b, 0xb0, 0x54, 0xfc, 0xa8, 0x54, 0xbb, 0xd6, 0x6d, 0xbb, 0x16, 0x3a, 0x2c, 0x16,
    0x63, 0x63, 0xa5, 0xc6, 0x7c, 0x7c, 0x84, 0xf8, 0x77, 0x77, 0x99, 0xee, 0x7b, 0x7b, 0x8d, 0xf6,
    0xf2, 0xf2, 0x0d, 0xff, 0x6b, 0x6b, 0xbd, 0xd6, 0x6f, 0x6f, 0xb1, 0xde, 0xc5, 0xc5, 0x54, 0x91,
    0x30, 0x30, 0x50, 0x60, 0x01, 0x01, 0x03, 0x02, 0x67, 0x67, 0xa9, 0xce, 0x2b, 0x2b, 0x7d, 0x56,
    0xfe, 0xfe, 0x19, 0xe7, 0xd7, 0xd7, 0x62, 0xb5, 0xab, 0xab, 0xe6, 0x4d, 0x76, 0x76, 0x9a, 0xec,
    0xca, 0xca, 0x45, 0x8f, 0x82, 0x82, 0x9d, 0x1f, 0xc9, 0xc9, 0x40, 0x89, 0x7d, 0x7d, 0x87, 0xfa,
    0xfa, 0xfa, 0x15, 0xef, 0x59, 0x59, 0xeb, 0xb2, 0x47, 0x47, 0xc9, 0x8e, 0xf0, 0xf0, 0x0b, 0xfb,
    0xad, 0xad, 0xec, 0x41, 0xd4, 0xd4, 0x67, 0xb3, 0xa2, 0xa2, 0xfd, 0x5f, 0xaf, 0xaf, 0xea, 0x45,
    0x9c, 0x9c, 0xbf, 0x23, 0xa4, 0xa4, 0xf7, 0x53, 0x72, 0x72, 0x96, 0xe4, 0xc0, 0xc0, 0x5b, 0x9b,
    0xb7, 0xb7, 0xc2, 0x75, 0xfd, 0xfd, 0x1c, 0xe1, 0x93, 0x93, 0xae, 0x3d, 0x26, 0x26, 0x6a, 0x4c,
    0x36, 0x36, 0x5a, 0x6c, 0x3f, 0x3f, 0x41, 0x7e, 0xf7, 0xf7, 0x02, 0xf5, 0xcc, 0xcc, 0x4f, 0x83,
    0x34, 0x34, 0x5c, 0x68, 0xa5, 0xa5, 0xf4, 0x51, 0xe5, 0xe5, 0x34, 0xd1, 0xf1, 0xf1, 0x08, 0xf9,
    0x71, 0x71, 0x93, 0xe2, 0xd8, 0xd8, 0x73, 0xab, 0x31, 0x31, 0x53, 0x62, 0x15, 0x15, 0x3f, 0x2a,
    0x04, 0x04, 0x0c, 0x08, 0xc7, 0xc7, 0x52, 0x95, 0x23, 0x23, 0x65, 0x46, 0xc3, 0xc3, 0x5e, 0x9d,
    0x18, 0x18, 0x28, 0x30, 0x96, 0x96, 0xa1, 0x37, 0x05, 0x05, 0x0f, 0x0a, 0x9a, 0x9a, 0xb5, 0x2f,
    0x07, 0x07, 0x09, 0x0e, 0x12, 0x12, 0x36, 0x24, 0x80, 0x80, 0x9b, 0x1b, 0xe2, 0xe2, 0x3d, 0xdf,
    0xeb, 0xeb, 0x26, 0xcd, 0x27, 0x27, 0x69, 0x4e, 0xb2, 0xb2, 0xcd, 0x7f, 0x75, 0x75, 0x9f, 0xea,
    0x09, 0x09, 0x1b, 0x12, 0x83, 0x83, 0x9e, 0x1d, 0x2c, 0x2c, 0x74, 0x58, 0x1a, 0x1a, 0x2e, 0x34,
    0x1b, 0x1b, 0x2d, 0x36, 0x6e, 0x6e, 0xb2, 0xdc, 0x5a, 0x5a, 0xee, 0xb4, 0xa0, 0xa0, 0xfb, 0x5b,
    0x52, 0x52, 0xf6, 0xa4, 0x3b, 0x3b, 0x4d, 0x76, 0xd6, 0xd6, 0x61, 0xb7, 0xb3, 0xb3, 0xce, 0x7d,
    0x29, 0x29, 0x7b, 0x52, 0xe3, 0xe3, 0x3e, 0xdd, 0x2f, 0x2f, 0x71, 0x5e, 0x84, 0x84, 0x97, 0x13,
    0x53, 0x53, 0xf5, 0xa6, 0xd1, 0xd1, 0x68, 0xb9, 0x00, 0x00, 0x00, 0x00, 0xed, 0xed, 0x2c, 0xc1,
    0x20, 0x20, 0x60, 0x40, 0xfc, 0xfc, 0x1f, 0xe3, 0xb1, 0xb1, 0xc8, 0x79, 0x5b, 0x5b, 0xed, 0xb6,
    0x6a, 0x6a, 0xbe, 0xd4, 0xcb, 0xcb, 0x46, 0x8d, 0xbe, 0xbe, 0xd9, 0x67, 0x39, 0x39, 0x4b, 0x72,
    0x4a, 0x4a, 0xde, 0x94, 0x4c, 0x4c, 0xd4, 0x98, 0x58, 0x58, 0xe8, 0xb0, 0xcf, 0xcf, 0x4a, 0x85,
    0xd0, 0xd0, 0x6b, 0xbb, 0xef, 0xef, 0x2a, 0xc5, 0xaa, 0xaa, 0xe5, 0x4f, 0xfb, 0xfb, 0x16, 0xed,
    0x43, 0x43, 0xc5, 0x86, 0x4d, 0x4d, 0xd7, 0x9a, 0x33, 0x33, 0x55, 0x66, 0x85, 0x85, 0x94, 0x11,
    0x45, 0x45, 0xcf, 0x8a, 0xf9, 0xf9, 0x10, 0xe9, 0x02, 0x02, 0x06, 0x04, 0x7f, 0x7f, 0x81, 0xfe,
    0x50, 0x50, 0xf0, 0xa0, 0x3c, 0x3c, 0x44, 0x78, 0x9f, 0x9f, 0xba, 0x25, 0xa8, 0xa8, 0xe3, 0x4b,
    0x51, 0x51, 0xf3, 0xa2, 0xa3, 0xa3, 0xfe, 0x5d, 0x40, 0x40, 0xc0, 0x80, 0x8f, 0x8f, 0x8a, 0x05,
    0x92, 0x92, 0xad, 0x3f, 0x9d, 0x9d, 0xbc, 0x21, 0x38, 0x38, 0x48, 0x70, 0xf5, 0xf5, 0x04, 0xf1,
    0xbc, 0xbc, 0xdf, 0x63, 0xb6, 0xb6, 0xc1, 0x77, 0xda, 0xda, 0x75, 0xaf, 0x21, 0x21, 0x63, 0x42,
    0x10, 0x10, 0x30, 0x20, 0xff, 0xff, 0x1a, 0xe5, 0xf3, 0xf3, 0x0e, 0xfd, 0xd2, 0xd2, 0x6d, 0xbf,
    0xcd, 0xcd, 0x4c, 0x81, 0x0c, 0x0c, 0x14, 0x18, 0x13, 0x13, 0x35, 0x26, 0xec, 0xec, 0x2f, 0xc3,
    0x5f, 0x5f, 0xe1, 0xbe, 0x97, 0x97, 0xa2, 0x35, 0x44, 0x44, 0xcc, 0x88, 0x17, 0x17, 0x39, 0x2e,
    0xc4, 0xc4, 0x57, 0x93, 0xa7, 0xa7, 0xf2, 0x55, 0x7e, 0x7e, 0x82, 0xfc, 0x3d, 0x3d, 0x47, 0x7a,
    0x64, 0x64, 0xac, 0xc8, 0x5d, 0x5d, 0xe7, 0xba, 0x19, 0x19, 0x2b, 0x32, 0x73, 0x73, 0x95, 0xe6,
    0x60, 0x60, 0xa0, 0xc0, 0x81, 0x81, 0x98, 0x19, 0x4f, 0x4f, 0xd1, 0x9e, 0xdc, 0xdc, 0x7f, 0xa3,
    0x22, 0x22, 0x66, 0x44, 0x2a, 0x2a, 0x7e, 0x54, 0x90, 0x90, 0xab, 0x3b, 0x88, 0x88, 0x83, 0x0b,
    0x46, 0x46, 0xca, 0x8c, 0xee, 0xee, 0x29, 0xc7, 0xb8, 0xb8, 0xd3, 0x6b, 0x14, 0x14, 0x3c, 0x28,
    0xde, 0xde, 0x79, 0xa7, 0x5e, 0x5e, 0xe2, 0xbc, 0x0b, 0x0b, 0x1d, 0x16, 0xdb, 0xdb, 0x76, 0xad,
    0xe0, 0xe0, 0x3b, 0xdb, 0x32, 0x32, 0x56, 0x64, 0x3a, 0x3a, 0x4e, 0x74, 0x0a, 0x0a, 0x1e, 0x14,
    0x49, 0x49, 0xdb, 0x92, 0x06, 0x06, 0x0a, 0x0c, 0x24, 0x24, 0x6c, 0x48, 0x5c, 0x5c, 0xe4, 0xb8,
    0xc2, 0xc2, 0x5d, 0x9f, 0xd3, 0xd3, 0x6e, 0xbd, 0xac, 0xac, 0xef, 0x43, 0x62, 0x62, 0xa6, 0xc4,
    0x91, 0x91, 0xa8, 0x39, 0x95, 0x95, 0xa4, 0x31, 0xe4, 0xe4, 0x37, 0xd3, 0x79, 0x79, 0x8b, 0xf2,
    0xe7, 0xe7, 0x32, 0xd5, 0xc8, 0xc8, 0x43, 0x8b, 0x37, 0x37, 0x59, 0x6e, 0x6d, 0x6d, 0xb7, 0xda,
    0x8d, 0x8d, 0x8c, 0x01, 0xd5, 0xd5, 0x64, 0xb1, 0x4e, 0x4e, 0xd2, 0x9c, 0xa9, 0xa9, 0xe0, 0x49,
    0x6c, 0x6c, 0xb4, 0xd8, 0x56, 0x56, 0xfa, 0xac, 0xf4, 0xf4, 0x07, 0xf3, 0xea, 0xea, 0x25, 0xcf,
    0x65, 0x65, 0xaf, 0xca, 0x7a, 0x7a, 0x8e, 0xf4, 0xae, 0xae, 0xe9, 0x47, 0x08, 0x08, 0x18, 0x10,
    0xba, 0xba, 0xd5, 0x6f, 0x78, 0x78, 0x88, 0xf0, 0x25, 0x25, 0x6f, 0x4a, 0x2e, 0x2e, 0x72, 0x5c,
    0x1c, 0x1c, 0x24, 0x38, 0xa6, 0xa6, 0xf1, 0x57, 0xb4, 0xb4, 0xc7, 0x73, 0xc6, 0xc6, 0x51, 0x97,
    0xe8, 0xe8, 0x23, 0xcb, 0xdd, 0xdd, 0x7c, 0xa1, 0x74, 0x74, 0x9c, 0xe8, 0x1f, 0x1f, 0x21, 0x3e,
    0x4b, 0x4b, 0xdd, 0x96, 0xbd, 0xbd, 0xdc, 0x61, 0x8b, 0x8b, 0x86, 0x0d, 0x8a, 0x8a, 0x85, 0x0f,
    0x70, 0x70, 0x90, 0xe0, 0x3e, 0x3e, 0x42, 0x7c, 0xb5, 0xb5, 0xc4, 0x71, 0x66, 0x66, 0xaa, 0xcc,
    0x48, 0x48, 0xd8, 0x90, 0x03, 0x03, 0x05, 0x06, 0xf6, 0xf6, 0x01, 0xf7, 0x0e, 0x0e, 0x12, 0x1c,
    0x61, 0x61, 0xa3, 0xc2, 0x35, 0x35, 0x5f, 0x6a, 0x57, 0x57, 0xf9, 0xae, 0xb9, 0xb9, 0xd0, 0x69,
    0x86, 0x86, 0x91, 0x17, 0xc1, 0xc1, 0x58, 0x99, 0x1d, 0x1d, 0x27, 0x3a, 0x9e, 0x9e, 0xb9, 0x27,
    0xe1, 0xe1, 0x38, 0xd9, 0xf8, 0xf8, 0x13, 0xeb, 0x98, 0x98, 0xb3, 0x2b, 0x11, 0x11, 0x33, 0x22,
    0x69, 0x69, 0xbb, 0xd2, 0xd9, 0xd9, 0x70, 0xa9, 0x8e, 0x8e, 0x89, 0x07, 0x94, 0x94, 0xa7, 0x33,
    0x9b, 0x9b, 0xb6, 0x2d, 0x1e, 0x1e, 0x22, 0x3c, 0x87, 0x87, 0x92, 0x15, 0xe9, 0xe9, 0x20, 0xc9,
    0xce, 0xce, 0x49, 0x87, 0x55, 0x55, 0xff, 0xaa, 0x28, 0x28, 0x78, 0x50, 0xdf, 0xdf, 0x7a, 0xa5,
    0x8c, 0x8c, 0x8f, 0x03, 0xa1, 0xa1, 0xf8, 0x59, 0x89, 0x89, 0x80, 0x09, 0x0d, 0x0d, 0x17, 0x1a,
    0xbf, 0xbf, 0xda, 0x65, 0xe6, 0xe6, 0x31, 0xd7, 0x42, 0x42, 0xc6, 0x84, 0x68, 0x68, 0xb8, 0xd0,
    0x41, 0x41, 0xc3, 0x82, 0x99, 0x99, 0xb0, 0x29, 0x2d, 0x2d, 0x77, 0x5a, 0x0f, 0x0f, 0x11, 0x1e,
    0xb0, 0xb0, 0xcb, 0x7b, 0x54, 0x54, 0xfc, 0xa8, 0xbb, 0xbb, 0xd6, 0x6d, 0x16, 0x16, 0x3a, 0x2c,
];

#[inline]
fn rotate_word(word: &[u8; 4]) -> [u8; 4] {
    [word[1], word[2], word[3], word[0]]
}

fn substitute_word(word: &[u8; 4]) -> [u8; 4] {
    let mut result = [0u8; 4];

    for i in 0..4 {
        let row = (word[i] >> 4) as usize;
        let col = (word[i] & 0x0F) as usize;
        result[i] = AES_SBOX[row][col];
    }

    result
}

#[inline]
fn xor_words(w1: &[u8; 4], w2: &[u8; 4]) -> [u8; 4] {
    [w1[0] ^ w2[0], w1[1] ^ w2[1], w1[2] ^ w2[2], w1[3] ^ w2[3]]
}

/// Extends the key in the same way as it is extended for AES256, but for
/// Cryptonight's hash we only need to extend to 10 round keys instead of 15
/// like AES256.
pub(crate) fn key_extend(key_bytes: &[u8; CN_AES_KEY_SIZE]) -> [[u8; 4]; NUM_AES_ROUND_KEYS * 4] {
    // NK comes from the AES specification, it is the number of 32-bit words in
    // the non-expanded key (For AES-256: 32/4 = 8)
    const NK: usize = 8;
    const WORDS_PER_ROUND_KEY: usize = 4;
    let mut expanded_key = [[0u8; 4]; NUM_AES_ROUND_KEYS * 4];

    // The base key forms
    for i in 0..CN_AES_KEY_SIZE {
        expanded_key[i / 4][i % 4] = key_bytes[i];
    }

    /// See FIPS-197, especially figure 11 to better understand how the expansion
    /// happens: https://nvlpubs.nist.gov/nistpubs/fips/nist.fips.197.pdf
    const ROUND_CONSTS: [u8; 11] = [
        0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36,
    ];

    // expand to 10 round keys (40 total words, 160 total bytes)
    const EXPAND_START: usize = CN_AES_KEY_SIZE / WORDS_PER_ROUND_KEY;
    const EXPAND_END: usize = NUM_AES_ROUND_KEYS * WORDS_PER_ROUND_KEY;
    for i in EXPAND_START..EXPAND_END {
        let mut temp = expanded_key[i - 1];
        if i % NK == 0 {
            let rc = [ROUND_CONSTS[i / NK], 0, 0, 0];
            temp = xor_words(&substitute_word(&rotate_word(&temp)), &rc);
        } else if i % NK == 4 {
            temp = substitute_word(&temp)
        }

        expanded_key[i] = xor_words(&expanded_key[i - CN_AES_KEY_SIZE / 4], &temp);
    }

    return expanded_key;
}

fn state_in(state: &mut [[u8; 4]; 4], input: &[u8]) {
    for i in 0..4 {
        for j in 0..4 {
            state[i][j] = input[i * 4 + j];
        }
    }
}

fn state_out(output: &mut [u8], state: [[u8; 4]; 4]) {
    for i in 0..4 {
        for j in 0..4 {
            output[i * 4 + j] = state[i][j];
        }
    }
}

pub(crate) fn round_fwd(state: &mut [[u8; 4]; 4], keys: &[[u8; 4]]) {
    debug_assert_eq!(keys.len(), 4);

    #[rustfmt::skip]
    const INDEX_ROTATIONS: [[u8; 4]; 4] = [
        [0, 1, 2, 3],
        [1, 2, 3, 0],
        [2, 3, 0, 1],
        [3, 0, 1, 2],
    ];

    let start_state = state.clone();

    for c in 0..4 {
        for i in 0..4 {
            let mut r = 0u8;
            for j in 0..4 {
                let w = INDEX_ROTATIONS[j][c] as usize;
                let s = start_state[w][j] as usize;
                r ^= CRYPTONIGHT_SBOX[j * 256 * 4 + s * 4 + i]; // max: 3*256*4 + 255*4 + 3 = 4095
            }
            state[c][i] = r ^ keys[c][i];
        }
    }
}

pub(crate) fn aesb_pseudo_round(block: &mut [u8], expanded_key: &[[u8; 4]; 40]) {
    debug_assert!(block.len() == AES_BLOCK_SIZE);

    let mut state = [[0u8; 4]; 4];

    state_in(&mut state, block);

    for i in (0..40).step_by(4) {
        round_fwd(&mut state, &expanded_key[i..i + 4]);
    }

    state_out(block, state);
}

pub(crate) fn aesb_single_round(block: &mut [u8], round_key_flat: &[u8; ROUND_KEY_SIZE]) {
    debug_assert!(block.len() == AES_BLOCK_SIZE);

    let mut round_key = [[0u8; 4]; 4];
    for i in 0..4 {
        round_key[i] = [
            round_key_flat[4 * i],
            round_key_flat[4 * i + 1],
            round_key_flat[4 * i + 2],
            round_key_flat[4 * i + 3],
        ];
    }

    let mut state = [[0u8; 4]; 4];

    state_in(&mut state, block);

    round_fwd(&mut state, &round_key[..]);

    state_out(block, state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_hex_to_array<const N: usize>(hex: &str) -> [u8; N] {
        assert_eq!(
            hex.len(),
            N * 2,
            "Hex string length must be twice the array size"
        );
        let mut bytes = [0u8; N];
        for i in 0..N {
            bytes[i] = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("Invalid hex string");
        }
        bytes
    }

    #[test]
    fn test_key_schedule() {
        let test = |key_hex: &str, expected_out: &str| {
            let key = decode_hex_to_array(key_hex);
            let expanded_key = key_extend(&key.into());
            let flat_expanded_key = expanded_key.iter().flatten().copied().collect::<Vec<u8>>();
            assert_eq!(expected_out, hex::encode(flat_expanded_key));
        };
        test(
            "ac156e17cdabc0b92e3e724a06ef21e5317eb71fbc7f1587403b30ae6962a21a",
            "ac156e17cdabc0b92e3e724a06ef21e5317eb71fbc7f1587403b30ae6962a21a072fcceeca840c57e4ba7e1de2555ff8a982785e15fd6dd955c65d773ca4ff6d4c39f00586bdfc526207824f8052ddb76482b9f7717fd42e24b98959181d7634ec01e8a86abc14fa08bb96b588e94b02a09c0a80d1e3deaef55a57f7ed4721c344fcc6fd2e40d20726fb44b2ae120fb044557c6795b6a2c960ecf53e8dabd4fd",
        );
        test(
            "688dcc56a1c9b8c9cd9e378a98a1388f17a2c05a698a37232ecd4a567dccdf79",
            "688dcc56a1c9b8c9cd9e378a98a1388f17a2c05a698a37232ecd4a567dccdf7922137aa983dac2604e44f5ead6e5cd65e17b7d1788f14a34a63c0062dbf0df1bac8dd5102f5717706113e29ab7f62fff48396801c0c8223566f42257bd04fd4c5ad9fc6a758eeb1a149d0980a36b267f42469fd3828ebde6e47a9fb1597e62fda173a8a1d4fd43bbc0604a3b630b6c44b96dcfc83be3722edf99ed9f86e78f62",
        );
        test(
            "a6116fc295f15ff03d538581a560a9c1fdaa1e7f5745d8e6125d6eb092c71b15",
            "a6116fc295f15ff03d538581a560a9c1fdaa1e7f5745d8e6125d6eb092c71b1561be368df44f697dc91cecfc6c7c453dadba7058faffa8bee8a2c60e7a65dd1b2e7f9957da30f02a132c1cd67f5059eb7fe9bbb18516130f6db4d50117d1081a144f3ba7ce7fcb8ddd53d75ba2038eb04592a256c084b159ad306458bae16c42e41f17532a60dcdef7330b8555308535b99635c079128499d422e0c16ec38c83",
        );
        test(
            "80784e1c1d3730e6f422aae6b10596ab16b190e41eea452af9aeedc97aee4b74",
            "80784e1c1d3730e6f422aae6b10596ab16b190e41eea452af9aeedc97aee4b74a9cbdcc6b4fcec2040de46c6f1dbd06db708e0d8a9e2a5f2504c483b2aa2034f91b05823254cb4036592f2c5944922a89533731a3cd1d6e86c9d9ed3463f9d9ce0ee8679c5a2327aa030c0bf3479e2178d85ebeab1543d02ddc9a3d19bf63e4daa5c656d6ffe5717cfce97a8fbb775bf822c76e233784be0eeb1e8317547d67c",
        );
        test(
            "cc08712809fd4c0f0b63dc21657f22b3752fba8f2ed5882e7d75e65906bb3399",
            "cc08712809fd4c0f0b63dc21657f22b3752fba8f2ed5882e7d75e65906bb339927cb9f472e36d34825550f69402a2dda7cca62d8521feaf62f6a0caf29d13f361bbe9ae2358849aa10dd46c350f76b192fa21d0c7dbdf7fa52d7fb557b06c46370a261c3452a286955f76eaa050005b344c17661397c819b6bab7ace10adbeaded0cf409a826dc60fdd1b2caf8d1b77905ffdfd73c835e4c5728248247859a2f",
        );
    }

    #[test]
    fn test_aesb_pseudo_round() {
        let test = |key_hex: &str, input_hex: &str, expected_out: &str| {
            let key: [u8; 32] = decode_hex_to_array(key_hex);
            let extended_key = key_extend(&key.into());
            let mut block: [u8; 16] = decode_hex_to_array(input_hex);

            aesb_pseudo_round(&mut block, &extended_key);
            assert_eq!(expected_out, hex::encode(block));
        };

        test(
            "1d0b47a047340e32cbe890ca0d61720a09bcfb39e01b7541d1100d1ef91f955f",
            "274fe9eeb2d1e4c71f0f0244a80e93a1",
            "be98612d6b05a6cd72df39326180066a",
        );
        test(
            "0093d86fe74698f7b02774e6d4f67e9e29eb71d1754804a19b77d986b8141434",
            "110f2e5d81f73a512ec95aa5b8e0d7be",
            "1f1750d997704943b828df66661f7cbf",
        );
        test(
            "d5044939d15af565447ef76445405cd899f81c6f41f4493a5a1323712f815e53",
            "6022f491b67e27909f74d0e71becebaa",
            "9f75d250681954d60e418b4333d247a5",
        );
        test(
            "256670ed9eba1db67e6ddec5dfb78f6bfbf55d0f74e2a46d06f2e3592a208014",
            "4de6ecad6a885ac88f09f9b2be4145fb",
            "cb286e70825609cb97b7c7ae72548fa9",
        );
        test(
            "e1077c3566d1e8bfeb2e8e48540ed76fb61e973f4951a821c3e8bb918facc03d",
            "2a2ff0dd38c79ab13fb6b06751824e93",
            "82f65ba66f8fc6d8e1f4e1f41976eed8",
        );
        test(
            "dee818b6a894121e5e967e2218bb8772b9486bec2241377fdcfed7db75f3b724",
            "eebc705f33d00fdf7c8add2481c62767",
            "bee070b25e969ea87578daa1c7831651",
        );
        test(
            "c9b653644f3d3adc3498c029a1373b63f548e853deadc48e559b1a0a05e5c543",
            "bef0968fc6adb8ce96bfa99642481624",
            "859fc5f637ee1ee835b6f9a3f16a41f8",
        );
        test(
            "8e65798ebbae347c969ef9778e04e06649e3765aa58f5cd776b6ee58afde98ff",
            "629f87e95b67e7bd5a3af528379cbef7",
            "04a697b4fb82466950e9c0668e8c3eb9",
        );
        test(
            "4c0f6a402316b3a73e2a778f20ca3f8335e7a7bb5aecdaf9db91664604b74d62",
            "3c9ab665451100d8d21029f96edf85f3",
            "de5e23b1ba21a16ac01098937b26f3a9",
        );
        test(
            "a0b2cb30088b6145d9651ed019b0d051e4e6bf6cc0c8165dc76e3aa9fa9849f0",
            "6a007f218c3f8b97c8489fe56433c99a",
            "1885d448a81b0a048cc241275b9d7dce",
        );
    }

    #[test]
    fn test_aesb_single_round() {
        let test = |key_hex: &str, input_hex: &str, expected_out: &str| {
            let round_key: [u8; ROUND_KEY_SIZE] = decode_hex_to_array(key_hex);
            let mut block: [u8; AES_BLOCK_SIZE] = decode_hex_to_array(input_hex);

            aesb_single_round(&mut block, &round_key);
            assert_eq!(expected_out, hex::encode(block));
        };

        test(
            "9af7bd044f96bba5251ebd8065f4c757",
            "8844d7f6f6aa2df5706ef0e7b26a3410",
            "a03593fc2b9b906069bfc3a86a12e7fe",
        );
        test(
            "9749a59d1ee692c3b70b9c38a0c88369",
            "f96b5a1984f7c57b92d7b2e82dd0ce46",
            "6b03d4c6edf7a914265ca765b784c5ee",
        );
        test(
            "20959275e137a08267e35afe66f9adeb",
            "3fce6f546d8fbc15bd9c6ac7d533eae4",
            "34692b49471e37df3cbe43a9459ebe97",
        );
        test(
            "5c429524d022dc5dd48f7cd529fdf4f2",
            "3edae93308c9aab4dfb6bfcd8e4012af",
            "2e3061ce680d75177bac5b7af3182543",
        );
        test(
            "e76c56ca69a7309866a730e8976da086",
            "39d8ee732a115b6b21f89ca181bd9ddc",
            "069ef0b7aaada2b65ea9665827dae9ae",
        );
        test(
            "afd540af324c4fcda6246c657424a3ce",
            "a5a01d75141522ff1ea717083abc5b5e",
            "e0320cbc9dd8279c5f7d121ef7e1ae46",
        );
        test(
            "dfe9ba9468ccf1ac20a305730d1bdcb7",
            "7be56ce9d924bf2fc4b574e225676f3c",
            "bee2b49ed0b578b2c94b03a8930d990c",
        );
        test(
            "381e788a8d3389f27fe9aff054a0b407",
            "b8d2600f71b0e9535d17c00ba90246f6",
            "2a305ae7f7f3f44a43cd0342180b9394",
        );
        test(
            "16a94460158a5512052626f6cb080d6d",
            "5ea0c238c05b8f3a913c1b36102eabeb",
            "ab8040d7395cc940ea2a47610989ceb1",
        );
        test(
            "7e584682efb38bf2adfc6f1958fe08ff",
            "80c78bb6ca2f114cbcb49fbaadaee9d1",
            "25f7717cdbaa9c614424ef3d4e9543ec",
        );
    }
}
