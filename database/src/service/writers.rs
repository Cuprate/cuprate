//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Writers
/// TODO
///
/// A struct representing the thread-pool (maybe just 2 threads).
/// that handles `tower::Service` write requests.
///
/// The reason this isn't a single thread is to allow a little
/// more work to be done during the leeway in-between DB operations, e.g:
///
/// ```text
///  Request1
///     |                                      Request2
///     v                                         |
///  Writer1                                      v
///     |                                      Writer2
/// doing work                                    |
///     |                               waiting on other writer
/// sending response  --|                         |
///     |               |- leeway            doing work
///     v             --|                         |
///    done                                       |
///                                        sending response
///                                               |
///                                               v
///                                              done
/// ```
///
/// During `leeway`, `Writer1` is:
/// - busy preparing the message
/// - creating the response channel
/// - sending it back to the channel
/// - ...etc
///
/// During this time, it would be wasteful if `Request2`'s was
/// just being waited on, so instead, `Writer2` handles that
/// work while `Writer1` is in-between tasks.
///
/// This leeway is pretty tiny (doesn't take long to allocate a channel
/// and send a response) so there's only 2 Writers for now (needs testing).
///
/// The database backends themselves will hang on write transactions if
/// there are other existing ones, so we ourselves don't need locks.
pub(crate) struct Writers;

//---------------------------------------------------------------------------------------------------- Writers Impl
impl Writers {
    /// TODO
    pub(super) fn init() -> Self {
        /* create channels, initialize data, etc */

        let this = Self;

        std::thread::spawn(move || {
            Self::main(this);
        });

        // TODO: return some handle, not `Writers` itself
        Self
    }

    /// The `Writers`'s main function.
    fn main(mut self) {
        loop {
            // 1. Hang on request channel
            // 2. Map request to some database function
            // 3. Execute that function, get the result
            // 4. Return the result via channel
            self.request_to_db_function();
        }
    }

    /// Map [`Request`]'s to specific database functions.
    fn request_to_db_function(&mut self) {
        todo!();
    }
}

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
