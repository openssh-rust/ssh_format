use core::convert::TryInto;

/// A trait for which can be used to store serialized output.
#[allow(clippy::len_without_is_empty)]
pub trait SerBacker {
    /// Return a new backer which len() == 4.
    fn new() -> Self;

    fn len(&self) -> usize;

    fn get_first_4byte_slice(&mut self) -> &mut [u8; 4];

    fn extend_from_slice(&mut self, other: &[u8]);
    fn push(&mut self, byte: u8);

    /// Reset to the initial state where len() == 4
    fn reset(&mut self);

    /// Reserves capacity for at least additional more bytes to be inserted.
    ///
    /// More than additional bytes may be reserved in order to avoid frequent
    /// reallocations. A call to reserve may result in an allocation.
    fn reserve(&mut self, additional: usize);
}

impl SerBacker for Vec<u8> {
    fn new() -> Self {
        vec![0, 0, 0, 0]
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn get_first_4byte_slice(&mut self) -> &mut [u8; 4] {
        (&mut self[..4]).try_into().unwrap()
    }

    fn extend_from_slice(&mut self, other: &[u8]) {
        self.extend_from_slice(other)
    }

    fn push(&mut self, byte: u8) {
        self.push(byte)
    }

    fn reset(&mut self) {
        self.resize(4, 0);
    }

    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}
