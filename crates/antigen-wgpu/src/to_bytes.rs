pub trait ToBytes {
    fn to_bytes(&self) -> &[u8];
}

impl ToBytes for f32 {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl<T> ToBytes for Vec<T> where T: bytemuck::Pod {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self[..])
    }
}

impl<T, const N: usize> ToBytes for [T; N] where [T; N]: bytemuck::Pod {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl<U, T> ToBytes for antigen_core::Usage<U, T> where T: ToBytes {
    fn to_bytes(&self) -> &[u8] {
        self.data.to_bytes()
    }
}
