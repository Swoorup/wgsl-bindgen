# Rough ideas. 

* Support `naga-oil` instead of `naga`. (Not sure how to achieve this)
  * Extract module to a seperate rust file?
  * Probably work out the dependency tree, and generate the base file
* Skip zsts padding for bytemuck generation
  * Need to return the size of the rust type generated along with the token stream
  * If the rust type size + current offset = next offset skip padding.
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

* Add a way to generate variant types in wgsl. 
  * Maybe a seperate binary that accepts rust source. 
  * Generates accessors, setters in wgsl
  * Struct fields are efficiently utilised.