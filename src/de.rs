use std::{borrow::Cow, convert::TryInto, iter, str};

use serde::de::{self, DeserializeSeed, IntoDeserializer, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;

use crate::{Error, Result};

#[derive(Copy, Clone, Debug)]
pub struct Deserializer<'de, It> {
    slice: &'de [u8],
    iter: It,
}

impl<'de, It> Deserializer<'de, It> {
    pub const fn new(iter: It) -> Self {
        Self { iter, slice: &[] }
    }

    pub fn into_inner(self) -> (&'de [u8], It) {
        (self.slice, self.iter)
    }
}

impl<'de> Deserializer<'de, iter::Empty<&'de [u8]>> {
    pub const fn from_bytes(slice: &'de [u8]) -> Self {
        Self {
            slice,
            iter: iter::empty(),
        }
    }
}

/// Return a deserialized value and trailing bytes.
///
/// # Example
///
/// Simple Usage:
///
/// ```ignore
/// let serialized = to_bytes(value).unwrap();
/// // Ignore the size
/// let (new_value, _trailing_bytes) = from_bytes::<T>(&serialized[4..]).unwrap();
///
/// assert_eq!(value, new_value);
/// ```
///
/// Replace `T` with type of `value`.
///
/// More complicated one (sending over socket):
///
/// ```ignore
/// let buffer = [0, 0, 0, 4];
/// let (size: u32, _trailing_bytes) = from_bytes(&buffer).unwrap();
///
/// let buffer = [0, 0, 4, 0];
/// let (val: <T>, _trailing_bytes) = from_bytes(&buffer).unwrap();
/// ```
///
/// Replace `T` with your own type.
pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<(T, &'a [u8])>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    Ok((t, deserializer.slice))
}

impl<'de, It> Deserializer<'de, It>
where
    It: iter::FusedIterator + Iterator<Item = &'de [u8]>,
{
    /// Extract the loop as a separate function so that `Self::update_slice`
    /// can be trivally inlined.
    fn update_slice_inner(&mut self) {
        self.slice = self.iter.find(|slice| !slice.is_empty()).unwrap_or(&[]);
    }

    #[inline]
    fn update_slice(&mut self) {
        if self.slice.is_empty() {
            self.update_slice_inner();
        }
    }

    fn next_byte(&mut self) -> Result<u8> {
        self.update_slice();

        let byte = self.slice.first().copied().ok_or(Error::Eof)?;
        self.slice = &self.slice[1..];

        Ok(byte)
    }

    fn fill_buffer(&mut self, mut buffer: &mut [u8]) -> Result<()> {
        loop {
            if buffer.is_empty() {
                break Ok(());
            }

            self.update_slice();

            if self.slice.is_empty() {
                break Err(Error::Eof);
            }

            let n = self.slice.len().min(buffer.len());

            buffer[..n].copy_from_slice(&self.slice[..n]);

            self.slice = &self.slice[n..];
            buffer = &mut buffer[n..];
        }
    }

    /// * `SIZE` - must not be 0!
    fn next_bytes_const<const SIZE: usize>(&mut self) -> Result<[u8; SIZE]> {
        assert_ne!(SIZE, 0);

        let mut bytes = [0_u8; SIZE];
        self.fill_buffer(&mut bytes)?;

        Ok(bytes)
    }

    fn next_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.next_bytes_const()?))
    }

    fn next_bytes(&mut self, size: usize) -> Result<Cow<'de, [u8]>> {
        self.update_slice();

        if self.slice.len() >= size {
            let slice = &self.slice[..size];
            self.slice = &self.slice[size..];

            Ok(Cow::Borrowed(slice))
        } else {
            let mut bytes = vec![0_u8; size];
            self.fill_buffer(&mut bytes)?;
            Ok(Cow::Owned(bytes))
        }
    }

    /// Parse &str and &[u8]
    fn parse_bytes(&mut self) -> Result<Cow<'de, [u8]>> {
        let len: usize = self.next_u32()?.try_into().map_err(|_| Error::TooLong)?;
        self.next_bytes(len)
    }
}

macro_rules! impl_for_deserialize_primitive {
    ( $name:ident, $visitor_fname:ident, $type:ty ) => {
        fn $name<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            visitor.$visitor_fname(<$type>::from_be_bytes(self.next_bytes_const()?))
        }
    };
}

impl<'de, 'a, It> de::Deserializer<'de> for &'a mut Deserializer<'de, It>
where
    It: iter::FusedIterator + Iterator<Item = &'de [u8]>,
{
    type Error = Error;

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.next_u32()? {
            1 => visitor.visit_bool(true),
            0 => visitor.visit_bool(false),
            _ => Err(Error::InvalidBoolEncoding),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.next_byte()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.next_byte()? as i8)
    }

    impl_for_deserialize_primitive!(deserialize_i16, visit_i16, i16);
    impl_for_deserialize_primitive!(deserialize_i32, visit_i32, i32);
    impl_for_deserialize_primitive!(deserialize_i64, visit_i64, i64);

    impl_for_deserialize_primitive!(deserialize_u16, visit_u16, u16);
    impl_for_deserialize_primitive!(deserialize_u32, visit_u32, u32);
    impl_for_deserialize_primitive!(deserialize_u64, visit_u64, u64);

    impl_for_deserialize_primitive!(deserialize_f32, visit_f32, f32);
    impl_for_deserialize_primitive!(deserialize_f64, visit_f64, f64);

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match char::from_u32(self.next_u32()?) {
            Some(ch) => visitor.visit_char(ch),
            None => Err(Error::InvalidChar),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_bytes()? {
            Cow::Owned(owned_bytes) => visitor.visit_string(String::from_utf8(owned_bytes)?),
            Cow::Borrowed(bytes) => visitor.visit_borrowed_str(str::from_utf8(bytes)?),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_bytes()? {
            Cow::Owned(owned_bytes) => visitor.visit_byte_buf(owned_bytes),
            Cow::Borrowed(bytes) => visitor.visit_borrowed_bytes(bytes),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(Access {
            deserializer: self,
            len,
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        impl<'a, 'de, It> serde::de::EnumAccess<'de> for &'a mut Deserializer<'de, It>
        where
            It: iter::FusedIterator + Iterator<Item = &'de [u8]>,
        {
            type Error = Error;
            type Variant = Self;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
            where
                V: de::DeserializeSeed<'de>,
            {
                let idx: u32 = self.next_u32()?;
                let val: Result<_> = seed.deserialize(idx.into_deserializer());
                Ok((val?, self))
            }
        }

        visitor.visit_enum(self)
    }

    #[cfg(feature = "is_human_readable")]
    /// Always return `false`
    fn is_human_readable(&self) -> bool {
        false
    }

    /// Unsupported
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.next_u32()? as usize;
        visitor.visit_seq(Access {
            deserializer: self,
            len,
        })
    }

    /// Unsupported
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported(&"deserialize_any"))
    }

    /// Unsupported
    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported(&"deserialize_option"))
    }

    /// Unsupported
    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported(&"deserialize_map"))
    }

    /// Unsupported
    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported(&"deserialize_identifier"))
    }

    /// Unsupported
    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Unsupported(&"deserialize_ignored_any"))
    }
}

impl<'a, 'de, It> VariantAccess<'de> for &'a mut Deserializer<'de, It>
where
    It: iter::FusedIterator + Iterator<Item = &'de [u8]>,
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        DeserializeSeed::deserialize(seed, self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self, fields.len(), visitor)
    }
}

struct Access<'a, 'de, It> {
    deserializer: &'a mut Deserializer<'de, It>,
    len: usize,
}

impl<'a, 'de, It> SeqAccess<'de> for Access<'a, 'de, It>
where
    It: iter::FusedIterator + Iterator<Item = &'de [u8]>,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            self.len -= 1;
            let value = seed.deserialize(&mut *self.deserializer)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

/// Test deserialization
#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use assert_matches::assert_matches;
    use generator::{done, Gn};
    use itertools::Itertools;
    use serde::{de::DeserializeOwned, Serialize};

    use super::*;
    use crate::to_bytes;

    /// Generate subslices, plus stuffing empty slices into the returned
    /// iterator.
    fn generate_subslices(mut bytes: &[u8], chunk_size: usize) -> impl Iterator<Item = &[u8]> {
        assert_ne!(chunk_size, 0);

        Gn::new_scoped(move |mut s| loop {
            for _ in 0..8 {
                // Stuffing empty slices
                s.yield_(&bytes[..0]);
            }

            let n = bytes.len().min(chunk_size);
            s.yield_(&bytes[..n]);
            bytes = &bytes[n..];

            if bytes.is_empty() {
                done!();
            }
        })
    }

    /// First serialize value, then deserialize it.
    fn test_roundtrip<T: Debug + Eq + Serialize + DeserializeOwned>(value: &T) {
        let serialized = to_bytes(value).unwrap();
        // Ignore the size
        let serialized = &serialized[4..];

        // Test from_bytes
        assert_eq!(from_bytes::<T>(serialized).unwrap().0, *value);

        // Test cutting it into multiple small vectors
        for chunk_size in 1..serialized.len() {
            let mut deserializer =
                Deserializer::new(generate_subslices(serialized, chunk_size).fuse());
            let val = T::deserialize(&mut deserializer).unwrap();
            assert_eq!(val, *value);

            let (slice, mut iter) = deserializer.into_inner();

            assert_eq!(slice, &[]);
            assert_eq!(iter.next(), None);
        }
    }

    #[test]
    fn test_integer() {
        test_roundtrip(&0x12_u8);
        test_roundtrip(&0x1234_u16);
        test_roundtrip(&0x12345678_u32);
        test_roundtrip(&0x1234567887654321_u64);
    }

    #[test]
    fn test_boolean() {
        test_roundtrip(&true);
        test_roundtrip(&false);
    }

    #[test]
    fn test_str() {
        let s = "Hello, world!";
        let serialized = to_bytes(&s).unwrap();
        // Ignore the size
        let deserialized: &str = from_bytes(&serialized[4..]).unwrap().0;
        assert_eq!(deserialized, s);
    }

    #[test]
    fn test_seq() {
        test_roundtrip(&vec![0x00_u8, 0x01_u8, 0x10_u8, 0x78_u8]);
        test_roundtrip(&vec![0x0010_u16, 0x0100_u16, 0x1034_u16, 0x7812_u16]);
    }

    #[test]
    fn test_tuple() {
        test_roundtrip(&(0x00_u8, 0x0100_u16, 0x1034_u16, 0x7812_u16));
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
        test_roundtrip(&S {
            v1: 0x00,
            v2: 0x0100,
            v3: 0x1034,
            v4: 0x7812,
        });
    }

    #[test]
    fn test_struct2() {
        #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
        struct S<'a> {
            v1: u8,
            v2: u16,
            v3: u16,
            v4: u16,
            v5: Cow<'a, str>,
        }
        test_roundtrip(&S {
            v1: 0x00,
            v2: 0x0100,
            v3: 0x1034,
            v4: 0x7812,
            v5: Cow::Owned((0..100).join(", ")),
        });
    }

    /// Test EOF error
    #[test]
    fn test_eof_error() {
        assert_matches!(from_bytes::<u8>(&[]), Err(Error::Eof));

        let s = "Hello, world!";
        let serialized = to_bytes(&s).unwrap();
        assert_matches!(
            from_bytes::<String>(&serialized[0..serialized.len() - 1]),
            Err(Error::Eof)
        );
    }
}
