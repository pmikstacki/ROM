use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, LitInt, LitStr, Path, parse_macro_input, spanned::Spanned};

pub(crate) fn derive(input: TokenStream, model: Model) -> TokenStream {
    expand(parse_macro_input!(input as DeriveInput), model)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Model {
    Resource,
    Input,
}
fn expand(input: DeriveInput, model: Model) -> syn::Result<proc_macro2::TokenStream> {
    let label = if model == Model::Resource {
        "Resource"
    } else {
        "Input"
    };
    let attribute = if model == Model::Resource {
        "resource"
    } else {
        "input"
    };
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            name.span(),
            format!("{label} requires concrete fields"),
        ));
    }
    let mut kind = None;
    let mut version = None;
    let mut crate_seen = false;
    let mut facade: Path = syn::parse_quote!(::rom);
    for attr in &input.attrs {
        if attr.path().is_ident("serde") {
            return Err(syn::Error::new(
                attr.span(),
                format!("{label} codec rejects independent serde configuration"),
            ));
        }
        if attr.path().is_ident(attribute) {
            attr.parse_nested_meta(|m| {
                if m.path.is_ident("version") {
                    if model == Model::Input {
                        return Err(m.error("Input does not support version"));
                    }
                    if version.is_some() {
                        return Err(m.error("duplicate version option"));
                    }
                    let value = m.value()?;
                    let literal: LitInt = value.parse().map_err(|_| {
                        syn::Error::new(value.span(), "version must be a positive u32 integer")
                    })?;
                    let parsed = literal
                        .base10_parse::<u32>()
                        .ok()
                        .filter(|v| *v != 0)
                        .ok_or_else(|| {
                            syn::Error::new(
                                literal.span(),
                                "version must be a positive u32 integer",
                            )
                        })?;
                    version = Some(parsed);
                    return Ok(());
                }
                let v: LitStr = m.value()?.parse()?;
                if model == Model::Resource && m.path.is_ident("name") && kind.is_none() {
                    kind = Some(v);
                    Ok(())
                } else if m.path.is_ident("crate") && !crate_seen {
                    crate_seen = true;
                    facade = v.parse()?;
                    Ok(())
                } else {
                    Err(m.error(if model == Model::Resource {
                        "expected one name, version or crate option"
                    } else {
                        "expected one crate option"
                    }))
                }
            })?;
        }
    }
    let version = version.unwrap_or(1);
    let kind = if model == Model::Input {
        Some(LitStr::new("input", name.span()))
    } else {
        kind
    }
    .ok_or_else(|| syn::Error::new(name.span(), "Resource needs #[resource(name = \"kind\")]"))?;
    let fields = match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(f) => f.named,
            _ => {
                return Err(syn::Error::new(
                    name.span(),
                    format!("{label} needs named fields"),
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                name.span(),
                format!("{label} needs a struct"),
            ));
        }
    };
    let mut names = std::collections::BTreeSet::new();
    let mut wire_names = vec![];
    let mut descriptors = vec![];
    let mut encodes = vec![];
    let mut decodes = vec![];
    let mut selectors = vec![];
    let mut field_codecs = vec![];
    for f in fields {
        let id = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        let rust_name = id.to_string();
        let default_name = if model == Model::Input {
            rust_name.trim_start_matches("r#")
        } else {
            &rust_name
        };
        let mut wire = LitStr::new(default_name, id.span());
        let mut renamed = false;
        for a in &f.attrs {
            if a.path().is_ident("serde") {
                return Err(syn::Error::new(
                    a.span(),
                    format!("{label} field rejects independent serde configuration"),
                ));
            }
            if a.path().is_ident(attribute) {
                a.parse_nested_meta(|m| {
                    if m.path.is_ident("rename") && !renamed {
                        renamed = true;
                        wire = m.value()?.parse()?;
                        Ok(())
                    } else {
                        Err(m.error("expected one rename option"))
                    }
                })?;
            }
        }
        if wire.value().is_empty() || !names.insert(wire.value()) {
            return Err(syn::Error::new(
                wire.span(),
                format!("duplicate or empty {label} field name"),
            ));
        }
        wire_names.push(wire.clone());
        descriptors.push(quote_spanned!(ty.span()=> #facade::FieldDescriptor { name:#wire.into(), shape:<#ty as #facade::Field>::shape() }));
        encodes.push(quote_spanned!(ty.span()=> if <#ty as #facade::Field>::is_present(&self.#id) { map.insert(#wire.into(),<#ty as #facade::Field>::encode(&self.#id)); }));
        let decode = if model == Model::Input {
            quote_spanned!(ty.span()=> #facade::__private::decode_input_member::<#ty>(map.remove(#wire)))
        } else {
            quote_spanned!(ty.span()=> match map.remove(#wire) { Some(value) => <#ty as #facade::Field>::decode(value), None => <#ty as #facade::Field>::decode_missing() })
        };
        decodes.push(quote_spanned!(ty.span()=> #id: (#decode).map_err(|_|#facade::Error::invalid(#kind,#wire))?));
        field_codecs.push(quote_spanned!(ty.span()=> #wire => <#ty as #facade::Field>::decode(value).map(|decoded|<#ty as #facade::Field>::encode(&decoded)),));
        let sel = format_ident!("{}_field", id);
        let selector_doc = format!(
            "Select the `{}` field for a typed Resource query.",
            wire.value()
        );
        selectors.push(quote_spanned!(ty.span()=> #[doc = #selector_doc] #[allow(dead_code)] pub fn #sel()->#facade::FieldRef<Self,#ty> { #facade::FieldRef::new(#wire) }));
    }
    let encode =
        quote! { let mut map=#facade::Map::new(); #(#encodes)* #facade::Value::Object(map) };
    let decode = quote! {
        let #facade::Value::Object(mut map)=value else { return Err(#facade::Error::invalid(#kind,"$")); };
        let result=Self { #(#decodes),* };
        if !map.is_empty() { return Err(#facade::Error::invalid(#kind,"unknown field")); }
        Ok(result)
    };
    if model == Model::Input {
        return Ok(quote! {
            impl #facade::Input for #name {
                fn field_names()-> &'static [&'static str] { &[#(#wire_names),*] }
                fn encode(&self)->#facade::Value { #encode }
                fn decode(value:#facade::Value)->#facade::Result<Self> { #decode }
            }
        });
    }
    Ok(quote! {
        impl #facade::Resource for #name {
            const KIND:&'static str=#kind;
            fn descriptor()->#facade::Descriptor { #facade::Descriptor {kind:Self::KIND.into(),version:#version,fields:vec![#(#descriptors),*]} }
            fn normalize_field(name:&str,value:#facade::Value)->#facade::Result<#facade::Value> {
                match name { #(#field_codecs)* _=>Err(#facade::Error::invalid(Self::KIND,name)) }
            }
            fn encode(&self)->#facade::Value { #encode }
            fn decode(value:#facade::Value)->::std::result::Result<Self,#facade::Error> { #decode }
        }
        impl #name { #(#selectors)* }
    })
}
