//! Parse advisory Resource authoring without changing descriptor or codec identity.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{LitStr, Path, Type, meta::ParseNestedMeta};

#[derive(Default)]
pub(crate) struct Presentation {
    label: Option<LitStr>,
    title_field: Option<LitStr>,
    settings: Option<(LitStr, LitStr)>,
    groups: Vec<(LitStr, LitStr)>,
    fields: Vec<(LitStr, FieldHints, Type)>,
}
#[derive(Default)]
pub(crate) struct FieldHints {
    label: Option<LitStr>,
    help: Option<LitStr>,
    group: Option<LitStr>,
}
fn text(meta: &ParseNestedMeta<'_>, target: &mut Option<LitStr>, limit: usize) -> syn::Result<()> {
    if target.is_some() {
        return Err(meta.error("duplicate presentation option"));
    }
    let value: LitStr = meta.value()?.parse()?;
    if value.value().is_empty() || value.value().len() > limit {
        return Err(syn::Error::new(
            value.span(),
            format!("presentation text must contain 1 to {limit} UTF-8 bytes"),
        ));
    }
    *target = Some(value);
    Ok(())
}
fn pair(meta: &ParseNestedMeta<'_>, key: &str) -> syn::Result<(LitStr, LitStr)> {
    let mut name = None;
    let mut label = None;
    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident(key) {
            text(&nested, &mut name, 256)
        } else if nested.path.is_ident("label") {
            text(&nested, &mut label, 256)
        } else {
            Err(nested.error(format!("expected {key} or label presentation option")))
        }
    })?;
    Ok((
        name.ok_or_else(|| meta.error(format!("presentation needs {key}")))?,
        label.ok_or_else(|| meta.error("presentation needs label"))?,
    ))
}
impl Presentation {
    pub(crate) fn parse(&mut self, meta: &ParseNestedMeta<'_>) -> syn::Result<bool> {
        if meta.path.is_ident("label") {
            text(meta, &mut self.label, 256)?;
        } else if meta.path.is_ident("title_field") {
            text(meta, &mut self.title_field, 256)?;
        } else if meta.path.is_ident("settings") {
            if self.settings.is_some() {
                return Err(meta.error("duplicate settings presentation option"));
            }
            self.settings = Some(pair(meta, "group")?);
        } else if meta.path.is_ident("group") {
            let group = pair(meta, "name")?;
            if self
                .groups
                .iter()
                .any(|(name, _)| name.value() == group.0.value())
            {
                return Err(syn::Error::new(
                    group.0.span(),
                    "duplicate presentation group",
                ));
            }
            self.groups.push(group);
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    pub(crate) fn field(&mut self, name: LitStr, hints: FieldHints, ty: Type) {
        self.fields.push((name, hints, ty));
    }
    pub(crate) fn expand(&self, facade: &Path) -> syn::Result<TokenStream> {
        if self.groups.len() > 1024 {
            return Err(syn::Error::new(
                self.groups[1024].0.span(),
                "presentation supports at most 1024 groups",
            ));
        }
        let populated: Vec<_> = self
            .fields
            .iter()
            .filter(|(_, hints, _)| hints.present())
            .collect();
        if populated.len() > 1024 {
            return Err(syn::Error::new(
                populated[1024].0.span(),
                "presentation supports at most 1024 field hints",
            ));
        }
        for (_, hints, _) in &self.fields {
            if let Some(group) = &hints.group
                && !self
                    .groups
                    .iter()
                    .any(|(name, _)| name.value() == group.value())
            {
                return Err(syn::Error::new(group.span(), "unknown presentation group"));
            }
        }
        if let Some(title) = &self.title_field {
            let (_, _, ty) = self
                .fields
                .iter()
                .find(|(name, _, _)| name.value() == title.value())
                .ok_or_else(|| {
                    syn::Error::new(
                        title.span(),
                        "title_field must name a canonical Resource field",
                    )
                })?;
            if known_non_text(ty) {
                return Err(syn::Error::new(
                    title.span(),
                    "title_field requires a built-in string field",
                ));
            }
        }
        if self.label.is_none()
            && self.title_field.is_none()
            && self.settings.is_none()
            && self.groups.is_empty()
            && populated.is_empty()
        {
            return Ok(TokenStream::new());
        }
        let label = optional_text(&self.label);
        let title = optional_text(&self.title_field);
        let settings = match &self.settings {
            Some((group, label)) => {
                quote!(Some(#facade::SettingsPresentation { group: #group.into(), label: #label.into() }))
            }
            None => quote!(None),
        };
        let groups = self.groups.iter().map(|(name, label)| quote!(#facade::PresentationGroup { name: #name.into(), label: #label.into() }));
        let fields = populated.iter().map(|(name, hints, _)| {
            let label = optional_text(&hints.label);
            let help = optional_text(&hints.help);
            let group = optional_text(&hints.group);
            quote!((#name.into(), #facade::FieldPresentation { label: #label, help: #help, group: #group }))
        });
        Ok(quote! {
            fn presentation() -> Option<#facade::ResourcePresentation> {
                Some(#facade::ResourcePresentation { label: #label, title_field: #title, settings: #settings, groups: vec![#(#groups),*], fields: [#(#fields),*].into() })
            }
        })
    }
}
impl FieldHints {
    pub(crate) fn parse(&mut self, meta: &ParseNestedMeta<'_>) -> syn::Result<bool> {
        if meta.path.is_ident("label") {
            text(meta, &mut self.label, 256)?;
        } else if meta.path.is_ident("help") {
            text(meta, &mut self.help, 2048)?;
        } else if meta.path.is_ident("group") {
            text(meta, &mut self.group, 256)?;
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    fn present(&self) -> bool {
        self.label.is_some() || self.help.is_some() || self.group.is_some()
    }
}
fn optional_text(value: &Option<LitStr>) -> TokenStream {
    match value {
        Some(value) => quote!(Some(#value.into())),
        None => quote!(None),
    }
}
// Unknown Field implementations, aliases and wrapped types remain registration's responsibility.
// Bare primitive names can be shadowed by aliases, so only explicit primitive paths qualify.
// In particular, a syntactic String cannot prove that a custom codec is absent.
fn known_non_text(ty: &Type) -> bool {
    match ty {
        Type::Path(ty)
            if ty.qself.is_none()
                && ty.path.leading_colon.is_some()
                && ty.path.segments.len() == 3
                && (ty.path.segments[0].ident == "core" || ty.path.segments[0].ident == "std")
                && ty.path.segments[1].ident == "primitive" =>
        {
            let id = ty.path.segments[2].ident.to_string();
            matches!(
                id.as_str(),
                "bool"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "f32"
                    | "f64"
                    | "char"
            )
        }
        Type::Group(group) => known_non_text(&group.elem),
        Type::Paren(paren) => known_non_text(&paren.elem),
        _ => false,
    }
}
