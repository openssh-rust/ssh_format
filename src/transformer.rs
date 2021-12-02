use super::{from_bytes, Result, Serializer};
use serde::{Deserialize, Serialize};

/// Serialize and deserialize types using the same buffer.
#[derive(Clone, Debug, Default)]
pub struct Transformer(Serializer);

impl Transformer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize value
    pub fn serialize(&mut self, value: impl Serialize) -> Result<&[u8]> {
        self.0.output.resize(4, 0);
        value.serialize(&mut self.0)?;
        Ok(self.0.get_output()?)
    }

    /// Get underlying serializer, can be used without reset
    pub fn get_ser(&mut self) -> &mut Serializer {
        self.0.output.resize(4, 0);
        &mut self.0
    }

    /// Return the buffer so that you can read the input into it.
    /// You can also adjust the capacity and free up memory.
    pub fn get_buffer(&mut self) -> &mut Vec<u8> {
        &mut self.0.output
    }

    /// NOTE that calling the serialized result cannot be deserialized directly,
    /// since it also including a 4-byte prefix representing the length of the
    /// serialized data.
    pub fn deserialize<'a, T: Deserialize<'a>>(&'a self) -> Result<(T, &'a [u8])> {
        from_bytes(&self.0.output)
    }
}

#[cfg(test)]
mod tests {
    use super::Transformer;

    use serde::{Deserialize, Serialize};
    use std::fmt::Debug;

    /// First serialize value, then deserialize it.
    fn test_roundtrip<'a, T: Debug + Eq + Serialize + Deserialize<'a>>(
        transformer: &'a mut Transformer,
        value: &T,
    ) {
        transformer.serialize(value).unwrap();
        for _ in 0..4 {
            transformer.get_buffer().remove(0);
        }
        // Ignore the size
        assert_eq!(transformer.deserialize::<'_, T>().unwrap().0, *value);
    }

    #[test]
    fn test_integer() {
        let mut transformer = Transformer::new();

        test_roundtrip(&mut transformer, &0x12_u8);
        test_roundtrip(&mut transformer, &0x1234_u16);
        test_roundtrip(&mut transformer, &0x12345678_u32);
        test_roundtrip(&mut transformer, &0x1234567887654321_u64);
    }

    #[test]
    fn test_boolean() {
        let mut transformer = Transformer::new();

        test_roundtrip(&mut transformer, &true);
        test_roundtrip(&mut transformer, &false);
    }

    #[test]
    fn test_str() {
        test_roundtrip(&mut Transformer::new(), &"Hello, world!");
    }

    #[test]
    fn test_array() {
        let mut transformer = Transformer::new();

        test_roundtrip(&mut transformer, &[0x00_u8, 0x01_u8, 0x10_u8, 0x78_u8]);
        test_roundtrip(
            &mut transformer,
            &[0x0010_u16, 0x0100_u16, 0x1034_u16, 0x7812_u16],
        );
    }

    #[test]
    fn test_tuple() {
        test_roundtrip(
            &mut Transformer::new(),
            &(0x00_u8, 0x0100_u16, 0x1034_u16, 0x7812_u16),
        );
    }

    #[test]
    fn test_struct() {
        #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
        struct S {
            v1: u8,
            v2: u16,
            v3: u16,
            v4: u16,
        }
        test_roundtrip(
            &mut Transformer::new(),
            &S {
                v1: 0x00,
                v2: 0x0100,
                v3: 0x1034,
                v4: 0x7812,
            },
        );
    }

    #[test]
    fn test_enum() {
        #[repr(u32)]
        #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
        enum E {
            A = 1,
            B = 2,
            C = 9999,
        }

        test_roundtrip(&mut Transformer::new(), &E::A);
        test_roundtrip(&mut Transformer::new(), &E::B);
        test_roundtrip(&mut Transformer::new(), &E::C);
    }
}
