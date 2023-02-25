use super::{Bucket, BucketError, BucketHead};

enum BucketDecoder {
    WaitingForHeader,
    WaitingForBody(BucketHead),
}

impl BucketDecoder {
    pub fn try_decode_bucket(
        &mut self,
        mut buf: &[u8],
    ) -> Result<(Option<Bucket>, usize), BucketError> {
        let mut len = 0;

        // first we decode header
        if let BucketDecoder::WaitingForHeader = self {
            if buf.len() < BucketHead::SIZE {
                return Ok((None, 0));
            }
            let header = BucketHead::from_bytes(&mut buf)?;
            len += BucketHead::SIZE;
            *self = BucketDecoder::WaitingForBody(header);
        };

        // next we check we have enough bytes to fill the body
        if let &mut Self::WaitingForBody(head) = self {
            if buf.len() < head.size as usize {
                return Ok((None, len));
            }
            *self = BucketDecoder::WaitingForHeader;
            Ok((
                Some(Bucket {
                    header: head,
                    body: buf.to_vec(),
                }),
                len + head.size as usize,
            ))
        } else {
            unreachable!()
        }
    }
}

pub struct BucketStream {
    decoder: BucketDecoder,
    buffer: Vec<u8>,
}

impl Default for BucketStream {
    fn default() -> Self {
        BucketStream {
            decoder: BucketDecoder::WaitingForHeader,
            buffer: Vec::new(),
        }
    }
}

impl BucketStream {
    
    pub fn received_bytes(&mut self, buf: &[u8]) {
        self.buffer.extend(buf);
    }

    pub fn decode_next_bucket(&mut self) -> Result<Option<Bucket>, BucketError> {
        let (bucket, len) = self.decoder.try_decode_bucket(&self.buffer)?;
        self.buffer.drain(..len);
        Ok(bucket)
    }
}
