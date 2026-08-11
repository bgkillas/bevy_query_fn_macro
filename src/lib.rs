use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericArgument, ItemFn, PatType, PathArguments, Type, TypePath, parse_macro_input,
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
    quote! {
        #input
        #(#structs)*
    }
    .into()
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
            for e in &tuple.elems {
                match e {
                    Type::Reference(r) if let Type::Path(p) = &*r.elem => tys.push(Ty {
                        path: p.clone(),
                        name: to_snake(p.path.segments.last()?.ident.to_string()),
                        mutable: r.mutability.map(|_| RefTy::Mut).unwrap_or(RefTy::Ref),
                    }),
                    Type::Path(p) => tys.push(Ty {
                        path: p.clone(),
                        name: to_snake(p.path.segments.last()?.ident.to_string()),
                        mutable: RefTy::None,
                    }),
                    _ => return None,
                }
            }
            let ty_ident = tys
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<String>>()
                .join("_");
            let ident = format_ident!("_{name}_{}", ty_ident);
            let new_arg = quote! {#ident};
            *p = syn::parse2(new_arg).unwrap();
            let any_mut = if tys.iter().any(|a| matches!(a.mutable, RefTy::Mut)) {
                quote! {#[query_data(mutable)]}
            } else {
                quote! {}
            };
            let types = tys.into_iter().map(
                |Ty {
                     path,
                     name,
                     mutable,
                 }| match mutable {
                    RefTy::Mut => {
                        quote! {&'static mut #name: #path}
                    }
                    RefTy::Ref => {
                        quote! {&'static #name: #path}
                    }
                    RefTy::None => {
                        quote! {#name: #path}
                    }
                },
            );
            Some(quote! {
                #[derive(bevy::ecs::query::QueryData)]
                #any_mut
                struct #ident {
                    #(#types,)*
                }
            })
        }
        _ => None,
    }
}
fn to_snake(str: String) -> String {
    str.to_snake_case()
}
struct Ty {
    path: TypePath,
    name: String,
    mutable: RefTy,
}
enum RefTy {
    Mut,
    Ref,
    None,
}
