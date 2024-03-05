# Rough ideas. 

* Treat `_pad` as padding members and automatically initiallize it in init struct
* Allow injecting dynamic shader defines at runtime
* Allow generation directly from source files.
* proc_macro as an option alongside of build.rs. We need proc_macro::tracked* feature?
* per struct field replacement? Specify like `UniformBuffer.player_position` => `MyDVec4`.
  This appears necessary as naga backend doesn't support custom alignment in struct members. 
  i.e we can't have a struct which is not a mutliple of 16 bytes inside a uniform buffer struct. 
  But we can have built in types like vec2, vec4, etc.

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
