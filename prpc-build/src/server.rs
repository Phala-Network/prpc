use super::{Method, Service};
use crate::{generate_doc_comment, generate_doc_comments, naive_snake_case, Builder};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Lit, LitStr};

/// Generate service for Server.
///
/// This takes some `Service` and will generate a `TokenStream` that contains
/// a public module containing the server service and handler trait.
pub fn generate<T: Service>(service: &T, config: &Builder) -> TokenStream {
    let attributes = &config.server_attributes;
    let methods = generate_methods(service, config, false);
    let json_methods = generate_methods(service, config, true);

    let server_service = quote::format_ident!("{}Server", service.name());
    let server_trait = quote::format_ident!("{}Rpc", service.name());
    let server_mod = quote::format_ident!("{}_server", naive_snake_case(service.name()));
    let service_name = Lit::Str(LitStr::new(service.name(), Span::call_site()));
    let supported_methods = generate_supported_methods(service, config);
    let method_enum = generate_methods_enum(service, config);
    let generated_trait = generate_trait(service, config, server_trait.clone());
    let service_doc = generate_doc_comments(service.comment());
    let mod_attributes = attributes.for_mod(service.package());
    let struct_attributes = attributes.for_struct(service.identifier());

    quote! {
        /// Generated server implementations.
        #(#mod_attributes)*
        pub mod #server_mod {
            use alloc::vec::Vec;

            #method_enum

            #generated_trait

            #service_doc
            #(#struct_attributes)*
            #[derive(Debug)]
            pub struct #server_service<T: #server_trait> {
                inner: T,
            }

            impl<T: #server_trait> #server_service<T> {
                pub fn new(inner: T) -> Self {
                    Self {
                        inner,
                    }
                }

                pub async fn dispatch_request(self, path: &str, _data: impl AsRef<[u8]>) -> Result<Vec<u8>, ::prpc::server::Error> {
                    #![allow(clippy::let_unit_value)]
                    match path {
                        #methods
                        _ => anyhow::bail!("Service not found: {path}"),
                    }
                }

                pub async fn dispatch_json_request(self, path: &str, _data: impl AsRef<[u8]>, _query: bool) -> Result<Vec<u8>, ::prpc::server::Error> {
                    #![allow(clippy::let_unit_value)]
                    match path {
                        #json_methods
                        _ => anyhow::bail!("Service not found: {path}"),
                    }
                }
                #supported_methods
            }

            impl<T: #server_trait> ::prpc::server::NamedService for #server_service<T> {
                const NAME: &'static str = #service_name;
            }
            impl<T: #server_trait> ::prpc::server::Service for #server_service<T> {
                type Methods = &'static [&'static str];
                fn methods() -> Self::Methods {
                    Self::supported_methods()
                }
                async fn dispatch_request(self, path: &str, data: impl AsRef<[u8]>, json: bool, query: bool) -> Result<Vec<u8>, ::prpc::server::Error> {
                    if json {
                        self.dispatch_json_request(path, data, query).await
                    } else {
                        self.dispatch_request(path, data).await
                    }
                }
            }
            impl<T: #server_trait> From<T> for #server_service<T> {
                fn from(inner: T) -> Self {
                    Self::new(inner)
                }
            }
        }
    }
}

fn generate_trait<T: Service>(service: &T, config: &Builder, server_trait: Ident) -> TokenStream {
    let methods =
        generate_trait_methods(service, &config.proto_path, config.compile_well_known_types);
    let trait_doc = generate_doc_comment(format!(
        "Generated trait containing RPC methods that should be implemented for use with {}Server.",
        service.name()
    ));

    quote! {
        #trait_doc
        pub trait #server_trait {
            #methods
        }
    }
}

fn generate_trait_methods<T: Service>(
    service: &T,
    proto_path: &str,
    compile_well_known_types: bool,
) -> TokenStream {
    let mut stream = TokenStream::new();

    for method in service.methods() {
        let name = quote::format_ident!("{}", method.name());

        let (req_message, res_message) =
            method.request_response_name(proto_path, compile_well_known_types);

        let method_doc = generate_doc_comments(method.comment());

        let method = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => {
                template_quote::quote! {
                    #method_doc
                    async fn #name(self
                        #(if req_message.is_some()) {
                            , request: #req_message
                        }
                    ) -> ::anyhow::Result<#res_message>;
                }
            }
            _ => {
                panic!("Streaming RPC not supported");
            }
        };

        stream.extend(method);
    }

    stream
}

fn generate_supported_methods<T: Service>(service: &T, config: &Builder) -> TokenStream {
    let mut all_methods = TokenStream::new();
    for method in service.methods() {
        let path = crate::join_path(
            config,
            service.package(),
            service.identifier(),
            method.identifier(),
        );

        let method_path = Lit::Str(LitStr::new(&path, Span::call_site()));
        all_methods.extend(quote! {
            #method_path,
        });
    }

    quote! {
        pub fn supported_methods()
            -> &'static [&'static str] {
                &[
                    #all_methods
                ]
            }
    }
}

fn generate_methods_enum<T: Service>(service: &T, config: &Builder) -> TokenStream {
    let mut paths = vec![];
    let mut variants = vec![];
    for method in service.methods() {
        let path = crate::join_path(
            config,
            service.package(),
            service.identifier(),
            method.identifier(),
        );

        let variant = Ident::new(method.identifier(), Span::call_site());
        variants.push(variant);

        let method_path = Lit::Str(LitStr::new(&path, Span::call_site()));
        paths.push(method_path);
    }

    let enum_name = Ident::new(
        &format!("{}Method", service.identifier()),
        Span::call_site(),
    );
    quote! {
        pub enum #enum_name {
            #(#variants,)*
        }

        impl #enum_name {
            #[allow(clippy::should_implement_trait)]
            pub fn from_str(path: &str) -> Option<Self> {
                match path {
                    #(#paths => Some(Self::#variants),)*
                    _ => None,
                }
            }
        }
    }
}

fn generate_methods<T: Service>(service: &T, config: &Builder, json: bool) -> TokenStream {
    let mut stream = TokenStream::new();

    for method in service.methods() {
        let path = crate::join_path(
            config,
            service.package(),
            service.identifier(),
            method.identifier(),
        );
        let method_path = Lit::Str(LitStr::new(&path, Span::call_site()));
        let method_ident = quote::format_ident!("{}", method.name());

        let method_stream = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => generate_unary(method, config, method_ident, json),
            _ => {
                panic!("Streaming RPC not supported");
            }
        };

        let method = quote! {
            #method_path => {
                #method_stream
            }
        };
        stream.extend(method);
    }

    stream
}

fn generate_unary<T: Method>(
    method: &T,
    config: &Builder,
    method_ident: Ident,
    json: bool,
) -> TokenStream {
    let (request, _response) =
        method.request_response_name(&config.proto_path, config.compile_well_known_types);

    if json {
        template_quote::quote! {
            #(if request.is_none()) {
                let response = self.inner.#method_ident().await?;
            }
            #(else) {
                let data = _data.as_ref();
                let input: #request = if data.is_empty() {
                    Default::default()
                } else if _query {
                    ::prpc::serde_qs::from_bytes(data)?
                } else {
                    ::prpc::serde_json::from_slice(data)?
                };
                let response = self.inner.#method_ident(input).await?;
            }
            Ok(serde_json::to_vec(&response)?)
        }
    } else {
        template_quote::quote! {
            #(if request.is_none()) {
                let response = self.inner.#method_ident().await?;
            }
            #(else) {
                let input: #request = ::prpc::Message::decode(_data.as_ref())?;
                let response = self.inner.#method_ident(input).await?;
            }
            Ok(::prpc::codec::encode_message_to_vec(&response))
        }
    }
}
