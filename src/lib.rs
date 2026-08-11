use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericArgument, ItemFn, Lifetime, PatType, PathArguments, Type, parse_macro_input,
};
#[proc_macro_attribute]
pub fn query_fn(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);
    let name = input.sig.ident.to_string();
    let mut structs = Vec::new();
    for arg in input.sig.inputs.iter_mut() {
        let FnArg::Typed(PatType { ty, .. }) = arg else {
            continue;
        };
        structs.extend(get_query_data(&name, ty));
    }
    let tokens = quote! {
        #[allow(private_interfaces)]
        #input
        #(#structs)*
    };
    tokens.into()
}
fn get_query_data(name: &str, t: &mut Type) -> Option<TokenStream> {
    let Type::Path(path) = &mut *t else {
        return None;
    };
    let last = path.path.segments.last_mut()?;
    let PathArguments::AngleBracketed(list) = &mut last.arguments else {
        return None;
    };
    let ty = list.args.iter_mut().next()?;
    let GenericArgument::Type(p) = ty else {
        return None;
    };
    match last.ident.to_string().as_str() {
        "Option" => get_query_data(name, p),
        "Single" | "Query" => {
            let Type::Tuple(tuple) = &*p else {
                return None;
            };
            if tuple.elems.len() <= 1 {
                return None;
            }
            let mut tys = Vec::new();
            let mut any_mut = false;
            for e in &tuple.elems {
                tys.extend(get_type_data(e.clone(), &mut any_mut))
            }
            let ty_ident = tys
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<String>>()
                .join("_");
            let ident = format_ident!("_{name}_{}", ty_ident);
            let new_arg = quote! {#ident};
            *p = syn::parse2(new_arg).ok()?;
            let types = tys
                .into_iter()
                .map(|Ty { path, name }| (path, format_ident!("{}", name)))
                .map(|(path, name)| quote! {#name: #path});
            let mut_att = if any_mut {
                quote! {#[query_data(mutable)]}
            } else {
                quote! {}
            };
            let tokens = quote! {
                #[derive(bevy::ecs::query::QueryData)]
                #mut_att
                struct #ident {
                    #(#types,)*
                }
            };
            Some(tokens)
        }
        _ => None,
    }
}
fn get_type_data(ty: Type, any_mut: &mut bool) -> Option<Ty> {
    match ty {
        Type::Reference(mut r) => {
            let name = if let Type::Path(p) = &*r.elem {
                to_snake(p.path.segments.last()?.ident.to_string())
            } else {
                return None;
            };
            r.lifetime = Some(Lifetime::new("'static", Span::call_site()));
            *any_mut |= r.mutability.is_some();
            Some(Ty {
                path: Type::Reference(r),
                name,
            })
        }
        Type::Path(p) if p.path.segments.last()?.ident == "Option" => {
            let last = p.path.segments.into_iter().next_back()?;
            let PathArguments::AngleBracketed(list) = last.arguments else {
                return None;
            };
            let ty = list.args.into_iter().next()?;
            let GenericArgument::Type(p) = ty else {
                return None;
            };
            let Ty { mut path, name } = get_type_data(p, any_mut)?;
            let tokens = quote! {Option<#path>};
            path = syn::parse2(tokens).ok()?;
            Some(Ty { path, name })
        }
        Type::Path(p) => {
            let name = to_snake(p.path.segments.last()?.ident.to_string());
            Some(Ty {
                path: Type::Path(p),
                name,
            })
        }
        _ => None,
    }
}
fn to_snake(str: String) -> String {
    str.to_snake_case()
}
struct Ty {
    path: Type,
    name: String,
}
