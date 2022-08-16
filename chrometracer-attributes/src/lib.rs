use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_quote,
    punctuated::Punctuated,
    Expr, Path, Token,
};

struct Fields {
    nested: Punctuated<Field, Token![,]>,
}

impl ToTokens for Fields {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.nested.to_tokens(tokens);
    }
}

struct Field {
    path: Path,
    eq_token: Token![=],
    value: Expr,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Field {
            path: input.parse()?,
            eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.path.to_tokens(tokens);
        self.eq_token.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

impl Parse for Fields {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Fields {
            nested: input.parse_terminated(Field::parse)?,
        })
    }
}

#[proc_macro_attribute]
pub fn instrument(attr: TokenStream, item: TokenStream) -> TokenStream {
    let fields = syn::parse_macro_input!(attr as Fields);

    let args = fields.nested.iter();
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
