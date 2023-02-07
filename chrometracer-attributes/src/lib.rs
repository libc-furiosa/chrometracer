use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, Parser},
    parse_quote,
};

#[proc_macro_attribute]
pub fn instrument(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = syn::Item::parse.parse(item).unwrap();

    if let syn::Item::Fn(ref mut item) = input {
        let original = &item.block;
        let name = &item.sig.ident;
        let is_async = item.sig.asyncness.is_some();

        item.block = Box::new(parse_quote! {{
            let start = chrometracer::current(|tracer| tracer.map(|t| t.start));

            if let Some(start) = start {
                let from = start.elapsed();
                let ret = #original;
                let to = start.elapsed();
                chrometracer::event!(name: stringify!(#name), from: from, to: to, is_async: #is_async);

                ret
            } else {
                #original
            }
        }});
    } else {
        unreachable!()
    }

    input.into_token_stream().into()
}

