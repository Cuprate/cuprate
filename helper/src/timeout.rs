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

use pin_project_lite::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::{sleep_until, Instant, Sleep},
};

/// Helper trait that add [`ExtractDuration::extract_duration`] function that can compute a timeout Duration
/// from a reference. This is sensibly the same as `D where Duration: From<&D>` but with
/// no specific lifetime requirement.
///
/// This trait is implemented for [`Duration`].
pub trait ExtractDuration: Clone + Unpin {
    fn extract_duration(&self) -> Duration;
}

impl ExtractDuration for Duration {
    fn extract_duration(&self) -> Duration {
        *self
    }
}

/// A timeout state with a specified duration.
///
/// `D` implements [`ExtractDuration`] trait which
/// permit custom types to compute a Timeout duration.
///
/// This can be useful for shared data structures that
/// modify the timeout on the fly.
pub struct TimeoutState<D> {
    timeout: D,
    refresh: bool,
    sleep: Pin<Box<Sleep>>,
}

impl<D: ExtractDuration> TimeoutState<D>
where
    Self: Unpin,
{
    /// Create a new [`TimeoutState`] with the given timeout type.
    pub fn new(timeout: D) -> Self {
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
            let timeout = proj.timeout.extract_duration();
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
    /// A timeout wrapper around an [`AsyncWrite`] implemented type.
    ///
    /// Returns a `TimedOut` error if any poll operation have been returning
    /// `Poll::Pending` for the timeout duration.
    pub struct WriteTimeout<D, W: AsyncWrite> {
        #[pin]
        writer: W,
        timeout: Pin<Box<TimeoutState<D>>>,
    }
}

impl<D: ExtractDuration, W: AsyncWrite> WriteTimeout<D, W> {
    /// Create a new [`WriteTimeout`] from a writer and an [`ExtractDuration`] enabled type.
    pub fn new(writer: W, timeout: D) -> Self {
        Self {
            writer,
            timeout: Box::pin(TimeoutState::new(timeout)),
        }
    }
}

impl<D: ExtractDuration, W: AsyncWrite> AsyncWrite for WriteTimeout<D, W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        poll_or_timeout!(self::writer..timeout => poll_write, cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, Error>> {
        poll_or_timeout!(self::writer..timeout => poll_write_vectored, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        poll_or_timeout!(self::writer..timeout => poll_flush, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        poll_or_timeout!(self::writer..timeout => poll_shutdown, cx)
    }

    fn is_write_vectored(&self) -> bool {
        self.writer.is_write_vectored()
    }
}

pin_project! {
    /// A timeout wrapper around an [`AsyncRead`] implemented type.
    ///
    /// Returns a `TimedOut` error if `poll_read` have been returning
    /// `Poll::Pending` for the timeout duration.
    pub struct ReadTimeout<D, R: AsyncRead> {
        #[pin]
        reader: R,
        timeout: Pin<Box<TimeoutState<D>>>,
    }
}

impl<D: ExtractDuration, R: AsyncRead> ReadTimeout<D, R> {
    /// Create a new [`ReadTimeout`] from a reader and an [`ExtractDuration`] enabled type.
    pub fn new(reader: R, timeout: D) -> Self {
        Self {
            reader,
            timeout: Box::pin(TimeoutState::new(timeout)),
        }
    }
}

impl<D: ExtractDuration, R: AsyncRead> AsyncRead for ReadTimeout<D, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        poll_or_timeout!(self::reader..timeout => poll_read, cx, buf)
    }
}

pin_project! {
    /// A timeout wrapper around an [`AsyncRead`] + [`AsyncWrite`] implemented type.
    ///
    /// Returns a `TimedOut` error if `poll_read` have been returning
    /// `Poll::Pending` for the timeout duration.
    pub struct StreamTimeout<DR, DW, S> {
        #[pin]
        stream: S,
        write_timeout: Pin<Box<TimeoutState<DW>>>,
        read_timeout: Pin<Box<TimeoutState<DR>>>
    }
}

impl<DR: ExtractDuration, DW: ExtractDuration, S: AsyncWrite + AsyncRead> StreamTimeout<DR, DW, S> {
    /// Create a new [`StreamTimeout`] from a stream and two [`ExtractDuration`] enabled type.
    pub fn new(stream: S, write_timeout: DW, read_timeout: DR) -> Self {
        Self {
            stream,
            write_timeout: Box::pin(TimeoutState::new(write_timeout)),
            read_timeout: Box::pin(TimeoutState::new(read_timeout)),
        }
    }
}

impl<DR: ExtractDuration, DW: ExtractDuration, S: AsyncWrite + AsyncRead> AsyncWrite
    for StreamTimeout<DR, DW, S>
{
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

impl<DR: ExtractDuration, DW: ExtractDuration, S: AsyncRead + AsyncWrite> AsyncRead
    for StreamTimeout<DR, DW, S>
{
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

    use crate::timeout::{ReadTimeout, StreamTimeout, WriteTimeout};

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
    fn tcp_write_timeout_err() {
        within_current_thread_runtime(spawn_tcp_setup(
            60031,
            async |_stream: TcpStream| {
                sleep(TEST_TIMEOUT + Duration::from_secs(1)).await;
            },
            async |stream: TcpStream| {
                let (_reader, writer) = stream.into_split();

                // Wrap writer half into a WriteTimeout.
                let mut writer = WriteTimeout::new(writer, TEST_TIMEOUT);

                // Write.
                let buf = vec![1_u8; 64 * 1024_usize.pow(2)]; // 64MiB
                select! {
                    r = writer.write_all(&buf) => {
                        if let Err(err) = r {
                            assert_eq!(err.kind(), ErrorKind::TimedOut);
                        } else {
                            panic!("Buffer have been successfully flushed. This test needs to updated.")
                        }
                    }
                    () = sleep(TEST_TIMEOUT + Duration::from_secs(1)) => {
                        panic!("No error has been returned after <timeout duration>+1 seconds.")
                    }
                }
            },
        ));
    }

    #[test]
    fn tcp_write_timeout_ok() {
        within_current_thread_runtime(spawn_tcp_setup(
            60032,
            async |stream: TcpStream| {
                let (mut reader, _writer) = stream.into_split();

                sleep(TEST_TIMEOUT / 2).await;

                loop {
                    let mut buf = vec![0_u8; 64 * 1024]; // 64KiB
                    if reader.read_exact(&mut buf).await.is_err() {
                        break;
                    }
                }
            },
            async |stream: TcpStream| {
                let (_reader, writer) = stream.into_split();

                // Wrap writer half into a WriteTimeout.
                let mut writer = WriteTimeout::new(writer, TEST_TIMEOUT);

                // Write.
                let buf = vec![1_u8; 1024_usize.pow(2)]; // 1MiB
                select! {
                    r = writer.write_all(&buf) => {
                        assert!(r.is_ok());
                    }
                    () = sleep(TEST_TIMEOUT + Duration::from_secs(1)) => {
                        panic!("No error has been returned after <timeout duration>+1 seconds.")
                    }
                }
            },
        ));
    }

    #[test]
    fn tcp_read_timeout_err() {
        within_current_thread_runtime(spawn_tcp_setup(
            60033,
            async |_stream: TcpStream| {
                sleep(TEST_TIMEOUT + Duration::from_secs(1)).await;
            },
            async |stream: TcpStream| {
                let (reader, _writer) = stream.into_split();

                // Wrap reader half into a ReadTimeout.
                let mut reader = ReadTimeout::new(reader, TEST_TIMEOUT);

                // Try to read.
                let mut buf = vec![0_u8; 1024]; // 1KiB
                select! {
                    r = reader.read_buf(&mut buf) => {
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

    #[test]
    fn tcp_read_timeout_ok() {
        within_current_thread_runtime(spawn_tcp_setup(
            60034,
            async |stream: TcpStream| {
                let (_reader, mut writer) = stream.into_split();

                sleep(TEST_TIMEOUT / 2).await;

                let _ = writer
                    .write(&[1])
                    .await
                    .expect("Unable to write into TCP stream");
            },
            async |stream: TcpStream| {
                let (reader, _writer) = stream.into_split();

                // Wrap reader half into a ReadTimeout.
                let mut reader = ReadTimeout::new(reader, TEST_TIMEOUT);

                // Try to read
                let mut buf = vec![0_u8; 1024]; // 1KiB
                select! {
                    r = reader.read_buf(&mut buf) => {
                        assert!(r.is_ok());
                    }
                    () = sleep(TEST_TIMEOUT + Duration::from_secs(1)) => {
                        panic!("No error has been returned after <timeout duration>+1 seconds.")
                    }
                }
            },
        ));
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
