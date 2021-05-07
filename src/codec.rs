use std::ops::{Deref, DerefMut};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LinesCodec};

pub use tokio_util::codec::LinesCodecError as ChatCodecError;

/// A wrapper around [`LinesCodec`] that enforces a `510` character limit for every line.
///
/// This is helpful to avoid DoS type attacks from users.
pub struct ChatCodec<S>(Framed<S, LinesCodec>);

impl<S: AsyncRead + AsyncWrite> ChatCodec<S> {
    const LENGTH_LIMIT: usize = 20_000;

    /// Creates a new instace of [`ChatCodec`].
    pub fn new(stream: S) -> Self {
        Self(Framed::new(
            stream,
            LinesCodec::new_with_max_length(Self::LENGTH_LIMIT),
        ))
    }
}

impl<S> Deref for ChatCodec<S> {
    type Target = Framed<S, LinesCodec>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for ChatCodec<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
