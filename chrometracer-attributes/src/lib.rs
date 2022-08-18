use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_quote,
    punctuated::Punctuated,
    Expr, LitStr, Path, Token,
};

struct ChromeEventArgs {
    event: Option<Event>,
    fields: Punctuated<Field, Token![,]>,
}

impl ToTokens for ChromeEventArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.fields.to_tokens(tokens);
    }
}

impl Parse for ChromeEventArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(ChromeEventArgs {
            event: {
                if input.peek(kw::event) {
                    Some(Event::parse(input)?)
                } else {
                    None
                }
            },
            fields: input.parse_terminated(Field::parse)?,
        })
    }
}

struct Event {
    keyword: kw::event,
    colon_token: Token![:],
    event: LitStr,
    comma_token: Token![,],
}

impl Parse for Event {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Event {
            keyword: input.parse()?,
            colon_token: input.parse()?,
            event: input.parse()?,
            comma_token: input.parse()?,
        })
    }
}

impl ToTokens for Event {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.keyword.to_tokens(tokens);
        self.colon_token.to_tokens(tokens);
        self.event.to_tokens(tokens);
        self.comma_token.to_tokens(tokens);
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

#[proc_macro_attribute]
pub fn instrument(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as ChromeEventArgs);

    let event = args
        .event
        .map(|e| e.event)
        .unwrap_or_else(|| LitStr::new("", Span::call_site()));

    let fields = args.fields.iter();
    let fields2 = args.fields.iter();
    let fields3 = args.fields.iter();

    let mut input = syn::Item::parse.parse(item).unwrap();

    if let syn::Item::Fn(ref mut item) = input {
        let original = &item.block;
        item.block = Box::new(parse_quote! {{
            let start = chrometracer::current(|tracer| tracer.map(|t| t.start));

            if let Some(start) = start {
                let event = match #event {
                    "async" => Some((chrometracer::EventType::AsyncStart, chrometracer::EventType::AsyncEnd)),
                    "" => None,
                    _ => panic!("Unknown event, expected one of \"async\"")
                };

                let ret = if let Some(event) = event {
                    chrometracer::event!(#(#fields,)* ph = event.0,
                        ts = ::std::time::Instant::now().duration_since(start).as_nanos() as f64 / 1000.0);
                    let ret = #original;
                    chrometracer::event!(#(#fields2,)* ph = event.1,
                        ts = ::std::time::Instant::now().duration_since(start).as_nanos() as f64 / 1000.0);
                    ret
                } else {
                    let now = ::std::time::Instant::now();
                    let ts = now.duration_since(start).as_nanos() as f64 / 1000.0;
                    let ret = #original;
                    let dur = ::std::time::Instant::now().duration_since(now).as_nanos() as f64 / 1000.0;

                    chrometracer::event!(#(#fields3,)* ph = chrometracer::EventType::Complete, dur = dur, ts = ts);
                    ret
                };

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

mod kw {
    syn::custom_keyword!(event);
}
