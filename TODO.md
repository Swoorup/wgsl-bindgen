# Rough ideas. 

* Treat `_pad` as padding members and automatically initiallize it in init struct. 
  This requires moving the pad field as a member in struct member entry info to a member entry variant type?
* Allow injecting dynamic shader defines at build-time (we already have a runtime mechanism from generation)
* Allow generation directly from shader strings.
* proc_macro as an option alongside of build.rs. We need proc_macro::tracked* feature?
* Use something like derivative and use MaybeUninit for padded fields

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
