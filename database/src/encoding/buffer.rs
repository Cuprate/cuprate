use super::Buffer;

impl Buffer for Vec<u8> {
	fn new() -> Self {
		Vec::new()
	}
}

macro_rules! impl_buffer_for_array {
	($size:expr) => {
		impl Buffer for [u8; $size] {
			fn new() -> Self {
				[0u8; $size]
			}
		}
	};
}

impl_buffer_for_array!(0);
impl_buffer_for_array!(1);
impl_buffer_for_array!(2);
impl_buffer_for_array!(4);
impl_buffer_for_array!(8);
impl_buffer_for_array!(16);
impl_buffer_for_array!(24);
impl_buffer_for_array!(32);
impl_buffer_for_array!(88);
impl_buffer_for_array!(120);