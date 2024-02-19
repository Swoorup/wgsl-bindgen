#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, bytemuck::Zeroable, bytemuck::Pod, encase::ShaderType)]
pub struct TestStruct {
    pub a: f32,
    pub b: glam::Vec2, // encase correctly knows this is 8 bytes aligned
}

#[cfg(test)]
mod tests {
    use crate::TestStruct;

    fn to_f32_buffer(byte_buffer: &[u8]) -> &[f32] {
        let f32_buffer: &[f32] = unsafe {
            std::slice::from_raw_parts(byte_buffer.as_ptr() as *const f32, byte_buffer.len() / 4)
        };
        f32_buffer
    }

    fn get_buffer_encoded_by_encase<T>(data: T) -> Vec<u8>
    where
        T: encase::ShaderType + encase::internal::WriteInto,
    {
        let mut encase_buffer = encase::UniformBuffer::new(Vec::new());
        encase_buffer.write(&data).unwrap();
        encase_buffer.into_inner()
    }

    #[test]
    #[ignore]
    fn check_generated_buffer_match() {
        let mut s1 = TestStruct::default();
        s1.a = 100f32;
        s1.b = glam::vec2(1f32, 2f32);

        let uniform_bytemuck = bytemuck::bytes_of(&s1);
        let encase_buffer = get_buffer_encoded_by_encase(&s1);

        assert_eq!(
            to_f32_buffer(uniform_bytemuck)[..],
            to_f32_buffer(encase_buffer.as_slice())[..]
        );
    }
}
