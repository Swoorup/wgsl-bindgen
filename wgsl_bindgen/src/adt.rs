//! Automatic extraction and code generation for fwgsl algebraic data types (ADTs).
//!
//! # Design
//!
//! fwgsl ADTs are lowered to `u32` discriminant values in WGSL (for simple enums)
//! or individual WGSL structs per constructor (for data-carrying variants).  Neither
//! encoding retains enough type information for wgsl-bindgen to detect them automatically
//! from the Naga IR.
//!
//! To bridge the gap, fwgsl's build.rs integration injects structured annotation
//! comments into the WGSL source before feeding it to wgsl-bindgen.  The annotation
//! format is a single line:
//!
//! ```text
//! // @fwgsl-adt: TypeName Variant0:tag0 Variant1:tag1:StructName1 ...
//! ```
//!
//! * `TypeName` — the name of the algebraic type (e.g. `Color`, `Shape`)
//! * `Variant:tag` — a tag-only variant (simple enum constructor, no payload)
//! * `Variant:tag:StructName` — a data-carrying variant; `StructName` is the WGSL struct
//!   (and corresponding Rust struct) that holds the variant's payload
//!
//! wgsl-bindgen reads these annotations from the raw WGSL source text and automatically
//! emits a matching Rust type without any additional configuration from the user.
//!
//! ## Generated Rust types
//!
//! ### Simple enums (all variants tag-only)
//!
//! ```rust,ignore
//! #[repr(u32)]
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! pub enum Color { Red = 0, Green = 1, Blue = 2 }
//!
//! impl TryFrom<u32> for Color { ... }
//! impl From<Color> for u32 { ... }
//! // bytemuck::Zeroable + Pod when discriminant 0 exists
//! ```
//!
//! ### Data-carrying enums (at least one variant with a struct payload)
//!
//! ```rust,ignore
//! #[derive(Debug, Clone, Copy)]
//! pub enum Shape {
//!     Circle(Circle),  // Circle is a WGSL struct emitted by fwgsl
//!     Rect(Rect),
//! }
//! impl Shape {
//!     pub fn tag(&self) -> u32 { match self { Self::Circle(_) => 0, Self::Rect(_) => 1 } }
//! }
//! ```

use std::collections::HashSet;

use proc_macro2::TokenStream;
use crate::qs::{format_ident, quote};
use crate::{WgslTypeSerializeStrategy};

// ─────────────────────────────────────────────
// Data model
// ─────────────────────────────────────────────

/// A single variant of an ADT annotated with `// @fwgsl-adt:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FwgslAdtVariant {
  /// Variant/constructor name (e.g. `Red`, `Circle`).
  pub name: String,
  /// The `u32` discriminant tag assigned by fwgsl.
  pub tag: u32,
  /// The WGSL struct name carrying this variant's payload, if any.
  /// `None` for tag-only (simple enum) constructors.
  pub struct_name: Option<String>,
}

/// A complete algebraic data type parsed from `// @fwgsl-adt:` annotations.
#[derive(Debug, Clone)]
pub(crate) struct FwgslAdtType {
  /// The type name (e.g. `Color`, `Shape`).
  pub name: String,
  pub variants: Vec<FwgslAdtVariant>,
}

impl FwgslAdtType {
  /// Returns `true` if every variant is tag-only (no struct payload).
  pub fn is_simple_enum(&self) -> bool {
    self.variants.iter().all(|v| v.struct_name.is_none())
  }
}

// ─────────────────────────────────────────────
// Parsing
// ─────────────────────────────────────────────

/// Parse all `// @fwgsl-adt:` annotation lines from a WGSL source string.
///
/// Lines that do not start with the magic prefix are silently ignored.
/// Malformed annotation lines are also silently skipped.
pub(crate) fn parse_fwgsl_adt_annotations(wgsl: &str) -> Vec<FwgslAdtType> {
  const PREFIX: &str = "// @fwgsl-adt:";

  let mut adts = Vec::new();

  for line in wgsl.lines() {
    let line = line.trim();
    if !line.starts_with(PREFIX) {
      continue;
    }

    let rest = line[PREFIX.len()..].trim();
    // rest = "TypeName Var0:tag0 Var1:tag1:StructName ..."
    let mut tokens = rest.split_whitespace();

    let type_name = match tokens.next() {
      Some(n) if !n.is_empty() => n.to_owned(),
      _ => continue,
    };

    let mut variants = Vec::new();
    for token in tokens {
      // token = "VarName:tag" or "VarName:tag:StructName"
      let parts: Vec<&str> = token.splitn(3, ':').collect();
      if parts.len() < 2 {
        continue;
      }
      let variant_name = parts[0].to_owned();
      let tag = match parts[1].parse::<u32>() {
        Ok(t) => t,
        Err(_) => continue,
      };
      let struct_name = parts.get(2).filter(|s| !s.is_empty()).map(|s| s.to_string());
      variants.push(FwgslAdtVariant { name: variant_name, tag, struct_name });
    }

    if !variants.is_empty() {
      adts.push(FwgslAdtType { name: type_name, variants });
    }
  }

  adts
}

// ─────────────────────────────────────────────
// Token generation
// ─────────────────────────────────────────────

/// Generate the Rust token stream for all ADTs from a WGSL entry, deduplicating
/// by type name across multiple calls.
///
/// Pass a mutable `seen_names` set to deduplicate ADTs that appear in more than
/// one shader entry point.
///
/// `module_name` is the Rust module path (e.g. `shape_compute`) used to qualify
/// struct type references in data-carrying enum variants.
pub(crate) fn adt_enum_tokens_dedup(
  adts: &[FwgslAdtType],
  module_name: &str,
  seen_names: &mut HashSet<String>,
  serialization_strategy: WgslTypeSerializeStrategy,
) -> TokenStream {
  let items: Vec<TokenStream> = adts
    .iter()
    .filter(|adt| seen_names.insert(adt.name.clone()))
    .map(|adt| generate_adt_tokens(adt, module_name, serialization_strategy))
    .collect();
  quote! { #(#items)* }
}

fn generate_adt_tokens(
  adt: &FwgslAdtType,
  module_name: &str,
  strategy: WgslTypeSerializeStrategy,
) -> TokenStream {
  if adt.is_simple_enum() {
    generate_simple_enum(adt, strategy)
  } else {
    generate_data_enum(adt, module_name)
  }
}

/// Generate a `#[repr(u32)]` simple enum with conversion traits.
fn generate_simple_enum(adt: &FwgslAdtType, strategy: WgslTypeSerializeStrategy) -> TokenStream {
  let enum_name = format_ident!("{}", adt.name);

  let variants: Vec<TokenStream> = adt
    .variants
    .iter()
    .map(|v| {
      let ident = format_ident!("{}", v.name);
      let tag = v.tag;
      quote! { #ident = #tag }
    })
    .collect();

  let try_from_arms: Vec<TokenStream> = adt
    .variants
    .iter()
    .map(|v| {
      let ident = format_ident!("{}", v.name);
      let tag = v.tag;
      quote! { #tag => ::core::result::Result::Ok(Self::#ident) }
    })
    .collect();

  // `bytemuck::Zeroable` is safe only when discriminant 0 is a valid variant
  // (zeroing the type produces a valid discriminant).
  //
  // `bytemuck::Pod` is intentionally NOT implemented for enums: it requires that
  // EVERY bit pattern is a valid value, which is never true for a `#[repr(u32)]` enum
  // with fewer than 2^32 variants.  Use `TryFrom<u32>` to safely convert raw GPU data.
  let bytemuck_impls = if strategy == WgslTypeSerializeStrategy::Bytemuck
    && adt.variants.iter().any(|v| v.tag == 0)
  {
    quote! {
      unsafe impl ::bytemuck::Zeroable for #enum_name {}
    }
  } else {
    quote! {}
  };

  quote! {
    /// Rust mirror of the fwgsl algebraic data type (ADT) of the same name.
    ///
    /// Each variant corresponds to a `u32` constructor tag emitted by the fwgsl compiler.
    /// Use [`TryFrom<u32>`] to decode a raw WGSL value and [`From<Self> for u32`] to
    /// write the enum into a WGSL-compatible buffer.
    ///
    /// This type is automatically generated from a `// @fwgsl-adt:` annotation in the
    /// shader source — no manual `WgslEnumDefinition` is required.
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum #enum_name {
      #(#variants),*
    }

    impl ::core::convert::TryFrom<u32> for #enum_name {
      type Error = u32;
      #[inline]
      fn try_from(v: u32) -> ::core::result::Result<Self, u32> {
        match v {
          #(#try_from_arms,)*
          other => ::core::result::Result::Err(other),
        }
      }
    }

    impl ::core::convert::From<#enum_name> for u32 {
      #[inline]
      fn from(e: #enum_name) -> u32 {
        e as u32
      }
    }

    #bytemuck_impls
  }
}

/// Generate a data-carrying Rust enum where each variant wraps the corresponding
/// WGSL struct (also generated by wgsl-bindgen from the WGSL source).
///
/// `module_name` is the Rust module path (e.g. `shape_compute`) that contains the
/// struct types.  The generated enum is placed at the crate root, so struct
/// references are qualified as `module_name::StructName`.
fn generate_data_enum(adt: &FwgslAdtType, module_name: &str) -> TokenStream {
  let enum_name = format_ident!("{}", adt.name);

  // Resolve a struct name to a fully-qualified Rust path.
  // If the struct lives in `shape_compute` the path is `shape_compute::Circle`.
  let qualify_struct = |sname: &str| -> proc_macro2::TokenStream {
    if module_name.is_empty() {
      let ident = format_ident!("{}", sname);
      quote! { #ident }
    } else {
      // Handle `::` separated module paths
      let parts: Vec<proc_macro2::Ident> = module_name
        .split("::")
        .chain(std::iter::once(sname))
        .map(|p| format_ident!("{}", p))
        .collect();
      quote! { #(#parts)::* }
    }
  };

  // For variants that have a struct payload, wrap it; for tag-only variants,
  // emit a unit variant.
  let variants: Vec<TokenStream> = adt
    .variants
    .iter()
    .map(|v| {
      let ident = format_ident!("{}", v.name);
      if let Some(ref sname) = v.struct_name {
        let struct_path = qualify_struct(sname);
        quote! { #ident(#struct_path) }
      } else {
        quote! { #ident }
      }
    })
    .collect();

  // tag() method arms
  let tag_arms: Vec<TokenStream> = adt
    .variants
    .iter()
    .map(|v| {
      let ident = format_ident!("{}", v.name);
      let tag = v.tag;
      if v.struct_name.is_some() {
        quote! { Self::#ident(_) => #tag }
      } else {
        quote! { Self::#ident => #tag }
      }
    })
    .collect();

  quote! {
    /// Rust mirror of the fwgsl algebraic data type (ADT) of the same name.
    ///
    /// Each variant either wraps the WGSL struct holding the constructor's payload or
    /// is a unit variant for tag-only constructors. Call [`Self::tag()`] to obtain the
    /// `u32` discriminant value used in the corresponding WGSL shader code.
    ///
    /// This type is automatically generated from a `// @fwgsl-adt:` annotation in the
    /// shader source — no manual `WgslEnumDefinition` is required.
    #[derive(Debug, Clone, Copy)]
    pub enum #enum_name {
      #(#variants),*
    }

    impl #enum_name {
      /// Returns the `u32` discriminant tag used by fwgsl in the corresponding WGSL shader.
      #[inline]
      pub fn tag(&self) -> u32 {
        match self {
          #(#tag_arms,)*
        }
      }
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_simple_enum() {
    let wgsl = "// @fwgsl-adt: Color Red:0 Green:1 Blue:2\nalias Color = u32;\n";
    let adts = parse_fwgsl_adt_annotations(wgsl);
    assert_eq!(adts.len(), 1);
    let adt = &adts[0];
    assert_eq!(adt.name, "Color");
    assert_eq!(adt.variants.len(), 3);
    assert_eq!(adt.variants[0], FwgslAdtVariant { name: "Red".into(), tag: 0, struct_name: None });
    assert_eq!(adt.variants[1], FwgslAdtVariant { name: "Green".into(), tag: 1, struct_name: None });
    assert_eq!(adt.variants[2], FwgslAdtVariant { name: "Blue".into(), tag: 2, struct_name: None });
    assert!(adt.is_simple_enum());
  }

  #[test]
  fn parse_data_carrying_enum() {
    let wgsl = "// @fwgsl-adt: Shape Circle:0:Circle Rect:1:Rect\n";
    let adts = parse_fwgsl_adt_annotations(wgsl);
    assert_eq!(adts.len(), 1);
    let adt = &adts[0];
    assert_eq!(adt.name, "Shape");
    assert!(!adt.is_simple_enum());
    assert_eq!(adt.variants[0].struct_name, Some("Circle".into()));
    assert_eq!(adt.variants[1].struct_name, Some("Rect".into()));
  }

  #[test]
  fn parse_mixed_enum() {
    // Mixed: some variants have data, some don't
    let wgsl = "// @fwgsl-adt: Event Tick:0 Resize:1:ResizeData Done:2\n";
    let adts = parse_fwgsl_adt_annotations(wgsl);
    assert_eq!(adts.len(), 1);
    let adt = &adts[0];
    assert!(!adt.is_simple_enum());
    assert_eq!(adt.variants[0].struct_name, None);
    assert_eq!(adt.variants[1].struct_name, Some("ResizeData".into()));
    assert_eq!(adt.variants[2].struct_name, None);
  }

  #[test]
  fn parse_multiple_adts() {
    let wgsl = "// @fwgsl-adt: Color Red:0 Green:1\n// @fwgsl-adt: Shape Circle:0:Circle\n";
    let adts = parse_fwgsl_adt_annotations(wgsl);
    assert_eq!(adts.len(), 2);
    assert_eq!(adts[0].name, "Color");
    assert_eq!(adts[1].name, "Shape");
  }

  #[test]
  fn parse_ignores_other_lines() {
    let wgsl = "struct Foo { x: f32 }\n// not an adt\n// @fwgsl-adt: Color Red:0\n";
    let adts = parse_fwgsl_adt_annotations(wgsl);
    assert_eq!(adts.len(), 1);
  }

  #[test]
  fn parse_malformed_lines_skipped() {
    let wgsl = "// @fwgsl-adt: \n// @fwgsl-adt: OnlyName\n// @fwgsl-adt: Bad bad_variant_no_colon\n";
    let adts = parse_fwgsl_adt_annotations(wgsl);
    // "OnlyName" has no variants → skipped; "Bad" has one malformed variant → skipped
    assert_eq!(adts.len(), 0);
  }
}
