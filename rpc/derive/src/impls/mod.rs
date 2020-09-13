pub mod method;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::*;

use self::method::*;

fn filter_forward_trait_item(item: &TraitItem) -> Option<TokenStream2> {
    match item {
        TraitItem::Method(_) => None,
        item => Some(quote! { #item }),
    }
}

fn filter_method_trait_item(item: &TraitItem) -> Option<TraitItemMethod> {
    match item {
        TraitItem::Method(method) => Some(method.clone()),
        _ => None,
    }
}

fn map_method_trait_item(method: TraitItemMethod) -> std::result::Result<MethodDef, TokenStream2> {
    let (attr, attributes) = match parse_method_attrs(&method.attrs) {
        Ok(res) => res,
        Err(err) => return Err(err),
    };
    let name = &method.sig.ident;
    let inputs = &method.sig.inputs;
    let output = &method.sig.output;
    let definition = quote! {
        #(#attributes)*
        async fn #name (#inputs) #output;
    };
    Ok(MethodDef {
        attr,
        item: method.clone(),
        definition,
        attributes,
    })
}

fn make_method_message_handler(def: &MethodDef) -> TokenStream2 {
    let name = def.name();
    let method_name = def.method_name();
    quote! {
        #method_name => {
            let params = message.read()?;
            match self. #name (&params).await {
                Ok(response) => {
                    Ok(builder::new_response::<M>(&message).with_data(&response)?.build())
                },
                Err(Error::Rpc(err)) => {
                    Ok(builder::new_error_response::<M>(&message, err).build())
                },
                Err(Error::Io(err)) => Err(err),
            }
        },
    }
}

fn make_request_method(def: &MethodDef) -> TokenStream2 {
    // Trait method name
    let name = Ident::new(&format!("{}_request", def.name()), Span::call_site());
    // RPC method name
    let method_name = def.method_name();
    // Get inputs without `self`
    let inputs = def.inputs();
    // Get input names
    let input_names = def.input_names();
    quote! {
        fn #name <M> (#inputs) -> std::io::Result<M>
        where
            M: net3_msg::builder::MessageBuilderExt<Builder = M> + Send + Sync + 'static,
            M: net3_msg::builder::MessageBuilder<M>,
        {
            net3_msg::builder::new_request::<M, _>(net3_msg::types::Id::Null, #method_name, Some(#input_names))
        }
    }
}

fn make_method_client(def: &MethodDef) -> TokenStream2 {
    let sig = def.signature();
    let name = &sig.ident;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let method_name = def.method_name();
    let input_names = def.input_names();
    quote! {
        async fn #name (#inputs) #output {
            self.request( #method_name , Some( #input_names )).await
        }
    }
}

pub fn make_rpc(item: ItemTrait) -> Result<TokenStream2> {
    // Filter RPC methods
    let methods = item
        .items
        .iter()
        .filter_map(filter_method_trait_item)
        .map(map_method_trait_item)
        .collect::<std::result::Result<Vec<_>, TokenStream2>>();
    let methods = match methods {
        Ok(methods) => methods,
        Err(err) => return Ok(err),
    };

    // Filter trait items to keep unchanged
    let fwd_trait_items = item
        .items
        .iter()
        .filter_map(filter_forward_trait_item)
        .collect::<Vec<_>>();

    // Blanket RPC methods definitions
    let definitions = methods
        .iter()
        .map(|def| &def.definition)
        .collect::<Vec<_>>();

    // Create method handlers in `handle_message`
    let method_handlers = methods
        .iter()
        .map(make_method_message_handler)
        .collect::<Vec<_>>();

    // Create request handlers for all RPC methods
    let method_requests = methods.iter().map(make_request_method).collect::<Vec<_>>();

    // Get slice of RPC method names
    let methods_slice = methods
        .iter()
        .map(|def| {
            let method_name = def.method_name();
            quote! { #method_name, }
        })
        .collect::<Vec<_>>();

    // Create client methods
    let methods_client = methods.iter().map(make_method_client).collect::<Vec<_>>();

    let vis = &item.vis;
    let unsafety = &item.unsafety;
    let trait_token = &item.trait_token;
    let rpc_trait_name = &item.ident;
    Ok(quote! {
        #[async_trait::async_trait]
        #vis
        #unsafety
        #trait_token
        #rpc_trait_name
        {
            #(#fwd_trait_items)*
            #(#definitions)*
            #(#method_requests)*

            fn methods() -> &'static [&'static str] {
                &[
                    #(#methods_slice)*
                ]
            }

            async fn handle_message<M>(&mut self, message: M) -> std::io::Result<M>
            where
                M: net3_msg::traits::Message + 'static,
                // M: net3_msg::builder::MessageBuilder<M>,
            {
                use net3_msg::prelude::*;
                use net3_rpc_error::Error;
                let method = message.method().expect("method name");
                match method {
                    #(#method_handlers)*
                    method => Ok(builder::new_error_response::<M>(
                        &message,
                        net3_msg::types::ErrorKind::MethodNotFound.into(),
                    )
                    .build()),
                }
            }
        }

        #[async_trait::async_trait]
        impl<M: net3_msg::traits::Message> #rpc_trait_name for net3_rpc_client::Handle<M> {
            #(#methods_client)*
        }

    })
}
