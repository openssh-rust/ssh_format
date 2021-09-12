use std::convert::TryInto;
use serde::{ser, Serialize};

use crate::{Error, Result};

#[derive(Debug, Default)]
pub struct Serializer {
    output: Vec<u8>,
}

impl Serializer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_output(&self) -> &Vec<u8> {
        &self.output
    }

    pub fn reset(&mut self) {
        self.output.clear();
    }
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer::default();
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

macro_rules! impl_for_serialize_primitive {
    ( $name:ident, $type:ty ) => {
        fn $name(self, v: $type) -> Result<()> {
            self.output.extend_from_slice(&v.to_be_bytes());
            Ok(())
        }
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    // Here we go with the simple methods. The following 12 methods receive one
    // of the primitive types of the data model and map it to JSON by appending
    // into the output string.
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.output.push(v);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.output.push(v as u8);
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
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        let len: u32 = v.len()
            .try_into()
            .map_err(|_| Error::BytesTooLong)?;

        self.serialize_u32(len)?;

        self.output.extend_from_slice(v);

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

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
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

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::Unsupported("serialize_map"))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::Unsupported("serialize_variant"))
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("serialize_variant"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::Unsupported("serialize_variant"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::Unsupported("serialize_variant"))
    }
}

macro_rules! impl_serialize_trait {
    ( $name:ident, $function_name:ident ) => {
        impl<'a> ser::$name for &'a mut Serializer {
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
    }
}

impl_serialize_trait!(SerializeSeq, serialize_element);
impl_serialize_trait!(SerializeTuple, serialize_element);
impl_serialize_trait!(SerializeTupleStruct, serialize_field);

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("serialize_map"))
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("serialize_map"))
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported("serialize_map"))
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
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

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("serialize_variant"))
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported("serialize_variant"))
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Unsupported("serialize_variant"))
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupported("serialize_variant"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer() {
        assert_eq!(to_bytes(&0x12_u8).unwrap(), [0x12]);
        assert_eq!(to_bytes(&0x1234_u16).unwrap(), [0x12, 0x34]);
        assert_eq!(to_bytes(&0x12345678_u32).unwrap(), [0x12, 0x34, 0x56, 0x78]);
        assert_eq!(
            to_bytes(&0x1234567887654321_u64).unwrap(),
            [0x12, 0x34, 0x56, 0x78, 0x87, 0x65, 0x43, 0x21]
        );
    }

    #[test]
    fn test_boolean() {
        assert_eq!(to_bytes(&true).unwrap(), [0, 0, 0, 1]);
        assert_eq!(to_bytes(&false).unwrap(), [0, 0, 0, 0]);
    }

    #[test]
    fn test_str() {
        let s = "Hello, world!";
        let serialized = to_bytes(&s).unwrap();
        assert_eq!(&serialized[..4], to_bytes(&(s.len() as u32)).unwrap());
        assert_eq!(&serialized[4..], s.as_bytes());
    }

    #[test]
    fn test_array() {
        let array = [0x00_u8, 0x01_u8, 0x10_u8, 0x78_u8];
        assert_eq!(to_bytes(&array).unwrap(), array);

        assert_eq!(
            to_bytes(&[0x0010_u16, 0x0100_u16, 0x1034_u16, 0x7812_u16]).unwrap(),
            &[0x00_u8, 0x10_u8, 0x01_u8, 0x00_u8, 0x10_u8, 0x34_u8, 0x78_u8, 0x12_u8]
        );
    }

    #[test]
    fn test_tuple() {
        assert_eq!(
            to_bytes(&(0x00_u8, 0x0100_u16, 0x1034_u16, 0x7812_u16)).unwrap(),
            &[0x00_u8, 0x01_u8, 0x00_u8, 0x10_u8, 0x34_u8, 0x78_u8, 0x12_u8]
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
            &[0x00_u8, 0x01_u8, 0x00_u8, 0x10_u8, 0x34_u8, 0x78_u8, 0x12_u8]
        );
    }
}
