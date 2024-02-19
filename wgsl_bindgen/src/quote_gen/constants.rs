use proc_macro2::Ident;

/// This mod is used such that all the mods in the out can reference this from anywhere
pub(crate) const MOD_REFERENCE_ROOT: &'static str = "_root";

pub(crate) fn mod_reference_root() -> Ident {
  unsafe { syn::parse_str(MOD_REFERENCE_ROOT).unwrap_unchecked() }
}
