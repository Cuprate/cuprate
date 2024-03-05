//! Abstracted database table value reference; `trait ValueGuard`.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Env
/// TODO
///
/// TODO: explain this exists because `redb` doesn't return `&T::Value`,
/// it returns a guard that has `value() -> &T::Value`, which is relatively
/// an expensive operation, so to prevent that, this trait exists to make
/// easy/implicit derefs hard to do.

pub trait ValueGuard<'a, Value: ?Sized> {
    /// TODO
    fn value(&'a self) -> &'a Value;
}

// Already created references don't need to do anything.
// Always inline such that the compiler can optimize out any caller.
impl<'a, T> ValueGuard<'a, T> for &'a T {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn value(&'a self) -> &'a T {
        self
    }
}
