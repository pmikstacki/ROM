//! Throwaway derive: generate only the same trait implementation humans can write.
use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{quote, quote_spanned};
use std::collections::BTreeMap;
use syn::{
    parse_macro_input, parse_quote_spanned, spanned::Spanned, Data, DeriveInput, Fields, LitStr,
    Path,
};

#[proc_macro_derive(Resource, attributes(resource))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> syn::Result<Tokens> {
    let ident = &input.ident;
    let context = format!("resource {ident}");
    let mut name: Option<LitStr> = None;
    let mut crate_path: Option<Path> = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("resource") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if name.is_some() {
                    return Err(meta.error(format!("{context}: duplicate name option")));
                }
                let literal: LitStr = meta
                    .value()?
                    .parse()
                    .map_err(|_| meta.error(format!("{context}: name must be a string literal")))?;
                if literal.value().is_empty() {
                    return Err(syn::Error::new(
                        literal.span(),
                        format!("{context}: name must not be empty"),
                    ));
                }
                name = Some(literal);
                Ok(())
            } else if meta.path.is_ident("crate") {
                if crate_path.is_some() {
                    return Err(meta.error(format!("{context}: duplicate crate path option")));
                }
                let literal: LitStr = meta.value()?.parse()?;
                crate_path = Some(literal.parse().map_err(|_| {
                    syn::Error::new(
                        literal.span(),
                        format!("{context}: crate must contain a Rust path, for example ::rom"),
                    )
                })?);
                Ok(())
            } else {
                Err(meta.error(format!("{context}: unknown option; use name or crate")))
            }
        })?;
    }
    let name = name.ok_or_else(|| {
        syn::Error::new(
            ident.span(),
            format!("{context}: add #[resource(name = \"stable-name\")]"),
        )
    })?;
    let facade = crate_path.unwrap_or_else(|| syn::parse_quote!(::resource_contract));
    let named = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("{context}: only structs with named fields are supported"),
                ))
            }
        },
        _ => {
            return Err(syn::Error::new(
                ident.span(),
                format!("{context}: only structs with named fields are supported"),
            ))
        }
    };
    let mut names = BTreeMap::new();
    let mut descriptors = Vec::new();
    let mut validations = Vec::new();
    let mut generics = input.generics.clone();
    for field in named {
        let field_ident = field.ident.as_ref().expect("named field");
        let field_context = format!("{context}, field {field_ident}");
        let mut rename: Option<LitStr> = None;
        for attr in &field.attrs {
            if !attr.path().is_ident("resource") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if !meta.path.is_ident("rename") {
                    return Err(meta.error(format!("{field_context}: unknown option; use rename")));
                }
                if rename.is_some() {
                    return Err(meta.error(format!("{field_context}: duplicate rename option")));
                }
                let literal: LitStr = meta.value()?.parse().map_err(|_| {
                    meta.error(format!("{field_context}: rename must be a string literal"))
                })?;
                if literal.value().is_empty() {
                    return Err(syn::Error::new(
                        literal.span(),
                        format!("{field_context}: rename must not be empty"),
                    ));
                }
                rename = Some(literal);
                Ok(())
            })?;
        }
        let external =
            rename.unwrap_or_else(|| LitStr::new(&field_ident.to_string(), field_ident.span()));
        if let Some(first_span) = names.insert(external.value(), external.span()) {
            let mut error = syn::Error::new(
                external.span(),
                format!(
                    "{context}: duplicate external field name `{}`",
                    external.value()
                ),
            );
            error.combine(syn::Error::new(
                first_span,
                "first field with this external name is here",
            ));
            return Err(error);
        }
        let ty = &field.ty;
        // Rust resolves aliases, imports and nested generics. The macro never classifies type names.
        // Concrete false where-bounds can blame #[derive] rather than the field.
        // For nongeneric resources the typed calls below enforce the same contract
        // while preserving a diagnostic at the actual field type.
        if !input.generics.params.is_empty() {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote_spanned!(ty.span()=>#ty: #facade::Field));
        }
        descriptors
            .push(quote_spanned! {ty.span()=> #facade::FieldDescriptor::of::<#ty>(#external)});
        validations.push(quote_spanned!{field_ident.span()=>#facade::validate_field::<#ty>(<Self as #facade::Resource>::NAME,#external,&self.#field_ident)?;});
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #facade::Resource for #ident #ty_generics #where_clause {
            const NAME: &'static str=#name;
            fn descriptor()->#facade::Descriptor {
                #facade::Descriptor {name:<Self as #facade::Resource>::NAME,fields: ::std::vec![#(#descriptors),*]}
            }
            fn validate(&self)->::std::result::Result<(),#facade::ValidationError> {
                #(#validations)*
                ::std::result::Result::Ok(())
            }
        }
    })
}
