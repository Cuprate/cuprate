//! General macros.

//---------------------------------------------------------------------------------------------------- Logging
// The below macros/types/functions allow for conditional
// logging with `tracing` depending on if the feature flag
// is enabled or not.
//
// These macros (e.g. [`info2`]) are used instead of the tracing macro directly.
// #[tracing::instrument] will still need to be handled by us by using `#[cfg_attr]`.

/// Dummy structure with methods that copy the ones on [`tracing::Span`].
///
/// This is used when "tracing" is disabled such that span functions:
/// ```rust,ignore
/// let span: DummySpan = info_span2!("span");
/// let _guard = span.enter();
/// ```
/// will still work as this struct will just be returned, then
/// [`DummySpan::enter`] will be called instead, which is just a dummy method.
pub(crate) struct DummySpan;

impl DummySpan {
    /// Emulates [`tracing::Span::enter`].
    #[inline]
    pub(crate) const fn enter(&self) -> &Self {
        self
    }
}

/// Drop impl is needed to satisfy clippy.
impl Drop for DummySpan {
    #[inline]
    fn drop(&mut self) {}
}

/// [`tracing::warn_span`].
macro_rules! warn_span2 {
    ($($token:tt)*) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::warn_span!($($token)*)
            } else {
                $crate::macros::DummySpan
            }
        }
    }};
}
pub(crate) use warn_span2;

/// [`tracing::warn`].
macro_rules! warn2 {
    ($($token:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::warn!($($token)*)
    };
}
pub(crate) use warn2;

/// [`tracing::trace_span`].
macro_rules! trace_span2 {
    ($($token:tt)*) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::trace_span!($($token)*)
            } else {
                $crate::macros::DummySpan
            }
        }
    }};
}
pub(crate) use trace_span2;

/// [`tracing::trace`].
macro_rules! trace2 {
    ($($token:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::trace!($($token)*)
    };
}
pub(crate) use trace2;

/// [`tracing::info_span`].
macro_rules! info_span2 {
    ($($token:tt)*) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::info_span!($($token)*)
            } else {
                $crate::macros::DummySpan
            }
        }
    }};
}
pub(crate) use info_span2;

/// [`tracing::info`].
macro_rules! info2 {
    ($($token:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::info!($($token)*)
    };
}
pub(crate) use info2;

/// [`tracing::error_span`].
macro_rules! error_span2 {
    ($($token:tt)*) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::error_span!($($token)*)
            } else {
                $crate::macros::DummySpan
            }
        }
    }};
}
pub(crate) use error_span2;

/// [`tracing::error`].
macro_rules! error2 {
    ($($token:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::error!($($token)*)
    };
}
pub(crate) use error2;

/// [`tracing::debug_span`].
macro_rules! debug_span2 {
    ($($token:tt)*) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::debug_span!($($token)*)
            } else {
                $crate::macros::DummySpan
            }
        }
    }};
}
pub(crate) use debug_span2;

/// [`tracing::debug`].
macro_rules! debug2 {
    ($($token:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::debug!($($token)*)
    };
}
pub(crate) use debug2;

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
