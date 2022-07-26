//! Bytes-streaming testing sink.

use std::{
    convert::Infallible,
    error,
    io::Read,
    ops::Deref,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use bytes::Buf;
use futures::{FutureExt, Sink, SinkExt};

/// A sink for unit testing.
///
/// All data sent to it will be written to a buffer immediately that can be read during
/// operation. It is guarded by a lock so that only complete writes are visible.
///
/// Additionally, a `Plug` can be inserted into the sink. While a plug is plugged in, no data
/// can flow into the sink. In a similar manner, the sink can be clogged - while it is possible
/// to start sending new data, it will not report being done until the clog is cleared.
///
/// ```text
///   Item ->     (plugged?)             [             ...  ] -> (clogged?) -> done flushing
///    ^ Input     ^ Plug (blocks input)   ^ Buffer contents      ^ Clog, prevents flush
/// ```
///
/// This can be used to simulate a sink on a busy or slow TCP connection, for example.
#[derive(Default, Debug)]
pub struct TestingSink {
    /// The state of the plug.
    obstruction: Mutex<SinkObstruction>,
    /// Buffer storing all the data.
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl TestingSink {
    /// Creates a new testing sink.
    ///
    /// The sink will initially be unplugged.
    pub fn new() -> Self {
        TestingSink::default()
    }

    /// Inserts or removes the plug from the sink.
    pub fn set_plugged(&self, plugged: bool) {
        let mut guard = self.obstruction.lock().expect("obstruction mutex poisoned");
        guard.plugged = plugged;

        // Notify any waiting tasks that there may be progress to be made.
        if !plugged {
            if let Some(ref waker) = guard.waker {
                waker.wake_by_ref()
            }
        }
    }

    /// Inserts or removes the clog from the sink.
    pub fn set_clogged(&self, clogged: bool) {
        let mut guard = self.obstruction.lock().expect("obstruction mutex poisoned");
        guard.clogged = clogged;

        // Notify any waiting tasks that there may be progress to be made.
        if !clogged {
            if let Some(ref waker) = guard.waker {
                waker.wake_by_ref()
            }
        }
    }

    /// Determine whether the sink is plugged.
    ///
    /// Will update the local waker reference.
    pub fn is_plugged(&self, cx: &mut Context<'_>) -> bool {
        let mut guard = self.obstruction.lock().expect("obstruction mutex poisoned");

        guard.waker = Some(cx.waker().clone());
        guard.plugged
    }

    /// Determine whether the sink is clogged.
    ///
    /// Will update the local waker reference.
    pub fn is_clogged(&self, cx: &mut Context<'_>) -> bool {
        let mut guard = self.obstruction.lock().expect("obstruction mutex poisoned");

        guard.waker = Some(cx.waker().clone());
        guard.clogged
    }

    /// Returns a copy of the contents.
    pub fn get_contents(&self) -> Vec<u8> {
        Vec::clone(
            &self
                .buffer
                .lock()
                .expect("could not lock test sink for copying"),
        )
    }

    /// Creates a new reference to the testing sink that also implements `Sink`.
    ///
    /// Internally, the reference has a static lifetime through `Arc` and can thus be passed
    /// on independently.
    pub fn into_ref(self: Arc<Self>) -> TestingSinkRef {
        TestingSinkRef(self)
    }

    /// Helper function for sink implementations, calling `poll_ready`.
    fn sink_poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        if self.is_plugged(cx) {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    /// Helper function for sink implementations, calling `start_end`.
    fn sink_start_send<F: Buf>(&self, item: F) -> Result<(), Infallible> {
        let mut guard = self.buffer.lock().expect("could not lock buffer");

        item.reader()
            .read_to_end(&mut guard)
            .expect("writing to vec should never fail");

        Ok(())
    }

    /// Helper function for sink implementations, calling `sink_poll_flush`.
    fn sink_poll_flush(&self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        // We're always done storing the data, but we pretend we need to do more if clogged.
        if self.is_clogged(cx) {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    /// Helper function for sink implementations, calling `sink_poll_close`.
    fn sink_poll_close(&self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        // Nothing to close, so this is essentially the same as flushing.
        self.sink_poll_flush(cx)
    }
}

/// A plug/clog inserted into the sink.
#[derive(Debug, Default)]
pub struct SinkObstruction {
    /// Whether or not the sink is plugged.
    plugged: bool,
    /// Whether or not the sink is clogged.
    clogged: bool,
    /// The waker of the last task to access the plug. Will be called when removing.
    waker: Option<Waker>,
}

/// Helper macro to implement forwarding the `Sink` traits methods to fixed methods on
/// `TestingSink`.
macro_rules! sink_impl_fwd {
    ($ty:ty) => {
        impl<F: Buf> Sink<F> for $ty {
            type Error = Box<dyn error::Error + Send + Sync>;

            fn poll_ready(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Result<(), Self::Error>> {
                self.sink_poll_ready(cx).map_err(Into::into)
            }

            fn start_send(self: Pin<&mut Self>, item: F) -> Result<(), Self::Error> {
                self.sink_start_send(item).map_err(Into::into)
            }

            fn poll_flush(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Result<(), Self::Error>> {
                self.sink_poll_flush(cx).map_err(Into::into)
            }

            fn poll_close(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Result<(), Self::Error>> {
                self.sink_poll_close(cx).map_err(Into::into)
            }
        }
    };
}

/// A reference to a testing sink that implements `Sink`.
#[derive(Debug)]
pub struct TestingSinkRef(Arc<TestingSink>);

impl Deref for TestingSinkRef {
    type Target = TestingSink;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

sink_impl_fwd!(TestingSink);
sink_impl_fwd!(&TestingSink);
sink_impl_fwd!(TestingSinkRef);

#[test]
fn simple_lifecycle() {
    let mut sink = TestingSink::new();
    assert!(sink.send(&b"one"[..]).now_or_never().is_some());
    assert!(sink.send(&b"two"[..]).now_or_never().is_some());
    assert!(sink.send(&b"three"[..]).now_or_never().is_some());

    assert_eq!(sink.get_contents(), b"onetwothree");
}

#[test]
fn plug_blocks_sink() {
    let sink = TestingSink::new();
    let mut sink_handle = &sink;

    sink.set_plugged(true);

    // The sink is plugged, so sending should fail. We also drop the future, causing the value
    // to be discarded.
    assert!(sink_handle.send(&b"dummy"[..]).now_or_never().is_none());
    assert!(sink.get_contents().is_empty());

    // Now stuff more data into the sink.
    let second_send = sink_handle.send(&b"second"[..]);
    sink.set_plugged(false);
    assert!(second_send.now_or_never().is_some());
    assert!(sink_handle.send(&b"third"[..]).now_or_never().is_some());
    assert_eq!(sink.get_contents(), b"secondthird");
}

#[test]
fn clog_blocks_sink_completion() {
    let sink = TestingSink::new();
    let mut sink_handle = &sink;

    sink.set_clogged(true);

    // The sink is clogged, so sending should fail to complete, but it is written.
    assert!(sink_handle.send(&b"first"[..]).now_or_never().is_none());
    assert_eq!(sink.get_contents(), b"first");

    // Now stuff more data into the sink.
    let second_send = sink_handle.send(&b"second"[..]);
    sink.set_clogged(false);
    assert!(second_send.now_or_never().is_some());
    assert!(sink_handle.send(&b"third"[..]).now_or_never().is_some());
    assert_eq!(sink.get_contents(), b"firstsecondthird");
}

/// Verifies that when a sink is clogged but later unclogged, any waiters on it are woken up.
#[tokio::test]
async fn waiting_tasks_can_progress_upon_unplugging_the_sink() {
    let sink = Arc::new(TestingSink::new());

    sink.set_plugged(true);

    let sink_alt = sink.clone();

    let join_handle = tokio::spawn(async move {
        sink_alt.as_ref().send(&b"sample"[..]).await.unwrap();
    });

    tokio::task::yield_now().await;
    sink.set_plugged(false);

    // This will block forever if the other task is not woken up. To verify, comment out the
    // `Waker::wake_by_ref` call in the sink implementation.
    join_handle.await.unwrap();
}
