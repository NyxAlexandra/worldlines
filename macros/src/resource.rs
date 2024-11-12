use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, DeriveInput, Generics, Ident, Path};

use crate::crate_path;

pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveResource { ident, generics, crate_path } =
        parse_macro_input!(input);
    let (impl_generics, type_generics, where_clause) =
        generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics ::#crate_path::resource::Resource for #ident #type_generics
        #where_clause
        {
        }
    }.into()
}

struct DeriveResource {
    ident: Ident,
    generics: Generics,
    crate_path: Path,
}

impl Parse for DeriveResource {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput { ident, generics, .. } = input.parse()?;
        let crate_path = crate_path()?;

        Ok(Self { ident, generics, crate_path })
    }
}
