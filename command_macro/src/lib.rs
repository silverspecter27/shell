use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, ExprArray, ItemFn, Lit, Token,
};

struct CommandArgs {
    name: Option<String>,
    description: Option<String>,
    aliases: Vec<String>,
    min: Option<usize>,
    max: Option<usize>,
}

impl Parse for CommandArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut aliases = Vec::new();
        let mut min = None;
        let mut max = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if ident == "name" {
                let lit: Lit = input.parse()?;
                if let Lit::Str(s) = lit {
                    name = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(lit, "name must be a string literal"));
                }
            } else if ident == "description" {
                let lit: Lit = input.parse()?;
                if let Lit::Str(s) = lit {
                    description = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(lit, "description must be a string literal"));
                }
            } else if ident == "aliases" {
                let expr: Expr = input.parse()?;
                if let Expr::Array(ExprArray { elems, .. }) = expr {
                    for elem in elems {
                        if let Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(s),
                            ..
                        }) = elem
                        {
                            aliases.push(s.value());
                        } else {
                            return Err(syn::Error::new_spanned(elem, "aliases must be string literals"));
                        }
                    }
                } else {
                    return Err(syn::Error::new_spanned(expr, "aliases must be an array literal"));
                }
            } else if ident == "min" {
                let lit: Lit = input.parse()?;
                if let Lit::Int(i) = lit {
                    min = Some(i.base10_parse()?);
                } else {
                    return Err(syn::Error::new_spanned(lit, "min must be an integer literal"));
                }
            } else if ident == "max" {
                let lit: Lit = input.parse()?;
                if let Lit::Int(i) = lit {
                    max = Some(i.base10_parse()?);
                } else {
                    return Err(syn::Error::new_spanned(lit, "max must be an integer literal"));
                }
            } else {
                return Err(syn::Error::new_spanned(ident, "unknown argument"));
            }

            // Consume trailing comma if present
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            name,
            description,
            aliases,
            min,
            max,
        })
    }
}

#[proc_macro_attribute]
pub fn command(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_args = parse_macro_input!(args as CommandArgs);
    let func = parse_macro_input!(input as ItemFn);

    let fn_name = &func.sig.ident;

    let name = parsed_args.name.expect("Missing `name` in #[command]");
    let description = parsed_args.description.unwrap_or_default();
    let aliases = parsed_args.aliases;
    let min = parsed_args.min.unwrap_or_default();
    let max = parsed_args.max.unwrap_or(usize::MAX);

    let alias_literals = aliases.iter().map(|s| quote! { #s }).collect::<Vec<_>>();
    let static_name = Ident::new(&format!("REGISTERED_COMMAND_{}", fn_name), Span::call_site());

    let output = quote! {
        #func

        #[linkme::distributed_slice(crate::COMMANDS)]
        static #static_name: fn() -> &'static crate::CommandInfo = || &crate::CommandInfo {
            name: #name,
            description: #description,
            aliases: &[ #( #alias_literals ),* ],
            min: #min,
            max: #max,
            handler: #fn_name,
        };
    };

    TokenStream::from(output)
}
