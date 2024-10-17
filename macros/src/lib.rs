use proc_macro::TokenStream;
use syn::Path;

mod bundle;
mod component;

#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    bundle::derive(input)
}

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component::derive(input)
}

fn crate_path() -> syn::Result<Path> {
    let crate_path =
        option_env!("ARCHETYPAL_ECS_PATH").unwrap_or("archetypal_ecs");

    syn::parse_str(crate_path)
}
