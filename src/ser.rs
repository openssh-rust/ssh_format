use serde::{ser, Serialize};
use std::convert::TryInto;

use crate::{Error, Result, SerOutput};

fn usize_to_u32(v: usize) -> Result<u32> {
    v.try_into().map_err(|_| Error::TooLong)
}

#[derive(Clone, Debug)]
pub struct Serializer<T: SerOutput = Vec<u8>> {
    pub output: T,
    len: usize,
}

impl<T: SerOutput + Default> Default for Serializer<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: SerOutput> Serializer<T> {
    pub fn new(output: T) -> Self {
        Self { output, len: 0 }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.output.reserve(additional);
    }

    /// * `len` - length of additional data included in the packet.
    pub fn create_header(&self, len: u32) -> Result<[u8; 4]> {
        let len: u32 = usize_to_u32(self.len + len as usize)?;

        Ok(len.to_be_bytes())
    }

    /// Reset the internal counter.
    /// This would cause [`Self::create_header`] to return `Ok([0, 0, 0, 0])`
    /// until you call [`Serialize::serialize`] again.
    pub fn reset_counter(&mut self) {
        self.len = 0;
    }

    fn extend_from_slice(&mut self, other: &[u8]) {
        self.output.extend_from_slice(other);
        self.len += other.len();
    }

    fn push(&mut self, byte: u8) {
        self.output.push(byte);
        self.len += 1;
    }

    fn serialize_usize(&mut self, v: usize) -> Result<()> {
        ser::Serializer::serialize_u32(self, usize_to_u32(v)?)
    }
}

/// Return a byte array with the first 4 bytes representing the size
/// of the rest of the serialized message.
///
/// See doc of `from_bytes` for examples.
pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut buffer = vec![0, 0, 0, 0];

    let mut serializer = Serializer::new(&mut buffer);
    value.serialize(&mut serializer)?;
    let header = serializer.create_header(0)?;

    buffer[..4].copy_from_slice(&header);

    Ok(buffer)
}

macro_rules! impl_for_serialize_primitive {
    ( $name:ident, $type:ty ) => {
        fn $name(self, v: $type) -> Result<()> {
            self.extend_from_slice(&v.to_be_bytes());
            Ok(())
        }
    };
}

impl<'a, Container: SerOutput> ser::Serializer for &'a mut Serializer<Container> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.push(v);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.push(v as u8);
        Ok(())
    }

    impl_for_serialize_primitive!(serialize_i16, i16);
    impl_for_serialize_primitive!(serialize_i32, i32);
    impl_for_serialize_primitive!(serialize_i64, i64);

    impl_for_serialize_primitive!(serialize_u16, u16);
    impl_for_serialize_primitive!(serialize_u32, u32);
    impl_for_serialize_primitive!(serialize_u64, u64);

    impl_for_serialize_primitive!(serialize_f32, f32);
    impl_for_serialize_primitive!(serialize_f64, f64);

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        fn is_null_byte(byte: &u8) -> bool {
            *byte == b'\0'
        }

        let bytes = v.as_bytes();

        let null_byte_counts = bytes.iter().copied().filter(is_null_byte).count();

        let len = bytes.len() - null_byte_counts;

        // Reserve bytes
        self.reserve(4 + len);

        self.serialize_usize(len)?;

        if null_byte_counts == 0 {
            self.extend_from_slice(v.as_bytes());
        } else {
            bytes
                .split(is_null_byte)
                .filter(|slice| !slice.is_empty())
                .for_each(|slice| {
                    self.extend_from_slice(slice);
                });
        }

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.reserve(4 + v.len());

        self.serialize_usize(v.len())?;

        self.extend_from_slice(v);

        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        if let Some(len) = len {
            self.reserve(4 + len as usize);

            self.serialize_usize(len)?;
        }
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.serialize_u32(variant_index)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.serialize_unit_variant(name, variant_index, variant)?;
        value.serialize(self)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.serialize_unit_variant(name, variant_index, variant)?;
        self.serialize_tuple(len)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_unit_variant(name, variant_index, variant)?;
        self.serialize_tuple(len)
    }

    #[cfg(feature = "is_human_readable")]
    /// Always return false
    fn is_human_readable(&self) -> bool {
        false
    }

    /// Unsupported
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::Unsupported(&"serialize_map"))
    }
}

macro_rules! impl_serialize_trait {
    ( $name:ident, $function_name:ident ) => {
        impl<'a, Container: SerOutput> ser::$name for &'a mut Serializer<Container> {
            type Ok = ();
            type Error = Error;

            fn $function_name<T>(&mut self, value: &T) -> Result<()>
            where
                T: ?Sized + Serialize,
            {
                value.serialize(&mut **self)
            }

            fn end(self) -> Result<()> {
                Ok(())
            }
        }
    };
}

impl_serialize_trait!(SerializeSeq, serialize_element);
impl_serialize_trait!(SerializeTuple, serialize_element);
impl_serialize_trait!(SerializeTupleStruct, serialize_field);
impl_serialize_trait!(SerializeTupleVariant, serialize_field);

/// Unsupported
impl<'a, Container: SerOutput> ser::SerializeMap for &'a mut Serializer<Container> {
    type Ok = ();
    type Error = Error;

    /// Unsupported
    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported(&"serialize_map"))
    }

    /// Unsupported
    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported(&"serialize_map"))
    }

    /// Unsupported
    fn end(self) -> Result<()> {
        Err(Error::Unsupported(&"serialize_map"))
    }
}

impl<'a, Container: SerOutput> ser::SerializeStruct for &'a mut Serializer<Container> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}
impl<'a, Container: SerOutput> ser::SerializeStructVariant for &'a mut Serializer<Container> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeStruct::end(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{to_bytes, Serializer};
    use serde::{ser, Serialize};
    use std::convert::TryInto;

    #[test]
    fn test_integer() {
        assert_eq!(to_bytes(&0x12_u8).unwrap(), [0, 0, 0, 1, 0x12]);
        assert_eq!(to_bytes(&0x1234_u16).unwrap(), [0, 0, 0, 2, 0x12, 0x34]);
        assert_eq!(
            to_bytes(&0x12345678_u32).unwrap(),
            [0, 0, 0, 4, 0x12, 0x34, 0x56, 0x78]
        );
        assert_eq!(
            to_bytes(&0x1234567887654321_u64).unwrap(),
            [0, 0, 0, 8, 0x12, 0x34, 0x56, 0x78, 0x87, 0x65, 0x43, 0x21]
        );
    }

    #[test]
    fn test_boolean() {
        assert_eq!(to_bytes(&true).unwrap(), [0, 0, 0, 4, 0, 0, 0, 1]);
        assert_eq!(to_bytes(&false).unwrap(), [0, 0, 0, 4, 0, 0, 0, 0]);
    }

    #[test]
    fn test_str() {
        let s = "Hello, world!";
        let serialized = to_bytes(&s).unwrap();
        let len: u32 = (serialized.len() - 4).try_into().unwrap();
        assert_eq!(&serialized[..4], len.to_be_bytes());
        assert_eq!(&serialized[4..8], (s.len() as u32).to_be_bytes());
        assert_eq!(&serialized[8..], s.as_bytes());
    }

    #[test]
    fn test_str_with_null() {
        let s = "\0Hello, world!";
        let serialized = to_bytes(&s).unwrap();
        let len: u32 = (serialized.len() - 4).try_into().unwrap();
        assert_eq!(&serialized[..4], len.to_be_bytes());
        assert_eq!(&serialized[4..8], ((s.len() - 1) as u32).to_be_bytes());

        assert_eq!(&serialized[8..], &s.as_bytes()[1..]);
    }

    #[test]
    fn test_array() {
        let array = [0x00_u8, 0x01_u8, 0x10_u8, 0x78_u8];
        let slice: &[_] = &array;

        let serialized = to_bytes(&slice).unwrap();
        assert_eq!(serialized[..4], [0, 0, 0, 8]);
        assert_eq!(serialized[4..8], [0, 0, 0, 4]);
        assert_eq!(serialized[8..], array);

        let slice: &[_] = &[0x0010_u16, 0x0100_u16, 0x1034_u16, 0x7812_u16];

        assert_eq!(
            to_bytes(&slice).unwrap(),
            &[0, 0, 0, 12, 0, 0, 0, 4, 0x00, 0x10, 0x01, 0x00, 0x10, 0x34, 0x78, 0x12_u8]
        );
    }

    #[test]
    fn test_tuple() {
        assert_eq!(
            to_bytes(&(0x00_u8, 0x0100_u16, 0x1034_u16, 0x7812_u16)).unwrap(),
            &[0, 0, 0, 7, 0x00_u8, 0x01_u8, 0x00_u8, 0x10_u8, 0x34_u8, 0x78_u8, 0x12_u8]
        );
    }

    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct S {
            v1: u8,
            v2: u16,
            v3: u16,
            v4: u16,
        }
        let v = S {
            v1: 0x00,
            v2: 0x0100,
            v3: 0x1034,
            v4: 0x7812,
        };
        assert_eq!(
            to_bytes(&v).unwrap(),
            &[0, 0, 0, 7, 0x00_u8, 0x01_u8, 0x00_u8, 0x10_u8, 0x34_u8, 0x78_u8, 0x12_u8]
        );
    }

    #[test]
    fn test_enum() {
        use ser::Serializer as SerdeSerializerTrait;

        let mut serializer: Serializer<Vec<u8>> = Serializer::default();

        serializer.serialize_unit_variant("", 1, "").unwrap();
        assert_eq!(serializer.create_header(0).unwrap(), [0, 0, 0, 4]);
        assert_eq!(serializer.output, [0, 0, 0, 1]);

        // Reset serializer
        serializer.reset_counter();
        serializer.output.clear();

        serializer.serialize_newtype_variant("", 0, "", &3).unwrap();
        assert_eq!(serializer.create_header(0).unwrap(), [0, 0, 0, 8]);
        assert_eq!(serializer.output, [0, 0, 0, 0, 0, 0, 0, 3]);
    }
}
