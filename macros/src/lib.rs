use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::ToTokens;
use syn::Ident;

mod bundle;
mod component;

#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component::derive(input)
}

#[proc_macro_derive(Bundle, attributes(bundle))]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    bundle::derive(input)
}

fn crate_path() -> syn::Result<syn::Path> {
    let crate_path = option_env!("WORLDLINES_PATH").unwrap_or("worldlines");

    syn::parse_str(crate_path)
}

enum FieldIdent {
    Named(Ident),
    Indexed(Literal),
}

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Named(ident) => ident.to_tokens(tokens),
            Self::Indexed(literal) => literal.to_tokens(tokens),
        }
    }
}
