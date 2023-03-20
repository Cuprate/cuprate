use bytes::Bytes;

use crate::bucket::{header::Flags, Bucket, BucketHead};

use super::ConnectionError;

#[derive(Debug, Default)]
pub struct BucketBuilder {
    have_to_return_data: Option<bool>,
    flags: Option<Flags>,
    command: Option<u32>,
    return_code: Option<i32>,
    body: Option<Bytes>,
}

impl BucketBuilder {
    pub fn set_have_to_return(&mut self, have_to_return_date: bool) {
        self.have_to_return_data = Some(have_to_return_date);
    }
    pub fn set_command(&mut self, command: u32) {
        self.command = Some(command);
    }
    pub fn set_return_code(&mut self, return_code: i32) {
        self.return_code = Some(return_code);
    }
    pub fn set_flags(&mut self, flags: Flags) {
        self.flags = Some(flags);
    }
    pub fn set_body(&mut self, body: Bytes) {
        self.body = Some(body);
    }
}

impl TryInto<Bucket> for BucketBuilder {
    type Error = ConnectionError;
    fn try_into(self) -> Result<Bucket, Self::Error> {
        let body = self
            .body
            .ok_or(ConnectionError::FailedToConstructBucket("missing body"))?;
        let header = BucketHead::build(
            body.len() as u64,
            self.have_to_return_data
                .ok_or(ConnectionError::FailedToConstructBucket("missing have_to_return"))?,
            self.command
                .ok_or(ConnectionError::FailedToConstructBucket("missing command"))?,
            self.flags
                .ok_or(ConnectionError::FailedToConstructBucket("missing flags"))?,
            self.return_code
                .ok_or(ConnectionError::FailedToConstructBucket("missing return_code"))?,
        );
        Ok(Bucket { header, body })
    }
}
