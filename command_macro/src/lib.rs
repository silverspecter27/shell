use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream}, parse_macro_input, Expr, ExprArray, GenericArgument, ItemFn,
    Lit, PathArguments, Token, Type,
};

struct CommandArgs {
    name: Option<String>,
    description: Option<String>,
    aliases: Vec<String>,
}

impl Parse for CommandArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = CommandArgs { name: None, description: None, aliases: vec![] };
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match ident.to_string().as_str() {
                "name" => args.name = Some(parse_lit_string(input)?),
                "description" => args.description = Some(parse_lit_string(input)?),
                "aliases" => args.aliases = parse_aliases_array(input)?,
                _ => return Err(syn::Error::new_spanned(ident, "unknown argument")),
            }
            if input.peek(Token![,]) { input.parse::<Token![,]>()?; }
        }
        Ok(args)
    }
}

fn parse_lit_string(input: ParseStream) -> syn::Result<String> {
    let Lit::Str(s) = input.parse()? else {
        return Err(input.error("expected string literal"));
    };
    Ok(s.value())
}

fn parse_aliases_array(input: ParseStream) -> syn::Result<Vec<String>> {
    let Expr::Array(ExprArray { elems, .. }) = input.parse()? else {
        return Err(input.error("aliases must be an array literal"));
    };
    elems.into_iter().map(|elem| {
        if let Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = elem {
            Ok(s.value())
        } else {
            Err(syn::Error::new_spanned(elem, "aliases must be string literals"))
        }
    }).collect()
}

fn extract_inner<'a>(ty: &'a Type, container: &str) -> Option<&'a Type> {
    if let Type::Path(path) = ty {
        path.path.segments.first().and_then(|seg| {
            if seg.ident == container {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
            None
        })
    } else { None }
}
fn extract_option(ty: &Type) -> Option<&Type> { extract_inner(ty, "Option") }
fn extract_vec(ty: &Type) -> Option<&Type> { extract_inner(ty, "Vec") }

fn min_count(args: &[(Ident, &Type)]) -> usize {
    args.iter().filter(|(_, ty)| extract_option(ty).is_none()).count()
}

fn generate_parse_exprs<'a>(
    fn_args: &'a [(Ident, &'a Type)],
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    fn_args.iter().enumerate().map(|(i, (ident, ty))| {
        if let Some(inner_vec) = extract_option(ty).and_then(extract_vec) {
            quote! {
                let #ident: Option<Vec<#inner_vec>> = if args.len() > #i {
                    Some(args[#i..].iter()
                        .map(|a| <#inner_vec as crate::ParseArgument>::parse(a))
                        .collect::<Result<Vec<_>, _>>()?)
                } else { None };
            }
        } else if let Some(inner_vec) = extract_vec(ty) {
            quote! {
                if args.len() <= #i {
                    return Err(crate::CommandError::TooFewArguments(args.len(), self.command_info()));
                }
                let #ident: Vec<#inner_vec> = args[#i..].iter()
                    .map(|a| <#inner_vec as crate::ParseArgument>::parse(a))
                    .collect::<Result<Vec<_>, _>>()?;
            }
        } else if let Some(inner) = extract_option(ty) {
            quote! {
                let #ident: Option<#inner> = if args.len() > #i {
                    Some(<#inner as crate::ParseArgument>::parse(args[#i])?)
                } else { None };
            }
        } else {
            quote! {
                if args.len() <= #i {
                    return Err(crate::CommandError::TooFewArguments(args.len(), self.command_info()));
                }
                let #ident: #ty = <#ty as crate::ParseArgument>::parse(args[#i])?;
            }
        }
    })
}

#[proc_macro_attribute]
pub fn command(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_args = parse_macro_input!(args as CommandArgs);
    let func = parse_macro_input!(input as ItemFn);
    let fn_name = &func.sig.ident;

    let fn_args: Vec<(Ident, &Type)> = func.sig.inputs.iter().filter_map(|arg| match arg {
        syn::FnArg::Typed(pat_type) => match &*pat_type.pat {
            syn::Pat::Ident(ident) => Some((ident.ident.clone(), &*pat_type.ty)),
            _ => None,
        },
        _ => None,
    }).collect();

    let handler_struct = format_ident!("{}Handler", fn_name.to_string().to_case(Case::UpperCamel));
    let handler_static = Ident::new(&format!("REGISTERED_COMMAND_{}", fn_name).to_uppercase(), Span::call_site());

    let name = parsed_args.name.expect("Missing `name` in #[command]");
    let description = parsed_args.description.unwrap_or_default();
    let alias_literals = parsed_args.aliases.iter().map(|s| quote! { #s });

    let min_args = min_count(&fn_args);
    let max_args = fn_args.iter().any(|(_, ty)| extract_vec(ty).is_some())
        .then(|| usize::MAX)
        .unwrap_or(fn_args.len());
    let parse_exprs = generate_parse_exprs(&fn_args);
    let call_args = fn_args.iter().map(|(ident, _)| ident);

    let output = quote! {
        #func

        struct #handler_struct;

        impl crate::CommandHandler for #handler_struct {
            fn call(&self, args: &[&str]) -> Result<(), crate::CommandError> {
                if args.len() < #min_args {
                    return Err(crate::CommandError::TooFewArguments(args.len(), self.command_info()));
                }
                if args.len() > #max_args {
                    return Err(crate::CommandError::TooManyArguments(args.len(), self.command_info()));
                }

                #(#parse_exprs)*

                #fn_name(#(#call_args),*)
            }

            fn command_info(&self) -> &'static crate::CommandInfo {
                #handler_static
            }
        }

        #[linkme::distributed_slice(crate::COMMANDS)]
        static #handler_static: &'static crate::CommandInfo = &crate::CommandInfo {
            name: #name,
            description: #description,
            aliases: &[ #( #alias_literals ),* ],
            min: #min_args,
            max: #max_args,
            handler: &#handler_struct,
        };
    };

    output.into()
}
