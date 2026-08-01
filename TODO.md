# Rough ideas. 

* Allow injecting dynamic shader defines at build-time (we already have a runtime mechanism from generation)

* Allow generation directly from shader strings.

* proc_macro as an option alongside of build.rs.

* Add a way to encode variant types in wgsl?. 
  * Maybe a separate binary that accepts rust source. 
  * Generates accessors, setters in wgsl
  * Struct fields are efficiently utilised.
