//! Structural Resource derive. Descriptor, codec and selectors come from one field list.
use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, LitStr, Path, parse_macro_input, spanned::Spanned};

#[proc_macro_derive(Resource, attributes(resource, serde))]
pub fn resource(input: TokenStream) -> TokenStream {
    expand(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            name.span(),
            "probe Resource requires concrete fields",
        ));
    }
    let mut kind = None;
    let mut facade: Path = syn::parse_quote!(::rom);
    for attr in &input.attrs {
        if attr.path().is_ident("serde") {
            return Err(syn::Error::new(
                attr.span(),
                "Resource codec rejects independent serde configuration",
            ));
        }
        if attr.path().is_ident("resource") {
            attr.parse_nested_meta(|m| {
                let v: LitStr = m.value()?.parse()?;
                if m.path.is_ident("name") && kind.is_none() {
                    kind = Some(v);
                    Ok(())
                } else if m.path.is_ident("crate") {
                    facade = v.parse()?;
                    Ok(())
                } else {
                    Err(m.error("expected one name or crate option"))
                }
            })?;
        }
    }
    let kind = kind.ok_or_else(|| {
        syn::Error::new(name.span(), "Resource needs #[resource(name = \"kind\")]")
    })?;
    let fields = match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(f) => f.named,
            _ => return Err(syn::Error::new(name.span(), "Resource needs named fields")),
        },
        _ => return Err(syn::Error::new(name.span(), "Resource needs a struct")),
    };
    let mut names = std::collections::BTreeSet::new();
    let mut descriptors = vec![];
    let mut encodes = vec![];
    let mut decodes = vec![];
    let mut selectors = vec![];
    let mut field_codecs = vec![];
    for f in fields {
        let id = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        let mut wire = LitStr::new(&id.to_string(), id.span());
        for a in &f.attrs {
            if a.path().is_ident("serde") {
                return Err(syn::Error::new(
                    a.span(),
                    "Resource field rejects independent serde configuration",
                ));
            }
            if a.path().is_ident("resource") {
                a.parse_nested_meta(|m| {
                    if m.path.is_ident("rename") {
                        wire = m.value()?.parse()?;
                        Ok(())
                    } else {
                        Err(m.error("expected rename"))
                    }
                })?;
            }
        }
        if wire.value().is_empty() || !names.insert(wire.value()) {
            return Err(syn::Error::new(
                wire.span(),
                "duplicate or empty Resource field name",
            ));
        }
        descriptors.push(quote_spanned!(ty.span()=> #facade::FieldDescriptor { name:#wire.into(), shape:<#ty as #facade::Field>::shape() }));
        encodes.push(quote_spanned!(ty.span()=> if <#ty as #facade::Field>::is_present(&self.#id) { map.insert(#wire.into(),<#ty as #facade::Field>::encode(&self.#id)); }));
        decodes.push(quote_spanned!(ty.span()=> #id:match map.remove(#wire) { Some(value) => <#ty as #facade::Field>::decode(value), None => <#ty as #facade::Field>::decode_missing() }.map_err(|_|#facade::Error::invalid(Self::KIND,#wire))?));
        field_codecs.push(quote_spanned!(ty.span()=> #wire => <#ty as #facade::Field>::decode(value).map(|decoded|<#ty as #facade::Field>::encode(&decoded)),));
        let sel = format_ident!("{}_field", id);
        let selector_doc = format!(
            "Select the `{}` field for a typed Resource query.",
            wire.value()
        );
        selectors.push(quote_spanned!(ty.span()=> #[doc = #selector_doc] #[allow(dead_code)] pub fn #sel()->#facade::FieldRef<Self,#ty> { #facade::FieldRef::new(#wire) }));
    }
    Ok(quote! {
        impl #facade::Resource for #name {
            const KIND:&'static str=#kind;
            fn descriptor()->#facade::Descriptor { #facade::Descriptor {kind:Self::KIND.into(),version:1,fields:vec![#(#descriptors),*]} }
            fn normalize_field(name:&str,value:#facade::Value)->#facade::Result<#facade::Value> {
                match name { #(#field_codecs)* _=>Err(#facade::Error::invalid(Self::KIND,name)) }
            }
            fn encode(&self)->#facade::Value { let mut map=#facade::Map::new(); #(#encodes)* #facade::Value::Object(map) }
            fn decode(value:#facade::Value)->::std::result::Result<Self,#facade::Error> {
                let mut map=value.as_object().cloned().ok_or_else(||#facade::Error::invalid(Self::KIND,"$"))?;
                let result=Self { #(#decodes),* };
                if !map.is_empty() { return Err(#facade::Error::invalid(Self::KIND,"unknown field")); }
                Ok(result)
            }
        }
        impl #name { #(#selectors)* }
    })
}
