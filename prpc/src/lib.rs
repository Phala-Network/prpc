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
    use parity_scale_codec::Error as ScaleCodecErr;

    use core::marker::PhantomData;

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

    pub trait NamedService: Service {
        const NAME: &'static str;
    }

    pub trait Service {
        fn methods() -> Vec<&'static str>;
        async fn dispatch_request(
            &self,
            path: &str,
            data: impl AsRef<[u8]>,
            json: bool,
        ) -> Result<Vec<u8>, Error>;
    }

    pub struct ComposedService<T, A> {
        app: A,
        _marker: PhantomData<T>,
    }

    impl<T, A> ComposedService<T, A> {
        pub fn new(app: A) -> Self {
            Self {
                app,
                _marker: PhantomData,
            }
        }
    }

    impl<A, T> From<A> for ComposedService<T, A> {
        fn from(app: A) -> Self {
            Self::new(app)
        }
    }

    // Macro to implement Foo for tuples where each element implements Foo.
    macro_rules! impl_service_for_tuple {
        // Base case for the recursion: an empty tuple implements Foo.
        () => {
            impl<A> Service for ComposedService<(), A> {
                fn methods() -> Vec<&'static str> {
                    Vec::new()
                }

                async fn dispatch_request(
                    &self,
                    _path: &str,
                    _data: impl AsRef<[u8]>,
                    _json: bool,
                ) -> Result<Vec<u8>, Error> {
                    Err(Error::NotFound)
                }
            }
        };

        // Recursive step: T implements Foo, and so does Tail.
        ( $head:ident $(, $tail:ident)* $(,)*) => {
            impl<A, $head, $( $tail, )*> Service for ComposedService<($head, $( $tail, )*), A>
            where
                A: Clone,
                $head: NamedService + From<A>,
                $( $tail: NamedService + From<A>, )*
            {
                fn methods() -> Vec<&'static str> {
                    let mut methods = Vec::new();
                    methods.extend_from_slice($head::methods().as_slice());
                    $(
                        methods.extend_from_slice($tail::methods().as_slice());
                    )*
                    methods
                }

                async fn dispatch_request(
                    &self,
                    path: &str,
                    data: impl AsRef<[u8]>,
                    json: bool,
                ) -> Result<Vec<u8>, Error> {
                    let service_name = path.split('.').next().unwrap_or_default();
                    if service_name == $head::NAME {
                        return $head::from(self.app.clone()).dispatch_request(path, data, json).await;
                    }
                    $(
                        if service_name == $tail::NAME {
                            return $tail::from(self.app.clone()).dispatch_request(path, data, json).await;
                        }
                    )*
                    Err(Error::NotFound)
                }
            }

            // Recurse to the next smaller tuple.
            impl_service_for_tuple!($($tail,)*);
        };
    }

    // impl_service_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
    impl_service_for_tuple!(T1, T2);
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
