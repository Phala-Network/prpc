#![cfg_attr(not(feature = "std"), no_std)]
#![allow(async_fn_in_trait)]

extern crate alloc;

use alloc::vec::Vec;

pub use prost::Message;

pub mod serde_helpers;

pub use serde_json;
pub use serde_qs;

pub mod server {
    use super::*;
    pub use anyhow::Error;

    use core::marker::PhantomData;
    use derive_more::Display;

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
        type Methods: AsRef<[&'static str]>;
        fn methods() -> Self::Methods;
        async fn dispatch_request(
            self,
            path: &str,
            data: impl AsRef<[u8]>,
            json: bool,
            query: bool,
        ) -> Result<Vec<u8>, Error>;
    }

    pub struct ComposedService<A, T> {
        app: A,
        _marker: PhantomData<T>,
    }

    impl<T, A> ComposedService<A, T> {
        pub fn new(app: A) -> Self {
            Self {
                app,
                _marker: PhantomData,
            }
        }
    }

    impl<A, T> From<A> for ComposedService<A, T> {
        fn from(app: A) -> Self {
            Self::new(app)
        }
    }

    // Macro to implement Foo for tuples where each element implements Foo.
    macro_rules! impl_service_for_tuple {
        // Base case for the recursion: an empty tuple implements Foo.
        () => {
            impl<A> Service for ComposedService<A, ()> {
                type Methods = Vec<&'static str>;
                fn methods() -> Vec<&'static str> {
                    Vec::new()
                }

                async fn dispatch_request(
                    self,
                    path: &str,
                    _data: impl AsRef<[u8]>,
                    _json: bool,
                    _query: bool,
                ) -> Result<Vec<u8>, Error> {
                    anyhow::bail!("Service not found: {path}")
                }
            }
        };

        // Recursive step: T implements Foo, and so does Tail.
        ( $head:ident $(, $tail:ident)* $(,)*) => {
            impl<A, $head, $( $tail, )*> Service for ComposedService<A, ($head, $( $tail, )*)>
            where
                $head: NamedService + From<A>,
                $( $tail: NamedService + From<A>, )*
            {
                type Methods = Vec<&'static str>;
                fn methods() -> Self::Methods {
                    let mut methods = Vec::new();
                    methods.extend_from_slice($head::methods().as_ref());
                    $(
                        methods.extend_from_slice($tail::methods().as_ref());
                    )*
                    methods
                }

                async fn dispatch_request(
                    self,
                    path: &str,
                    data: impl AsRef<[u8]>,
                    json: bool,
                    query: bool,
                ) -> Result<Vec<u8>, Error> {
                    let service_name = path.split('.').next().unwrap_or_default();
                    if service_name == $head::NAME {
                        return $head::from(self.app).dispatch_request(path, data, json, query).await;
                    }
                    $(
                        if service_name == $tail::NAME {
                            return $tail::from(self.app).dispatch_request(path, data, json, query).await;
                        }
                    )*
                    anyhow::bail!("Service not found: {service_name}")
                }
            }

            // Recurse to the next smaller tuple.
            impl_service_for_tuple!($($tail,)*);
        };
    }

    impl_service_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
}

pub mod client {
    use serde::{de::DeserializeOwned, Serialize};

    use super::*;
    pub use anyhow::Error;

    /// Trait for RPC client to implement the underlying data transport.
    /// Required by the generated RPC client.
    pub trait RequestClient {
        async fn request<T, R>(&self, path: &str, body: T) -> Result<R, Error>
        where
            T: Message + Serialize,
            R: Message + DeserializeOwned;
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
