# Rough ideas. 

* Allow to use naga-oil in the final generated output.
  * Allows injecting dynamic shader defines at runtime
* Generate from string instead of source files?
* proc_macro as an option instead of build.rs. We need proc_macro::tracked* feature?

* Use struct like this instead directly using the array.
  * ```rust
    #[repr(C)]
    struct PaddedField<const N: usize, T> {
        field: T,
        padding: [u8; N],
    }

    impl<const N: usize, T> PaddedField<N, T> {
      pub fn new(value: T) -> Self {
        Self {
          field: value,
          padding: [0; N],
        }
      }
    }
    ```

  - https://github.com/rust-lang/rust/issues/73557
  - https://www.reddit.com/r/rust/comments/16e18kp/how_to_set_alignment_of_individual_struct_members/

* Add a way to encode variant types in wgsl?. 
  * Maybe a seperate binary that accepts rust source. 
  * Generates accessors, setters in wgsl
  * Struct fields are efficiently utilised.
