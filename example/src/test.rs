#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, bytemuck::Zeroable, encase::ShaderType)]
pub struct MatricesF32_E {
    pub a: [[f32; 4]; 4],
    pub b: [[f32; 4]; 3],
    pub c: [[f32; 4]; 2],
    pub d: [[f32; 3]; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LightStorage<const N: usize> {
    s: f32,
    point: [f32; N],
}

unsafe impl<const N: usize> bytemuck::Pod for LightStorage<N> {}
unsafe impl<const N: usize> bytemuck::Zeroable for LightStorage<N> {}

#[cfg(test)]
mod tests {
    use super::LightStorage;

    fn to_f32_buffer(byte_buffer: &[u8]) -> &[f32] {
        let f32_buffer: &[f32] = unsafe {
            std::slice::from_raw_parts(byte_buffer.as_ptr() as *const f32, byte_buffer.len() / 4)
        };
        f32_buffer
    }

    #[test]
    fn check_generated_buffer_match() {
        // let mut m_2 = MatricesF32::default();
        // m_2.a = [
        //     [1.0, 2.0, 3.0, 4.0],
        //     [5.0, 6.0, 7.0, 8.0],
        //     [9.0, 10.0, 11.0, 12.0],
        //     [13.0, 14.0, 15.0, 16.0],
        // ];
        // m_2.b = [
        //     [17.0, 18.0, 19.0, 20.0],
        //     [21.0, 22.0, 23.0, 24.0],
        //     [25.0, 26.0, 27.0, 28.0],
        // ];
        // m_2.c = [[29.0, 30.0, 31.0, 32.0], [33.0, 34.0, 35.0, 36.0]];
        // m_2.d = [
        //     [37.0, 38.0, 39.0],
        //     [40.0, 41.0, 42.0],
        //     [43.0, 44.0, 45.0],
        //     [46.0, 47.0, 48.0],
        // ];

        // let m_1 = MatricesF32_E {
        //     a: [
        //         [1.0, 2.0, 3.0, 4.0],
        //         [5.0, 6.0, 7.0, 8.0],
        //         [9.0, 10.0, 11.0, 12.0],
        //         [13.0, 14.0, 15.0, 16.0],
        //     ],
        //     b: [
        //         [17.0, 18.0, 19.0, 20.0],
        //         [21.0, 22.0, 23.0, 24.0],
        //         [25.0, 26.0, 27.0, 28.0],
        //     ],
        //     c: [[29.0, 30.0, 31.0, 32.0], [33.0, 34.0, 35.0, 36.0]],
        //     d: [
        //         [37.0, 38.0, 39.0],
        //         [40.0, 41.0, 42.0],
        //         [43.0, 44.0, 45.0],
        //         [46.0, 47.0, 48.0],
        //     ],
        // };

        // let uniform_bytemuck = bytemuck::bytes_of(&m_2);
        // let mut encase_buffer = encase::UniformBuffer::new(Vec::new());
        // encase_buffer.write(&m_1).unwrap();
        // let byte_butter = encase_buffer.into_inner();

        // assert_eq!(
        //     to_f32_buffer(uniform_bytemuck)[12..],
        //     to_f32_buffer(byte_butter.as_slice())[12..]
        // );
    }

    #[test]
    fn test_rst() {
        let l = LightStorage {
            s: 100f32,
            point: [1f32, 2f32],
        };
        let buf = bytemuck::bytes_of(&l);

        println!("lightStorage buffer {:?}", to_f32_buffer(&buf));
    }
}
