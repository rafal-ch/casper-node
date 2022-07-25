//! Frame reading and writing
//!
//! Frame readers and writers are responsible for writing a [`Bytes`] frame to an [`AsyncWrite`]
//! writer, or reading them from [`AsyncRead`] reader. While writing works for any value that
//! implements the [`bytes::Buf`] trait, decoding requires an implementation of the [`FrameDecoder`]
//! trait.

use std::{
    error, io,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, BytesMut};
use futures::{ready, AsyncRead, AsyncWrite, Sink, Stream};

use crate::{
    codec::{DecodeResult, FrameDecoder, Transcoder},
    try_ready,
};

/// Frame decoder for an underlying reader.
///
/// Uses the given [`FrameDecoder`] `D` to read frames from the underlying IO.
///
/// # Cancellation safety
///
/// The [`Stream`] implementation on [`FrameDecoder`] is cancellation safe, as it buffers data
/// inside the reader, not the `next` future.
pub struct FrameReader<D, R> {
    /// Decoder used to decode frames.
    decoder: D,
    /// Underlying async bytestream being read.
    stream: R,
    /// Internal buffer for incomplete frames.
    buffer: BytesMut,
    /// Maximum number of bytes to read.
    max_read_buffer_increment: usize,
}

/// Writer for frames.
///
/// Simply writes any given [`Buf`]-implementing frame to the underlying writer.
///
/// # Cancellation safety
///
/// The [`Sink`] methods on [`FrameWriter`] are cancellation safe. Only a single item is buffered
/// inside the writer itself.
pub struct FrameWriter<F, E: Transcoder<F>, W> {
    /// The encoder used to encode outgoing frames.
    encoder: E,
    /// Underlying async bytestream being written.
    stream: W,
    /// The frame in process of being sent.
    current_frame: Option<E::Output>,
}

impl<D, R> FrameReader<D, R> {
    /// Creates a new frame reader on a given stream with the given read buffer increment.
    pub fn new(decoder: D, stream: R, max_read_buffer_increment: usize) -> Self {
        Self {
            decoder,
            stream,
            buffer: BytesMut::new(),
            max_read_buffer_increment,
        }
    }

    /// Deconstructs a frame reader into decoder, reader and buffer.
    pub fn into_parts(self) -> (D, R, BytesMut) {
        (self.decoder, self.stream, self.buffer)
    }
}

impl<D, R> Stream for FrameReader<D, R>
where
    D: FrameDecoder + Unpin,
    R: AsyncRead + Unpin,
{
    type Item = io::Result<<D as FrameDecoder>::Output>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let FrameReader {
            ref mut stream,
            ref mut decoder,
            ref mut buffer,
            max_read_buffer_increment,
        } = self.get_mut();
        loop {
            let next_read = match decoder.decode_frame(buffer) {
                DecodeResult::Item(frame) => return Poll::Ready(Some(Ok(frame))),
                DecodeResult::Incomplete => *max_read_buffer_increment,
                DecodeResult::Remaining(remaining) => remaining.min(*max_read_buffer_increment),
                DecodeResult::Failed(error) => {
                    return Poll::Ready(Some(Err(io::Error::new(io::ErrorKind::Other, error))))
                }
            };

            let start = buffer.len();
            let end = start + next_read;
            buffer.resize(end, 0x00);

            match Pin::new(&mut *stream).poll_read(cx, &mut buffer[start..end]) {
                Poll::Ready(Ok(bytes_read)) => {
                    buffer.truncate(start + bytes_read);
                    if bytes_read == 0 {
                        return Poll::Ready(None);
                    }
                }
                Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<F, E, W> FrameWriter<F, E, W>
where
    E: Transcoder<F>,
    <E as Transcoder<F>>::Output: Buf,
{
    /// Creates a new frame writer with the given encoder.
    pub fn new(encoder: E, stream: W) -> Self {
        Self {
            encoder,
            stream,
            current_frame: None,
        }
    }

    pub fn finish_sending(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>
    where
        Self: Sink<F> + Unpin,
        F: Buf,
        W: AsyncWrite + Unpin,
    {
        loop {
            match self.current_frame {
                // No more frame to send, we're ready.
                None => return Poll::Ready(Ok(())),

                Some(ref mut current_frame) => {
                    // TODO: Implement support for `poll_write_vectored`.

                    let stream_pin = Pin::new(&mut self.stream);
                    match stream_pin.poll_write(cx, current_frame.chunk()) {
                        Poll::Ready(Ok(bytes_written)) => {
                            current_frame.advance(bytes_written);

                            // If we're done, clear the current frame and return.
                            if !current_frame.has_remaining() {
                                self.current_frame.take();
                                return Poll::Ready(Ok(()));
                            }

                            // Otherwise, repeat the loop.
                        }
                        // Error occured, we have to abort.
                        Poll::Ready(Err(error)) => {
                            return Poll::Ready(Err(error));
                        }
                        // The underlying output stream is blocked, no progress can be made.
                        Poll::Pending => return Poll::Pending,
                    }
                }
            }
        }
    }
}

impl<F, E, W> Sink<F> for FrameWriter<F, E, W>
where
    Self: Unpin,
    E: Transcoder<F>,
    <E as Transcoder<F>>::Output: Buf,
    F: Buf,
    W: AsyncWrite + Unpin,
    <E as Transcoder<F>>::Error: std::error::Error,
{
    type Error = Box<dyn error::Error + Send + Sync>;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let self_mut = self.get_mut();

        try_ready!(ready!(self_mut.finish_sending(cx)));

        // Even though there may be outstanding writes on the underlying stream, our item buffer is
        // empty, so we are ready to accept the next item.
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: F) -> Result<(), Self::Error> {
        let wrapped_frame = self.encoder.transcode(item);
        let wrapped_frame = wrapped_frame?;
        self.current_frame = Some(wrapped_frame);

        // We could eaglerly poll and send to the underlying writer here, but for ease of
        // implementation we don't.

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let self_mut = self.get_mut();

        // We need to make sure all data is buffered to the underlying stream first.
        try_ready!(ready!(self_mut.finish_sending(cx)));

        // Finally it makes sense to flush.
        let wpin = Pin::new(&mut self_mut.stream);
        wpin.poll_flush(cx).map_err(Into::into)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let self_mut = self.get_mut();

        // Finish buffering our outstanding item.
        try_ready!(ready!(self_mut.finish_sending(cx)));

        let wpin = Pin::new(&mut self_mut.stream);
        wpin.poll_close(cx).map_err(Into::into)
    }
}
