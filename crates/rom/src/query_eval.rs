//! Private canonical query planning, comparison and storage selection.
mod comparison;
mod normalization;
mod read;

pub(crate) use normalization::{Plan, make_anchor, normalize};
