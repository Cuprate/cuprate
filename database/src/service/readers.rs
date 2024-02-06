//! TODO

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Readers
/// TODO
///
/// A struct representing a thread-pool of database
/// readers that handle `tower::Service` requests.
pub(crate) struct Readers;

//---------------------------------------------------------------------------------------------------- Readers Impl
impl Readers {
    /// TODO
    pub(super) fn init() -> Self {
        /* create channels, initialize data, etc */

        let this = Self;

        std::thread::spawn(move || {
            Self::main(this);
        });

        // TODO: return some handle, not `Readers` itself
        Self
    }

    /// The `Readers`'s main function.
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
