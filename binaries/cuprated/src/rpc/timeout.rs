//! IO Timeout Wrapper
//!
//! This module implements wrapper around [`AsyncRead`]/[`AsyncWrite`] types to return `TimedOut` error if
//! they haven't been able to complete operation after a period of time.
//!
//! This is used as a denial of service mitigation mechanism against keep-alive or one-way spamming connections.
//!
//! Internally these wrappers abstract the `Duration` field to welcome shared data structures that can be used to
//! adapt the timeout period on the fly.
//!

use std::{
    future::Future,
    io::{Error, ErrorKind},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use hyper::rt::ReadBufCursor;
use pin_project_lite::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::{sleep_until, Instant, Sleep},
};

/// A pinned timeout state with a specified duration.
pub struct TimeoutState {
    timeout: Duration,
    refresh: bool,
    sleep: Pin<Box<Sleep>>,
}

impl TimeoutState
where
    Self: Unpin,
{
    /// Create a new [`TimeoutState`] with the given timeout type.
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            refresh: true,
            sleep: Box::pin(sleep_until(Instant::now())),
        }
    }

    /// Poll inner [`Sleep`] for completion. Update its deadline on first use and return
    /// `Poll::Ready(Error::from(ErrorKind::TimedOut))` on completion, `Poll::Pending` otherwise
    pub fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Error> {
        let mut proj = self;

        // On first poll after refresh activate couldown.
        if proj.refresh {
            proj.refresh = false;
            let timeout = proj.timeout;
            proj.sleep.as_mut().reset(Instant::now() + timeout);
        }

        proj.sleep
            .as_mut()
            .poll(cx)
            .map(|()| Error::from(ErrorKind::TimedOut))
    }
}

// Helper macros for reducing redundancy. This main logic is present in every poll.
macro_rules! poll_or_timeout {
    ($self:ident::$io:ident..$timeout:ident => $poll:ident, $cx:ident, $($arg:expr),*) => {{
        let proj = $self .project();

        match proj.$io.$poll($cx, $($arg),*) {
            Poll::Pending => proj.$timeout.as_mut().poll($cx).map(Err),
            Poll::Ready(r) => {
                proj.$timeout.refresh = true;
                Poll::Ready(r)
            }
        }
    }};
    ($self:ident::$io:ident..$timeout:ident => $poll:ident, $cx:ident) => {{
        let proj = $self .project();

        match proj.$io.$poll($cx) {
            Poll::Pending => proj.$timeout.as_mut().poll($cx).map(Err),
            Poll::Ready(r) => {
                proj.$timeout.refresh = true;
                Poll::Ready(r)
            }
        }
    }};
}

pin_project! {
    /// A timeout wrapper around an [`AsyncRead`] + [`AsyncWrite`] implemented type.
    ///
    /// Returns a `TimedOut` error if `poll_read` have been returning
    /// `Poll::Pending` for the timeout duration.
    pub struct StreamTimeout<S> {
        #[pin]
        stream: S,
        write_timeout: Pin<Box<TimeoutState>>,
        read_timeout: Pin<Box<TimeoutState>>
    }
}

impl<S: AsyncWrite + AsyncRead> StreamTimeout<S> {
    /// Create a new [`StreamTimeout`] from a stream and two [`ExtractDuration`] enabled type.
    pub fn new(stream: S, write_timeout: Duration, read_timeout: Duration) -> Self {
        Self {
            stream,
            write_timeout: Box::pin(TimeoutState::new(write_timeout)),
            read_timeout: Box::pin(TimeoutState::new(read_timeout)),
        }
    }
}

impl<S: AsyncWrite + AsyncRead> AsyncWrite for StreamTimeout<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        poll_or_timeout!(self::stream..write_timeout => poll_write, cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, Error>> {
        poll_or_timeout!(self::stream..write_timeout => poll_write_vectored, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        poll_or_timeout!(self::stream..write_timeout => poll_flush, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        poll_or_timeout!(self::stream..write_timeout => poll_shutdown, cx)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }
}

impl<S: AsyncRead + AsyncWrite> AsyncRead for StreamTimeout<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        poll_or_timeout!(self::stream..read_timeout => poll_read, cx, buf)
    }
}

#[cfg(test)]
mod test {
    use std::{
        future::Future,
        io::ErrorKind,
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        select,
        task::JoinSet,
        time::{sleep, timeout},
    };

    use crate::rpc::timeout::StreamTimeout;

    #[cfg(target_os = "macos")]
    const TEST_TIMEOUT: Duration = Duration::from_secs(2);
    #[cfg(not(target_os = "macos"))]
    const TEST_TIMEOUT: Duration = Duration::from_secs(10);

    fn within_current_thread_runtime(future: impl Future) {
        // Start tokio runtime
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(future);
    }

    // Common setup used between TCP tests.
    async fn spawn_tcp_setup<C, L, R1, R2>(port: u16, client_test: C, listener_test: L)
    where
        R1: Future<Output = ()> + Send + 'static,
        R2: Future<Output = ()> + Send + 'static,
        C: Fn(TcpStream) -> R1 + Send + 'static,
        L: Fn(TcpStream) -> R2 + Send + 'static,
    {
        let socketaddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);

        let listener = TcpListener::bind(socketaddr)
            .await
            .expect("Unable to bind TCP Listener");

        let mut set = JoinSet::new();

        // Spawn Listener
        set.spawn(async move {
            let connection = listener
                .accept()
                .await
                .expect("Unable to accept incoming connection");

            listener_test(connection.0).await;
        });

        // Spawn client
        set.spawn(async move {
            let Ok(stream) = timeout(TEST_TIMEOUT, TcpStream::connect(socketaddr))
                .await
                .expect("Unable to connect listener")
            else {
                panic!("No connection has been made to the listener!");
            };

            client_test(stream).await;
        });

        set.join_all().await;
    }

    #[test]
    fn tcp_stream_read_timeout_err() {
        within_current_thread_runtime(spawn_tcp_setup(
            60035,
            async |_stream: TcpStream| {
                sleep(TEST_TIMEOUT + Duration::from_secs(1)).await;
            },
            async |stream: TcpStream| {
                // Wrap stream into StreamTimeout
                let mut stream = StreamTimeout::new(stream, TEST_TIMEOUT, TEST_TIMEOUT);

                // Try to read
                let mut buf = vec![0_u8; 1024]; // 1KiB
                select! {
                    r = stream.read_buf(&mut buf) => {
                        if let Err(err) = r {
                            assert_eq!(err.kind(), ErrorKind::TimedOut);
                        } else {
                            panic!("The buffer has been successfully filled. This test needs to updated.")
                        }
                    }
                    () = sleep(TEST_TIMEOUT + Duration::from_secs(1)) => {
                        panic!("No error has been returned after <timeout duration>+1 seconds.")
                    }
                }
            },
        ));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn tcp_stream_write_timeout_err() {
        within_current_thread_runtime(spawn_tcp_setup(
            60036,
            async |_stream: TcpStream| {
                sleep(TEST_TIMEOUT + Duration::from_secs(1)).await;
            },
            async |stream: TcpStream| {
                // Wrap stream into StreamTimeout
                let mut stream = StreamTimeout::new(stream, TEST_TIMEOUT, TEST_TIMEOUT);

                // Try to write
                let buf = vec![1_u8; 64 * 1024_usize.pow(2)]; // 16MiB
                select! {
                    r = stream.write_all(&buf) => {
                        if let Err(err) = r {
                            assert_eq!(err.kind(), ErrorKind::TimedOut);
                        } else {
                            panic!("Buffer have been successfully flushed. This test needs to updated.")
                        }
                    }
                    () = sleep(TEST_TIMEOUT * 2) => {
                        panic!("No error has been returned after <timeout duration>+1 seconds.")
                    }
                }
            },
        ));
    }
}
