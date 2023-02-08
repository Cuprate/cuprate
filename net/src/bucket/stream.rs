use super::{Bucket, BucketBody, BucketError, BucketHead};

pub enum DecoderState {
    WaitingForHeader,
    WaitingForBody(BucketHead),
}

impl Default for DecoderState {
    fn default() -> Self {
        DecoderState::WaitingForHeader
    }
}

impl DecoderState {
    fn handle_waiting_for_header(
        &mut self,
        mut bytes: &[u8],
    ) -> Result<Option<BucketHead>, BucketError> {
        if bytes.len() < BucketHead::SIZE {
            Ok(None)
        } else {
            Ok(Some(BucketHead::from_bytes(&mut bytes)?))
        }
    }

    pub fn received_bytes(&mut self, bytes: &[u8]) -> Result<(Option<Bucket>, usize), BucketError> {
        let mut len = 0;
        loop {
            match self {
                DecoderState::WaitingForHeader => {
                    let header = self.handle_waiting_for_header(bytes)?;
                    let Some(header) = header else {
                        return Ok((None, 0));
                    };
                    *self = DecoderState::WaitingForBody(header);
                    len = BucketHead::SIZE;
                }

                &mut DecoderState::WaitingForBody(header) => {
                    if header.size > (bytes.len() - len) as u64 {
                        return Ok((None, len));
                    }

                    let body = BucketBody::from_bytes(
                        &bytes[len..],
                        header.have_to_return_data,
                        header.flags,
                        header.command,
                    )?;
                    let bucket = Bucket { header, body };
                    *self = DecoderState::WaitingForHeader;
                    return Ok((Some(bucket), len + header.size as usize));
                }
            }
        }
    }
}

pub struct BucketStream {
    buffer: Vec<u8>,
    state: DecoderState,
}

impl Default for BucketStream {
    fn default() -> Self {
        BucketStream {
            buffer: Vec::new(),
            state: DecoderState::WaitingForHeader,
        }
    }
}

impl BucketStream {
    pub fn try_decode_next_bucket(&mut self) -> Result<Option<Bucket>, BucketError> {
        let (bucket, len) = self.state.received_bytes(&self.buffer)?;
        self.buffer = self.buffer.drain(len..).collect();
        Ok(bucket)
    }

    pub fn received_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }
}



#[cfg(test)]
mod tests {
    use super::BucketStream;

    #[test]
    fn decode_message() {
        let bytes = [1, 33, 1, 1, 1, 1, 1, 1, 211, 1, 0, 0, 0, 0, 0, 0, 0, 233, 3, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 1, 17, 1, 1, 1, 1, 2, 1, 1, 12, 9, 110, 111, 100, 101, 95, 100, 97, 116, 97, 12, 24, 7, 109, 121, 95, 112, 111, 114, 116, 6, 168, 70, 0, 0, 10, 110, 101, 116, 119, 111, 114, 107, 95, 105, 100, 10, 64, 18, 48, 241, 113, 97, 4, 65, 97, 23, 49, 0, 130, 22, 161, 161, 16, 7, 112, 101, 101, 114, 95, 105, 100, 5, 153, 5, 227, 61, 188, 214, 159, 10, 13, 115, 117, 112, 112, 111, 114, 116, 95, 102, 108, 97, 103, 115, 6, 1, 0, 0, 0, 8, 114, 112, 99, 95, 112, 111, 114, 116, 7, 0, 0, 20, 114, 112, 99, 95, 99, 114, 101, 100, 105, 116, 115, 95, 112, 101, 114, 95, 104, 97, 115, 104, 6, 0, 0, 0, 0, 12, 112, 97, 121, 108, 111, 97, 100, 95, 100, 97, 116, 97, 12, 24, 21, 99, 117, 109, 117, 108, 97, 116, 105, 118, 101, 95, 100, 105, 102, 102, 105, 99, 117, 108, 116, 121, 5, 59, 90, 163, 153, 0, 0, 0, 0, 27, 99, 117, 109, 117, 108, 97, 116, 105, 118, 101, 95, 100, 105, 102, 102, 105, 99, 117, 108, 116, 121, 95, 116, 111, 112, 54, 52, 5, 0, 0, 0, 0, 0, 0, 0, 0, 14, 99, 117, 114, 114, 101, 110, 116, 95, 104, 101, 105, 103, 104, 116, 5, 190, 50, 0, 0, 0, 0, 0, 0, 12, 112, 114, 117, 110, 105, 110, 103, 95, 115, 101, 101, 100, 6, 0, 0, 0, 0, 6, 116, 111, 112, 95, 105, 100, 10, 128, 230, 40, 186, 45, 79, 79, 224, 164, 117, 133, 84, 130, 185, 94, 4, 1, 57, 126, 74, 145, 238, 238, 122, 44, 214, 85, 129, 237, 230, 14, 67, 218, 11, 116, 111, 112, 95, 118, 101, 114, 115, 105, 111, 110, 8, 1, 18, 108, 111, 99, 97, 108, 95, 112, 101, 101, 114, 108, 105, 115, 116, 95, 110, 101, 119, 140, 4, 24, 3, 97, 100, 114, 12, 8, 4, 116, 121, 112, 101, 8, 1, 4, 97, 100, 100, 114, 12, 8, 4, 109, 95, 105, 112, 6, 225, 219, 21, 0, 6, 109, 95, 112, 111, 114, 116, 7, 0, 0, 2, 105, 100, 5, 0, 0, 0, 0, 0, 0, 0, 0, 9, 108, 97, 115, 116, 95, 115, 101, 101, 110, 1, 0, 0, 0, 0, 0, 0, 0, 0, 12, 112, 114, 117, 110, 105, 110, 103, 95, 115, 101, 101, 100, 6, 0, 0, 0, 0, 8, 114, 112, 99, 95, 112, 111, 114, 116, 7, 0, 0, 20, 114, 112, 99, 95, 99, 114, 101, 100, 105, 116, 115, 95, 112, 101, 114, 95, 104, 97, 115, 104, 6, 0, 0, 0, 0];

        let mut stream = BucketStream::default();

        stream.received_bytes(&bytes);

        println!("{:?}", stream.try_decode_next_bucket());

    }

    #[test]
    fn decode_message_in_parts() {
        let bytes = [1, 33, 1, 1, 1, 1, 1, 1, 6, 1, 0, 0, 0, 0, 0, 0, 1, 233, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 17, 1, 1, 1, 1, 2, 1, 1, 8, 9, 110, 111, 100, 101, 95, 100, 97, 116, 97, 12, 16, 7, 109, 121, 95, 112, 111, 114, 116, 6, 160, 70, 0, 0, 10, 110, 101, 116, 119, 111, 114, 107, 95, 105, 100, 10, 64, 18, 48, 241, 113, 97, 4, 65, 97, 23, 49, 0, 130, 22, 161, 161, 16, 7, 112, 101, 101, 114, 95, 105, 100, 5, 179, 104, 214, 194, 57, 124, 140, 194, 13, 115, 117, 112, 112, 111, 114, 116, 95, 102, 108, 97, 103, 115, 6, 1, 0, 0, 0, 12, 112, 97, 121, 108, 111, 97, 100, 95, 100, 97, 116, 97, 12, 20, 21, 99, 117, 109, 117, 108, 97, 116, 105, 118, 101, 95, 100, 105, 102, 102, 105, 99, 117, 108, 116, 121, 5, 59, 90, 163, 153, 0, 0, 0, 0, 27, 99, 117, 109, 117, 108, 97, 116, 105, 118, 101, 95, 100, 105, 102, 102, 105, 99, 117, 108, 116, 121, 95, 116, 111, 112, 54, 52, 5, 0, 0, 0, 0, 0, 0, 0, 0, 14, 99, 117, 114, 114, 101, 110, 116, 95, 104, 101, 105, 103, 104, 116, 5, 190, 50, 0, 0, 0, 0, 0, 0, 6, 116, 111, 112, 95, 105, 100, 10, 128, 230, 40, 186, 45, 79, 79, 224, 164, 117, 133, 84, 130, 185, 94, 4, 1, 57, 126, 74, 145, 238, 238, 122, 44, 214, 85, 129, 237, 230, 14, 67, 218, 11, 116, 111, 112, 95, 118, 101, 114, 115, 105, 111, 110, 8, 1];

        let mut stream = BucketStream::default();

        // input less bytes than the header
        stream.received_bytes(&bytes[0..20]);

        assert_eq!(stream.try_decode_next_bucket().unwrap(), None);

        // input the header (plus a bit more)
        stream.received_bytes(&bytes[20..200]);

        assert_eq!(stream.try_decode_next_bucket().unwrap(), None);

        // input the full amount
        stream.received_bytes(&bytes[200..]);

        println!("{:?}", stream.try_decode_next_bucket());

    }
}