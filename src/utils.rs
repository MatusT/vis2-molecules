//!
//! Unassigned helpful utilities.
//!

pub fn load_glsl(code: &[u8]) -> Vec<u32> {
    wgpu::read_spirv(std::io::Cursor::new(&code[..])).unwrap()

    // wgpu::read_spirv(glsl_to_spirv::compile(&code, ty).unwrap()).unwrap()
}

pub trait RoundToMultiple {
    fn round_to_multiple(&self, multiple: i32) -> Self;
}

impl RoundToMultiple for f32 {
    fn round_to_multiple(&self, multiple: i32) -> f32 {
        let number = *self;

        if number < 0.0 {
            let number = (number.floor() as i32) * -1;
            -(((number + multiple - 1) / multiple) * multiple) as f32
        } else {
            let number = number.ceil() as i32;
            (((number + multiple - 1) / multiple) * multiple) as f32
        }
    }
}
