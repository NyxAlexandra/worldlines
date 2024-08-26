use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Ident};

#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, generics, data, .. } =
        parse_macro_input!(input as DeriveInput);
    let Data::Struct(DataStruct { fields, .. }) = data else {
        panic!("`Bundle` can only be implemented on structs");
    };

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
                ::archetypal_ecs::TypeData::of::<#ty>(),
                ::core::ptr::NonNull::from(&mut self.#ident).cast(),
            )
        }
    });

    quote! {
        unsafe impl #impl_generics ::archetypal_ecs::Bundle for #ident #type_generics
        #where_clause
        {
            type TakeIter = <
                [(::archetypal_ecs::TypeData, ::core::ptr::NonNull<u8>); #len]
                as ::core::iter::IntoIterator
            >::IntoIter;

            fn types() -> ::archetypal_ecs::TypeSet {
                let mut out = ::archetypal_ecs::TypeSet::new();

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
