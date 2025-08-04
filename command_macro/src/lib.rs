use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse_macro_input, AttributeArgs, ItemFn, Lit, Meta, NestedMeta,
};

#[proc_macro_attribute]
pub fn command(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let func = parse_macro_input!(input as ItemFn);

    let mut name: Option<String> = None;
    let mut aliases: Vec<String> = Vec::new();
    let mut description: Option<String> = None;
    let mut min: Option<usize> = None;
    let mut max: Option<usize> = None;

    for arg in args {
        match arg {
            NestedMeta::Meta(Meta::NameValue(nv)) => {
                if nv.path.is_ident("name") {
                    if let Lit::Str(s) = nv.lit {
                        name = Some(s.value());
                    }
                } else if nv.path.is_ident("description") {
                    if let Lit::Str(s) = nv.lit {
                        description = Some(s.value());
                    }
                } else if nv.path.is_ident("min") {
                    if let Lit::Int(i) = nv.lit {
                        min = Some(i.base10_parse().unwrap_or(0));
                    }
                } else if nv.path.is_ident("max") {
                    if let Lit::Int(i) = nv.lit {
                        max = Some(i.base10_parse().unwrap_or(0));
                    }
                }
            }
            NestedMeta::Meta(Meta::List(ml)) if ml.path.is_ident("aliases") => {
                for nested in ml.nested {
                    if let NestedMeta::Lit(Lit::Str(lit_str)) = nested {
                        aliases.push(lit_str.value());
                    }
                }
            }
            _ => {}
        }
    }

    let name = name.expect("command must have a name");
    let alias_literals = aliases
        .iter()
        .map(|s| quote! { #s })
        .collect::<Vec<_>>();

    let description = description.unwrap_or_default();
    let min = min.unwrap_or_default();
    let max = max.unwrap_or(usize::MAX);

    let fn_name = &func.sig.ident;
    let static_name = Ident::new(&format!("REGISTERED_COMMAND_{}", fn_name), Span::call_site());

    let expanded = quote! {
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

    TokenStream::from(expanded)
}