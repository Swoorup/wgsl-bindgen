// https://github.com/LucentFlux/naga-to-tokenstream/blob/main/src/lib.rs#L26
pub fn module_to_source(
  module: &naga::Module,
) -> Result<String, naga::back::wgsl::Error> {
  // Clone since we sometimes modify things
  #[allow(unused_mut)]
  let mut module = module.clone();

  // If we minify, do the first pass before writing out
  #[cfg(feature = "minify")]
  {
    wgsl_minifier::minify_module(&mut module);
  }

  // Mini validation to get module info
  let info = naga::valid::Validator::new(
    naga::valid::ValidationFlags::empty(),
    naga::valid::Capabilities::all(),
  )
  .validate(&module);

  // Write to wgsl
  let info = info.unwrap();
  let src = naga::back::wgsl::write_string(
    &module,
    &info,
    naga::back::wgsl::WriterFlags::empty(),
  )?;

  // Remove whitespace if minifying
  #[cfg(feature = "minify")]
  let src = wgsl_minifier::minify_wgsl_source(&src);

  return Ok(src);
}
