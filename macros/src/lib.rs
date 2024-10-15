use proc_macro::TokenStream;
use proc_macro2::Span;
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

#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let DeriveBundle { ident, generics, fields, crate_path } = parse_macro_input!(input);

    let len = fields.len();
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    // insert the type of each field into the type set
    let insert_field_types = fields.iter().map(|Field { ty, .. }| {
        quote! { out.insert::<#ty>(); }
    });
    // `(TypeData::of::<T0>, NonNull::from(&mut self.t0).cast())`
    let field_tuples = fields.iter().enumerate().map(|(i, Field { ident, ty, .. })| {
        let ident = if let Some(ident) = ident {
            quote! { #ident }
        } else {
            let i = Ident::new(&i.to_string(), Span::call_site());

            quote! { #i }
        };

        quote! {
            (
                ::#crate_path::TypeData::of::<#ty>(),
                ::core::ptr::NonNull::from(&mut self.#ident).cast(),
            )
        }
    });

    quote! {
        unsafe impl #impl_generics ::#crate_path::Bundle for #ident #type_generics
        #where_clause
        {
            type TakeIter = <
                [(::#crate_path::TypeData, ::core::ptr::NonNull<u8>); #len]
                as ::core::iter::IntoIterator
            >::IntoIter;

            fn types() -> ::#crate_path::TypeSet {
                let mut out = ::#crate_path::TypeSet::new();

                #(#insert_field_types)*

                out
            }

            fn take(mut self, f: impl FnOnce(Self::TakeIter)) {
                f([#(#field_tuples),*].into_iter());

                ::core::mem::forget(self);
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

        let crate_path = option_env!("ARCHETYPAL_ECS_PATH").unwrap_or("archetypal_ecs");
        let crate_path = syn::parse_str(crate_path)?;

        Ok(Self { ident, generics, fields, crate_path })
    }
}
