use heck::ToSnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
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
    let name = input.sig.ident.clone();
    let mut structs = Vec::new();
    for (i, arg) in input.sig.inputs.iter_mut().enumerate() {
        let FnArg::Typed(PatType { ty, .. }) = arg else {
            continue;
        };
        if let Some(vec) = get_query_data(&name, i, ty) {
            structs.extend(vec);
        }
    }
    let tokens = quote! {
        #input
        #(#structs)*
    };
    tokens.into()
}
fn get_query_data(name: &Ident, i: usize, t: &mut Type) -> Option<Vec<TokenStream>> {
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
    let ident = last.ident.to_string();
    match ident.as_str() {
        "Option" => get_query_data(name, i, p),
        "ParamSet" => {
            let Type::Tuple(tuple) = &mut *p else {
                return None;
            };
            let mut vec = Vec::new();
            for t in &mut tuple.elems {
                if let Some(inner) = get_query_data(name, i, t) {
                    vec.extend(inner);
                }
            }
            Some(vec)
        }
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
            let ident = format_ident!("_{name}_{i}", span = name.span());
            let new_arg = quote! {#ident};
            *p = syn::parse2(new_arg).ok()?;
            let types = tys
                .into_iter()
                .map(|Ty { path, name }| (path, format_ident!("{name}", span = name.span())))
                .map(|(path, name)| quote! {#name: #path});
            let mut_att = if any_mut {
                quote! {#[query_data(mutable)]}
            } else {
                quote! {}
            };
            let tokens = quote! {
                #[derive(bevy::ecs::query::QueryData)]
                #mut_att
                #[allow(nonstandard_style)]
                pub struct #ident {
                    #(#types,)*
                }
            };
            Some(vec![tokens])
        }
        _ => None,
    }
}
fn get_type_data(ty: Type, any_mut: &mut bool) -> Option<Ty> {
    match ty {
        Type::Reference(mut r) => {
            let Type::Path(p) = &*r.elem else {
                return None;
            };
            let original = &p.path.segments.last()?.ident.clone();
            let name = to_snake(original.to_string());
            r.lifetime = Some(Lifetime::new("'static", Span::call_site()));
            *any_mut |= r.mutability.is_some();
            Some(Ty {
                path: Type::Reference(r),
                name: Ident::new(&name, original.span()),
            })
        }
        Type::Path(mut path) if path.path.segments.last()?.ident == "Option" => {
            let last = path.path.segments.last_mut()?;
            let PathArguments::AngleBracketed(list) = &mut last.arguments else {
                return None;
            };
            let ty = list.args.iter_mut().next()?;
            let GenericArgument::Type(inner) = ty else {
                return None;
            };
            let Ty { path: new, name } = get_type_data(inner.clone(), any_mut)?;
            *inner = new;
            Some(Ty {
                path: Type::Path(path),
                name,
            })
        }
        Type::Path(p) => {
            let original = &p.path.segments.last()?.ident.clone();
            let name = to_snake(original.to_string());
            Some(Ty {
                path: Type::Path(p),
                name: Ident::new(&name, original.span()),
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
    name: Ident,
}
