use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, Parser},
    parse_quote, Error,
};

#[proc_macro_attribute]
pub fn instrument(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut args = vec![];
    for arg in syn::parse_macro_input!(attr as syn::AttributeArgs) {
        match arg {
            syn::NestedMeta::Meta(meta) => match meta {
                syn::Meta::Path(_) => unreachable!(),
                syn::Meta::List(_) => unreachable!(),
                syn::Meta::NameValue(value) => {
                    args.push(value);
                }
            },
            syn::NestedMeta::Lit(lit) => {
                return Error::new(lit.span(), "Unexpected attribute")
                    .into_compile_error()
                    .into();
            }
        };
    }

    let mut input = syn::Item::parse.parse(item).unwrap();

    if let syn::Item::Fn(ref mut item) = input {
        let original = &item.block;
        item.block = Box::new(parse_quote! {{
            let start = chrometracer::current(|tracer| tracer.start);

            let now = ::std::time::Instant::now();

            let ts = now.duration_since(start).as_nanos() as f64 / 1000.0;
            let ret = #original;
            let dur = ::std::time::Instant::now().duration_since(now).as_nanos() as f64 / 1000.0;

            chrometracer::event!(#(#args ,)* ph = chrometracer::EventType::Complete, dur = dur, ts = ts);

            ret
        }});
    } else {
        unreachable!()
    }

    input.into_token_stream().into()
}
