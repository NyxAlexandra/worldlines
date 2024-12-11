use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input,
    DeriveInput,
    Expr,
    Generics,
    Ident,
    Meta,
    Path,
    Token,
};

use crate::crate_path;

pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveComponent {
        ident,
        generics,
        crate_path,
        after_insert,
        before_remove,
    } = parse_macro_input!(input);
    let (impl_generics, type_generics, where_clause) =
        generics.split_for_impl();

    let after_insert = after_insert.map(|expr| {
        quote! {
            fn after_insert(entity: ::#crate_path::entity::EntityMut<'_>) {
                (#expr)(entity);
            }
        }
    });
    let before_remove = before_remove.map(|expr| {
        quote! {
            fn before_remove(entity: ::#crate_path::entity::EntityMut<'_>) {
                (#expr)(entity);
            }
        }
    });

    quote! {
        #[automatically_derived]
        impl #impl_generics ::#crate_path::component::Component for #ident #type_generics
        #where_clause
        {
            #after_insert

            #before_remove
        }
    }
    .into()
}

struct DeriveComponent {
    ident: Ident,
    generics: Generics,
    crate_path: Path,
    after_insert: Option<Expr>,
    before_remove: Option<Expr>,
}

impl Parse for DeriveComponent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput { ident, generics, attrs, .. } = input.parse()?;
        let crate_path = crate_path()?;

        let mut after_insert = None;
        let mut before_remove = None;

        for attr in attrs {
            if attr.path().is_ident("component") {
                let span = attr.meta.span();

                let Meta::List(list) = attr.meta else {
                    return Err(syn::Error::new(
                        span,
                        "expected `#[component(key = val)]`",
                    ));
                };

                list.parse_args_with(|input: ParseStream| {
                    let add_hook = |hook: &mut Option<_>, span| {
                        input.parse::<Token![=]>()?;

                        let expr: Expr = input.parse()?;

                        if hook.replace(expr).is_some() {
                            Err(syn::Error::new(span, "duplicate attribute"))
                        } else {
                            Ok(())
                        }
                    };

                    loop {
                        if input.is_empty() {
                            break;
                        }

                        let ident: Ident = input.parse()?;
                        let span = ident.span();

                        if ident == "after_insert" {
                            add_hook(&mut after_insert, span)?;
                        } else if ident == "before_remove" {
                            add_hook(&mut before_remove, span)?;
                        } else {
                            return Err(syn::Error::new(
                                span,
                                "expected `after_insert` or `before_remove`",
                            ));
                        }

                        if input.is_empty() {
                            break;
                        }

                        input.parse::<Token![,]>()?;
                    }

                    Ok(())
                })?;
            }
        }

        Ok(Self { ident, generics, crate_path, after_insert, before_remove })
    }
}
