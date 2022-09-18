/// A trait for which can be used to store serialized output.
pub trait SerBacker {
    fn extend_from_slice(&mut self, other: &[u8]);
    fn push(&mut self, byte: u8);

    /// Reserves capacity for at least additional more bytes to be inserted.
    ///
    /// More than additional bytes may be reserved in order to avoid frequent
    /// reallocations. A call to reserve may result in an allocation.
    fn reserve(&mut self, additional: usize);
}

impl<T: SerBacker> SerBacker for &mut T {
    fn extend_from_slice(&mut self, other: &[u8]) {
        (*self).extend_from_slice(other)
    }

    fn push(&mut self, byte: u8) {
        (*self).push(byte)
    }

    fn reserve(&mut self, additional: usize) {
        (*self).reserve(additional);
    }
}

impl SerBacker for Vec<u8> {
    fn extend_from_slice(&mut self, other: &[u8]) {
        self.extend_from_slice(other)
    }

    fn push(&mut self, byte: u8) {
        self.push(byte)
    }

    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}
