use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::Expr;

fn do_bullshit(input: TokenStream, name: &str) -> TokenStream {
    let input2: proc_macro2::TokenStream = input.into();

    let mut seen_first_comma = false;
    let mut part1 = TokenStream2::new();
    let mut part_rest = TokenStream2::new();
    for tt in input2.into_iter() {
        if !seen_first_comma {
            if let TokenTree::Punct(p) = &tt {
                if p.as_char() == ',' {
                    seen_first_comma = true;
                    continue;
                }
            }
            part1.extend(std::iter::once(tt));
        } else {
            part_rest.extend(std::iter::once(tt));
        }
    }

    if !seen_first_comma {
        panic!("invalid input: expected comma after dst");
    }

    let dst: Expr = syn::parse2(part1).expect("invalid dst expr");

    let v: Vec<TokenTree> = part_rest.clone().into_iter().collect();
    let mut last_comma_pos: Option<usize> = None;
    let depth = 0usize;
    for (i, tt) in v.iter().enumerate() {
        match tt {
            TokenTree::Punct(p) if p.as_char() == ',' && depth == 0 => {
                last_comma_pos = Some(i);
            }
            _ => {}
        }
    }

    let (ft_ts, val_ts_opt) = if let Some(pos) = last_comma_pos {
        let ft = TokenStream2::from_iter(v[..pos].iter().cloned());
        let val = TokenStream2::from_iter(v[pos + 1..].iter().cloned());
        (ft, Some(val))
    } else {
        (part_rest, None)
    };

    let s = ft_ts.to_string();
    let mut sanitized: String = s
        // .chars()
        // .filter_map(|c| match c {
        //     '>' => None,
        //     '<' => Some('_'),
        //     other => Some(other),
        // })
        // .collect();
        .split("<")
        .collect::<Vec<&str>>()
        .first()
        .unwrap()
        .to_string();
    if sanitized
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        sanitized.insert(0, '_');
    }
    sanitized = sanitized
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    sanitized = sanitized.to_snake_case();

    let method_name = format!("{}{}", name, sanitized);
    let ident = proc_macro2::Ident::new(&method_name, Span::call_site());

    let out = if let Some(val_ts) = val_ts_opt {
        let val: Expr = syn::parse2(val_ts).expect("invalid value expression");
        quote! { #dst.#ident(#val)?; }
    } else {
        quote! { #dst.#ident()? }
    };

    out.into()
}

#[proc_macro]
pub fn put_ident(input: TokenStream) -> TokenStream {
    do_bullshit(input, "put_")
}

#[proc_macro]
pub fn get_ident(input: TokenStream) -> TokenStream {
    do_bullshit(input, "get_")
}
