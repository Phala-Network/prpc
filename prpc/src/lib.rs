#![cfg_attr(not(feature = "std"), no_std)]
#![allow(async_fn_in_trait)]

#[macro_use]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use derive_more::Display;
use prost::DecodeError;

pub use prost::Message;

pub mod serde_helpers;

pub mod server {
    use super::*;
    use alloc::string::ToString;
    use parity_scale_codec::Error as ScaleCodecErr;

    /// Error for server side RPC handlers. Finally, this error will be wrapped in a `ProtoError`.
    #[derive(Display, Debug)]
    pub enum Error {
        /// The requesting RPC method is not recognized
        NotFound,
        /// Failed to decode the request parameters
        DecodeError(DecodeError),
        /// Error for contract query
        BadRequest(String),
    }

    impl From<DecodeError> for Error {
        fn from(e: DecodeError) -> Self {
            Self::DecodeError(e)
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for Error {}

    #[cfg(not(feature = "std"))]
    impl From<Error> for anyhow::Error {
        fn from(error: Error) -> Self {
            Self::msg(error)
        }
    }

    impl From<anyhow::Error> for Error {
        fn from(error: anyhow::Error) -> Self {
            Self::BadRequest(error.to_string())
        }
    }

    impl From<ScaleCodecErr> for Error {
        fn from(e: ScaleCodecErr) -> Self {
            Self::DecodeError(DecodeError::new(e.to_string()))
        }
    }

    impl From<serde_json::Error> for Error {
        fn from(e: serde_json::Error) -> Self {
            Self::DecodeError(DecodeError::new(e.to_string()))
        }
    }

    /// The final Error type of RPCs to be serialized to protobuf.
    #[derive(Display, Message)]
    pub struct ProtoError {
        #[prost(string, tag = "1")]
        pub message: ::prost::alloc::string::String,
    }

    impl ProtoError {
        pub fn new(message: impl Into<String>) -> ProtoError {
            ProtoError {
                message: message.into(),
            }
        }
    }

    pub trait Service {
        async fn dispatch_request(
            &self,
            path: &str,
            data: impl AsRef<[u8]>,
            json: bool,
        ) -> Result<Vec<u8>, Error>;
    }
}

pub mod client {
    use super::*;

    /// The Error type for the generated client-side RPCs.
    #[derive(Display, Debug)]
    pub enum Error {
        /// Failed to decode the response from the server.
        DecodeError(DecodeError),
        /// The error returned by the server.
        ServerError(super::server::ProtoError),
        /// Other errors sush as networking error.
        RpcError(String),
    }

    impl From<DecodeError> for Error {
        fn from(e: DecodeError) -> Self {
            Self::DecodeError(e)
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for Error {}

    #[cfg(not(feature = "std"))]
    impl From<Error> for anyhow::Error {
        fn from(error: Error) -> Self {
            Self::msg(error)
        }
    }

    /// Trait for RPC client to implement the underlying data transport.
    /// Required by the generated RPC client.
    pub trait RequestClient {
        async fn request(&self, path: &str, body: Vec<u8>) -> Result<Vec<u8>, Error>;
    }
}

pub mod codec {
    use super::*;

    pub use parity_scale_codec as scale;

    pub fn encode_message_to_vec(msg: &impl Message) -> Vec<u8> {
        let mut buf = Vec::with_capacity(msg.encoded_len());

        msg.encode_raw(&mut buf);
        buf
    }
}
