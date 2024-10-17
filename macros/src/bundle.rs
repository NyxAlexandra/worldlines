use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{
    parse_macro_input,
    Data,
    DataStruct,
    DeriveInput,
    Field,
    Fields,
    Generics,
    Ident,
    Path,
};

use crate::crate_path;

pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveBundle { ident, generics, fields, crate_path } =
        parse_macro_input!(input);

    let (impl_generics, type_generics, where_clause) =
        generics.split_for_impl();

    // `(T::types(), self.t.take(&mut f))`
    let (types, takes): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .enumerate()
        .map(|(i, Field { ident, ty, .. })| {
            (
                quote! {
                    #ty::types()
                },
                {
                    let field_ident = ident.map(FieldIdent::Named).unwrap_or(
                        FieldIdent::Indexed(Literal::usize_unsuffixed(i)),
                    );

                    quote! {
                        self.#field_ident.take(writer);
                    }
                },
            )
        })
        .unzip();

    quote! {
        unsafe impl #impl_generics ::#crate_path::Bundle for #ident #type_generics
        #where_clause
        {
            fn types() -> ::#crate_path::TypeSet {
                let iter = ::core::iter::empty();

                #(
                    let types = #types;
                    let iter = iter.chain(types.iter());
                )*

                ::#crate_path::TypeSet::from_iter(iter)
            }

            #[allow(unused)]
            fn take(mut self, writer: &mut ::#crate_path::BundleWriter<'_>) {
                #(#takes)*
            }
        }
    }
    .into()
}

struct DeriveBundle {
    ident: Ident,
    generics: Generics,
    fields: Fields,
    crate_path: Path,
}

enum FieldIdent {
    Named(Ident),
    Indexed(Literal),
}

impl Parse for DeriveBundle {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput { ident, generics, data, .. } = input.parse()?;
        let Data::Struct(DataStruct { fields, .. }) = data else {
            return Err(input.error("`Bundle` can only be derived for structs"));
        };
        let crate_path = crate_path()?;

        Ok(Self { ident, generics, fields, crate_path })
    }
}

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Named(ident) => ident.to_tokens(tokens),
            Self::Indexed(literal) => literal.to_tokens(tokens),
        }
    }
}
