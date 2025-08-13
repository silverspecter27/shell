use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream}, parse_macro_input, Expr, ExprArray, GenericArgument, ItemFn,
    Lit, PathArguments, Token, Type,
};

/// Command arguments for the macro
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
    let lit: Lit = input.parse()?;
    if let Lit::Str(s) = lit { Ok(s.value()) } else { Err(syn::Error::new_spanned(lit, "expected string literal")) }
}

fn parse_aliases_array(input: ParseStream) -> syn::Result<Vec<String>> {
    let expr: Expr = input.parse()?;
    if let Expr::Array(ExprArray { elems, .. }) = expr {
        elems.into_iter().map(|elem| {
            if let Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = elem {
                Ok(s.value())
            } else {
                Err(syn::Error::new_spanned(elem, "aliases must be string literals"))
            }
        }).collect()
    } else { Err(syn::Error::new_spanned(expr, "aliases must be an array literal")) }
}

fn extract_option_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(path) = ty {
        path.path.segments.first().and_then(|seg| {
            if seg.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() { return Some(inner); }
                }
            }
            None
        })
    } else { None }
}

fn extract_vec_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(path) = ty {
        path.path.segments.first().and_then(|seg| {
            if seg.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() { return Some(inner); }
                }
            }
            None
        })
    } else { None }
}

fn min_count(args: &[(Ident, &Type)]) -> usize {
    args.iter().filter(|(_, ty)| extract_option_inner(ty).is_none()).count()
}

/// Detect the last argument type (Vec / Option<Vec> / normal)
fn detect_last_arg<'a>(args: &'a [(Ident, &'a Type)]) -> (bool, Option<&'a Type>, bool) {
    if let Some((_, last_ty)) = args.last() {
        let vec_inner = extract_vec_inner(last_ty);
        let option_vec_inner = extract_option_inner(last_ty).and_then(extract_vec_inner);
        (vec_inner.is_some(), vec_inner.or(option_vec_inner), option_vec_inner.is_some())
    } else {
        (false, None, false)
    }
}

/// Generate parsing code for each function argument
fn generate_parse_exprs<'a>(
    fn_args: &'a [(Ident, &'a Type)],
    last_index: usize,
    is_last_vec: bool,
    last_vec_inner: Option<&'a Type>,
    is_last_option_vec: bool,
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    fn_args.iter().enumerate().map(move |(i, (ident, ty))| {
        let parse_single = |ty: &Type, idx: usize| {
            if let Some(inner) = extract_option_inner(ty) {
                quote! {
                    let #ident: Option<#inner> = if args.len() > #idx {
                        Some(<#inner as crate::ParseArgument>::parse(args[#idx])?)
                    } else { None };
                }
            } else {
                quote! {
                    if args.len() <= #idx {
                        return Err(crate::CommandError::TooFewArguments(args.len(), self.command_info()));
                    }
                    let #ident: #ty = <#ty as crate::ParseArgument>::parse(args[#idx])?;
                }
            }
        };

        if i < last_index { parse_single(ty, i) } 
        else if is_last_vec {
            let inner_ty = last_vec_inner.unwrap();
            if is_last_option_vec {
                quote! {
                    let #ident: Option<Vec<#inner_ty>> = if args.len() > #i {
                        Some(args[#i..].iter()
                            .map(|a| <#inner_ty as crate::ParseArgument>::parse(a))
                            .collect::<Result<Vec<_>, _>>()?)
                    } else { None };
                }
            } else {
                quote! {
                    if args.len() <= #i {
                        return Err(crate::CommandError::TooFewArguments(args.len(), self.command_info()));
                    }
                    let #ident: Vec<#inner_ty> = args[#i..].iter()
                        .map(|a| <#inner_ty as crate::ParseArgument>::parse(a))
                        .collect::<Result<Vec<_>, _>>()?;
                }
            }
        } else { parse_single(ty, i) }
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

    let total_args = fn_args.len();
    let min_args = min_count(&fn_args);
    let last_index = total_args.saturating_sub(1);
    let (is_last_vec, last_vec_inner, is_last_option_vec) = detect_last_arg(&fn_args);
    let max_args = if is_last_vec { usize::MAX } else { total_args };
    let parse_exprs = generate_parse_exprs(&fn_args, last_index, is_last_vec, last_vec_inner, is_last_option_vec);
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
