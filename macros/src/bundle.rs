use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::quote;
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

use crate::{crate_path, FieldIdent};

pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveBundle { ident, generics, fields, crate_path } =
        parse_macro_input!(input);

    let (impl_generics, type_generics, where_clause) =
        generics.split_for_impl();

    // `(B::components(builder), self.field.write(writer))`
    let (components_bodies, write_bodies): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .enumerate()
        .map(|(i, Field { ident, ty, .. })| {
            (
                quote! {
                    #ty::components(builder)
                },
                {
                    let field_ident = ident.map(FieldIdent::Named).unwrap_or(
                        FieldIdent::Indexed(Literal::usize_unsuffixed(i)),
                    );

                    quote! {
                        self.#field_ident.write(writer);
                    }
                },
            )
        })
        .unzip();

    quote! {
        #[automatically_derived]
        unsafe impl #impl_generics ::#crate_path::component::Bundle for #ident #type_generics
        #where_clause
        {
            fn components(builder: &mut ::#crate_path::component::ComponentSetBuilder<'_>) {
                #(#components_bodies);*
            }

            #[allow(unused)]
            fn write(mut self, writer: &mut ::#crate_path::component::ComponentWriter<'_, '_>) {
                #(#write_bodies);*
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
