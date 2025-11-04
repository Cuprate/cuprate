use cuprate_database::DbResult;

pub trait Database<ReadReq, ReadRes, WriteReq, WriteRes>: Send + 'static + Sync {
    fn handle_read(&self, req: ReadReq) -> DbResult<ReadRes>;
    fn handle_write(&mut self, req: WriteReq) -> DbResult<WriteRes>;
}
