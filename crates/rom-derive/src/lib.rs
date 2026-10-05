//! Structural Resource derive. Descriptor, codec and selectors share one field list.
use proc_macro::TokenStream;
mod expansion;
mod presentation;
use expansion::Model;

// Rust requires proc-macro entry points at the crate root. Expansion lives in its module.
#[proc_macro_derive(Resource, attributes(resource, serde))]
pub fn resource(input: TokenStream) -> TokenStream {
    expansion::derive(input, Model::Resource)
}

/// Generate a strict action object codec without registering a Resource.
#[proc_macro_derive(Input, attributes(input, serde))]
pub fn input(input: TokenStream) -> TokenStream {
    expansion::derive(input, Model::Input)
}
