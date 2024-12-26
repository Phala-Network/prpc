use super::{Method, Service};
use crate::{generate_doc_comments, naive_snake_case, Builder};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate service for client.
///
/// This takes some `Service` and will generate a `TokenStream` that contains
/// a public module with the generated client.
pub fn generate<T: Service>(service: &T, config: &Builder) -> TokenStream {
    let attributes = &config.client_attributes;
    let service_ident = quote::format_ident!("{}Client", service.name());
    let client_mod = quote::format_ident!("{}_client", naive_snake_case(service.name()));
    let methods = generate_methods(service, config);

    let service_doc = generate_doc_comments(service.comment());
    let mod_attributes = attributes.for_mod(service.package());
    let struct_attributes = attributes.for_struct(service.identifier());

    quote! {
        /// Generated client implementations.
        #(#mod_attributes)*
        pub mod #client_mod {
            #service_doc
            #(#struct_attributes)*
            #[derive(Debug)]
            pub struct #service_ident<Client> {
                pub client: Client,
            }

            impl<Client> #service_ident<Client>
            where
                Client: ::prpc::client::RequestClient
            {
                pub fn new(client: Client) -> Self {
                    Self { client }
                }

                #methods
            }
        }
    }
}

fn generate_methods<T: Service>(service: &T, config: &Builder) -> TokenStream {
    let mut stream = TokenStream::new();
    for method in service.methods() {
        let path = crate::join_path(
            config,
            service.package(),
            service.identifier(),
            method.identifier(),
        );

        stream.extend(generate_doc_comments(method.comment()));

        let method = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => generate_unary(method, config, path),
            _ => {
                panic!("Only unary method supported");
            }
        };

        stream.extend(method);
    }

    stream
}

fn generate_unary<T: Method>(method: &T, config: &Builder, path: String) -> TokenStream {
    let ident = format_ident!("{}", method.name());
    let (request, response) =
        method.request_response_name(&config.proto_path, config.compile_well_known_types);

    template_quote::quote! {
        pub async fn #ident(
            &self
            #(if request.is_some())
            {
                , request: #request,
            }
        ) -> Result<#response, ::prpc::client::Error> {
            #(if request.is_none())
            {
                let request = ();
            }
            Ok(self.client.request(#path, request).await?)
        }
    }
}
