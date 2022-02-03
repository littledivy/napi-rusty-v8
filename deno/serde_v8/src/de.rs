// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
use serde::de::{self, Visitor};
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::keys::{v8_struct_key, KeyCache};
use crate::payload::ValueType;

use crate::magic;

pub struct Deserializer<'a, 'b, 's> {
  input: v8::Local<'a, v8::Value>,
  scope: &'b mut v8::HandleScope<'s>,
  _key_cache: Option<&'b mut KeyCache>,
}

impl<'a, 'b, 's> Deserializer<'a, 'b, 's> {
  pub fn new(
    scope: &'b mut v8::HandleScope<'s>,
    input: v8::Local<'a, v8::Value>,
    key_cache: Option<&'b mut KeyCache>,
  ) -> Self {
    Deserializer {
      input,
      scope,
      _key_cache: key_cache,
    }
  }
}

// from_v8 deserializes a v8::Value into a Deserializable / rust struct
pub fn from_v8<'de, 'a, 'b, 's, T>(
  scope: &'b mut v8::HandleScope<'s>,
  input: v8::Local<'a, v8::Value>,
) -> Result<T>
where
  T: Deserialize<'de>,
{
  let mut deserializer = Deserializer::new(scope, input, None);
  let t = T::deserialize(&mut deserializer)?;
  Ok(t)
}

// like from_v8 except accepts a KeyCache to optimize struct key decoding
pub fn from_v8_cached<'de, 'a, 'b, 's, T>(
  scope: &'b mut v8::HandleScope<'s>,
  input: v8::Local<'a, v8::Value>,
  key_cache: &mut KeyCache,
) -> Result<T>
where
  T: Deserialize<'de>,
{
  let mut deserializer = Deserializer::new(scope, input, Some(key_cache));
  let t = T::deserialize(&mut deserializer)?;
  Ok(t)
}

macro_rules! wip {
  ($method:ident) => {
    fn $method<V>(self, _v: V) -> Result<V::Value>
    where
      V: Visitor<'de>,
    {
      unimplemented!()
    }
  };
}

// TODO: maybe check for BigInt truncation ?
// (i.e: values larger than i64/u64 can hold)
macro_rules! deserialize_signed {
  ($dmethod:ident, $vmethod:ident, $t:tt) => {
    fn $dmethod<V>(self, visitor: V) -> Result<V::Value>
    where
      V: Visitor<'de>,
    {
      let value: $t = match self.input.is_big_int() {
        true => {
          let bigint = v8::Local::<v8::BigInt>::try_from(self.input);
          bigint.unwrap().i64_value().0 as $t
        }
        false => self.input.integer_value(&mut self.scope).unwrap() as $t,
      };
      visitor.$vmethod(value)
    }
  };
}

macro_rules! deserialize_unsigned {
  ($dmethod:ident, $vmethod:ident, $t:tt) => {
    fn $dmethod<V>(self, visitor: V) -> Result<V::Value>
    where
      V: Visitor<'de>,
    {
      let value: $t = match self.input.is_big_int() {
        true => {
          let bigint = v8::Local::<v8::BigInt>::try_from(self.input);
          bigint.unwrap().u64_value().0 as $t
        }
        false => self.input.integer_value(&mut self.scope).unwrap() as $t,
      };
      visitor.$vmethod(value)
    }
  };
}

impl<'de, 'a, 'b, 's, 'x> de::Deserializer<'de>
  for &'x mut Deserializer<'a, 'b, 's>
{
  type Error = Error;

  fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match ValueType::from_v8(self.input) {
      ValueType::Null => self.deserialize_unit(visitor),
      ValueType::Bool => self.deserialize_bool(visitor),
      // Handle floats & ints separately to work with loosely-typed serde_json
      ValueType::Number => {
        if self.input.is_uint32() {
          self.deserialize_u32(visitor)
        } else if self.input.is_int32() {
          self.deserialize_i32(visitor)
        } else {
          self.deserialize_f64(visitor)
        }
      }
      ValueType::String => self.deserialize_string(visitor),
      ValueType::Array => self.deserialize_seq(visitor),
      ValueType::Object => self.deserialize_map(visitor),
      // Map to Vec<u8> when deserialized via deserialize_any
      // e.g: for untagged enums or StringOrBuffer
      ValueType::ArrayBufferView => {
        v8::Local::<v8::ArrayBufferView>::try_from(self.input)
          .and_then(|view| {
            magic::zero_copy_buf::ZeroCopyBuf::try_from((
              &mut *self.scope,
              view,
            ))
          })
          .map_err(|_| Error::ExpectedBuffer)
          .and_then(|zb| visitor.visit_byte_buf(Vec::from(&*zb)))
      }
      ValueType::ArrayBuffer => {
        v8::Local::<v8::ArrayBuffer>::try_from(self.input)
          .and_then(|buffer| {
            magic::zero_copy_buf::ZeroCopyBuf::try_from(buffer)
          })
          .map_err(|_| Error::ExpectedBuffer)
          .and_then(|zb| visitor.visit_byte_buf(Vec::from(&*zb)))
      }
    }
  }

  fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    // Relaxed typechecking, will map all non-true vals to false
    visitor.visit_bool(self.input.is_true())
  }

  // signed
  deserialize_signed!(deserialize_i8, visit_i8, i8);
  deserialize_signed!(deserialize_i16, visit_i16, i16);
  deserialize_signed!(deserialize_i32, visit_i32, i32);
  deserialize_signed!(deserialize_i64, visit_i64, i64);
  // unsigned
  deserialize_unsigned!(deserialize_u8, visit_u8, u8);
  deserialize_unsigned!(deserialize_u16, visit_u16, u16);
  deserialize_unsigned!(deserialize_u32, visit_u32, u32);
  deserialize_unsigned!(deserialize_u64, visit_u64, u64);

  fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_f32(self.input.number_value(self.scope).unwrap() as f32)
  }

  fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_f64(self.input.number_value(self.scope).unwrap())
  }

  wip!(deserialize_char);

  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_string(visitor)
  }

  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    if self.input.is_string() {
      let v8_string = v8::Local::<v8::String>::try_from(self.input).unwrap();
      let string = v8_string.to_rust_string_lossy(self.scope);
      visitor.visit_string(string)
    } else {
      Err(Error::ExpectedString)
    }
  }

  wip!(deserialize_bytes);
  wip!(deserialize_byte_buf);

  fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    if self.input.is_null_or_undefined() {
      visitor.visit_none()
    } else {
      visitor.visit_some(self)
    }
  }

  fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_unit()
  }

  fn deserialize_unit_struct<V>(
    self,
    _name: &'static str,
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_unit(visitor)
  }

  // As is done here, serializers are encouraged to treat newtype structs as
  // insignificant wrappers around the data they contain. That means not
  // parsing anything other than the contained value.
  fn deserialize_newtype_struct<V>(
    self,
    _name: &'static str,
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_newtype_struct(self)
  }

  fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    let arr = v8::Local::<v8::Array>::try_from(self.input)
      .map_err(|_| Error::ExpectedArray)?;
    let len = arr.length();
    let obj = v8::Local::<v8::Object>::from(arr);
    let seq = SeqAccess {
      pos: 0,
      len,
      obj,
      scope: self.scope,
    };
    visitor.visit_seq(seq)
  }

  // Like deserialize_seq except it prefers tuple's length over input array's length
  fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    // TODO: error on length mismatch
    let obj = v8::Local::<v8::Object>::try_from(self.input).unwrap();
    let seq = SeqAccess {
      pos: 0,
      len: len as u32,
      obj,
      scope: self.scope,
    };
    visitor.visit_seq(seq)
  }

  // Tuple structs look just like sequences in JSON.
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

  fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
  where
    V: de::Visitor<'de>,
  {
    // Assume object, then get_own_property_names
    let obj = v8::Local::<v8::Object>::try_from(self.input)
      .map_err(|_| Error::ExpectedObject)?;
    let prop_names = obj.get_own_property_names(self.scope);
    let mut keys: Vec<magic::Value> = match prop_names {
      Some(names) => from_v8(self.scope, names.into()).unwrap(),
      None => vec![],
    };
    let keys: Vec<v8::Local<v8::Value>> = keys
      .drain(..)
      .map(|x| x.into())
      // Filter keys to drop keys whose value is undefined
      // TODO: optimize, since this doubles our get calls
      .filter(|key| !obj.get(self.scope, *key).unwrap().is_undefined())
      .collect();

    let map = MapAccess {
      obj,
      keys,
      pos: 0,
      scope: self.scope,
    };
    visitor.visit_map(map)
  }

  fn deserialize_struct<V>(
    self,
    name: &'static str,
    fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    // Magic for serde_v8::magic::Value, to passthrough v8::Value
    // TODO: ensure this is cross-platform and there's no alternative
    if name == magic::NAME {
      let mv = magic::Value {
        v8_value: self.input,
      };
      let hack: u64 = unsafe { std::mem::transmute(mv) };
      return visitor.visit_u64(hack);
    }

    // Magic Buffer
    if name == magic::buffer::BUF_NAME {
      let zero_copy_buf = match self.input.is_array_buffer() {
        // ArrayBuffer
        true => v8::Local::<v8::ArrayBuffer>::try_from(self.input)
          .and_then(magic::zero_copy_buf::ZeroCopyBuf::try_from),
        // maybe ArrayBufferView
        false => v8::Local::<v8::ArrayBufferView>::try_from(self.input)
          .and_then(|view| {
            magic::zero_copy_buf::ZeroCopyBuf::try_from((
              &mut *self.scope,
              view,
            ))
          }),
      }
      .map_err(|_| Error::ExpectedBuffer)?;
      let data: [u8; 32] = unsafe { std::mem::transmute(zero_copy_buf) };
      return visitor.visit_bytes(&data);
    }

    // Magic ByteString
    if name == magic::bytestring::NAME {
      if let Some(v8_string) = self.input.to_string(self.scope) {
        if v8_string.contains_only_onebyte() {
          let mut buffer: Vec<u8> = vec![0u8; v8_string.length()];
          let written = v8_string.write_one_byte(
            self.scope,
            &mut buffer,
            0,
            v8::WriteOptions::NO_NULL_TERMINATION,
          );
          assert!(written == v8_string.length());
          return visitor.visit_byte_buf(buffer);
        } else {
          return Err(Error::Message(
            "Expected a valid ByteString.".to_string(),
          ));
        }
      } else {
        return Err(Error::ExpectedString);
      }
    }

    // Regular struct
    let obj = v8::Local::<v8::Object>::try_from(self.input)
      .map_err(|_| Error::ExpectedObject)?;
    let struct_access = StructAccess {
      fields,
      obj,
      pos: 0,
      scope: self.scope,
      _cache: None,
    };

    visitor.visit_seq(struct_access)
  }

  /// To be compatible with `serde-json`, we expect enums to be:
  /// - `"Variant"`: strings for unit variants, i.e: Enum::Variant
  /// - `{ Variant: payload }`: single K/V pairs, converted to `Enum::Variant { payload }`
  fn deserialize_enum<V>(
    self,
    _name: &str,
    _variants: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    // Unit variant
    if self.input.is_string() {
      let payload = v8::undefined(self.scope).into();
      visitor.visit_enum(EnumAccess {
        scope: self.scope,
        tag: self.input,
        payload,
      })
    }
    // Struct or tuple variant
    else if self.input.is_object() {
      // Assume object
      let obj = v8::Local::<v8::Object>::try_from(self.input).unwrap();
      // Unpack single-key
      let tag = {
        let prop_names = obj.get_own_property_names(self.scope);
        let prop_names = prop_names.ok_or(Error::ExpectedEnum)?;
        if prop_names.length() != 1 {
          return Err(Error::LengthMismatch);
        }
        prop_names.get_index(self.scope, 0).unwrap()
      };

      let payload = obj.get(self.scope, tag).unwrap();
      visitor.visit_enum(EnumAccess {
        scope: self.scope,
        tag,
        payload,
      })
    } else {
      // TODO: improve error
      Err(Error::ExpectedEnum)
    }
  }

  // An identifier in Serde is the type that identifies a field of a struct or
  // the variant of an enum. In JSON, struct fields and enum variants are
  // represented as strings. In other formats they may be represented as
  // numeric indices.
  fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_str(visitor)
  }

  fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_none()
  }
}

struct MapAccess<'a, 'b, 's> {
  obj: v8::Local<'a, v8::Object>,
  scope: &'b mut v8::HandleScope<'s>,
  keys: Vec<v8::Local<'a, v8::Value>>,
  pos: usize,
}

impl<'de> de::MapAccess<'de> for MapAccess<'_, '_, '_> {
  type Error = Error;

  fn next_key_seed<K: de::DeserializeSeed<'de>>(
    &mut self,
    seed: K,
  ) -> Result<Option<K::Value>> {
    Ok(match self.keys.get(self.pos) {
      Some(key) => {
        let mut deserializer = Deserializer::new(self.scope, *key, None);
        Some(seed.deserialize(&mut deserializer)?)
      }
      None => None,
    })
  }

  fn next_value_seed<V: de::DeserializeSeed<'de>>(
    &mut self,
    seed: V,
  ) -> Result<V::Value> {
    if self.pos >= self.keys.len() {
      return Err(Error::LengthMismatch);
    }
    let key = self.keys[self.pos];
    self.pos += 1;
    let v8_val = self.obj.get(self.scope, key).unwrap();
    let mut deserializer = Deserializer::new(self.scope, v8_val, None);
    seed.deserialize(&mut deserializer)
  }

  fn next_entry_seed<
    K: de::DeserializeSeed<'de>,
    V: de::DeserializeSeed<'de>,
  >(
    &mut self,
    kseed: K,
    vseed: V,
  ) -> Result<Option<(K::Value, V::Value)>> {
    if self.pos >= self.keys.len() {
      return Ok(None);
    }
    let v8_key = self.keys[self.pos];
    self.pos += 1;
    let mut kdeserializer = Deserializer::new(self.scope, v8_key, None);
    Ok(Some((kseed.deserialize(&mut kdeserializer)?, {
      let v8_val = self.obj.get(self.scope, v8_key).unwrap();
      let mut deserializer = Deserializer::new(self.scope, v8_val, None);
      vseed.deserialize(&mut deserializer)?
    })))
  }
}

struct StructAccess<'a, 'b, 's> {
  obj: v8::Local<'a, v8::Object>,
  scope: &'b mut v8::HandleScope<'s>,
  fields: &'static [&'static str],
  pos: usize,
  _cache: Option<&'b mut KeyCache>,
}

impl<'de> de::SeqAccess<'de> for StructAccess<'_, '_, '_> {
  type Error = Error;

  fn next_element_seed<T: de::DeserializeSeed<'de>>(
    &mut self,
    seed: T,
  ) -> Result<Option<T::Value>> {
    if self.pos >= self.fields.len() {
      return Ok(None);
    }

    let pos = self.pos;
    self.pos += 1;

    let field = self.fields[pos];
    let key = v8_struct_key(self.scope, field).into();
    let val = self.obj.get(self.scope, key).unwrap();
    let mut deserializer = Deserializer::new(self.scope, val, None);
    match seed.deserialize(&mut deserializer) {
      Ok(val) => Ok(Some(val)),
      // Fallback to Ok(None) for #[serde(Default)] at cost of error quality ...
      // TODO(@AaronO): double check that there's no better solution
      Err(_) if val.is_undefined() => Ok(None),
      Err(e) => Err(e),
    }
  }
}

struct SeqAccess<'a, 'b, 's> {
  obj: v8::Local<'a, v8::Object>,
  scope: &'b mut v8::HandleScope<'s>,
  len: u32,
  pos: u32,
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'_, '_, '_> {
  type Error = Error;

  fn next_element_seed<T: de::DeserializeSeed<'de>>(
    &mut self,
    seed: T,
  ) -> Result<Option<T::Value>> {
    let pos = self.pos;
    self.pos += 1;

    if pos < self.len {
      let val = self.obj.get_index(self.scope, pos).unwrap();
      let mut deserializer = Deserializer::new(self.scope, val, None);
      Ok(Some(seed.deserialize(&mut deserializer)?))
    } else {
      Ok(None)
    }
  }

  fn size_hint(&self) -> Option<usize> {
    Some((self.len - self.pos) as usize)
  }
}

struct EnumAccess<'a, 'b, 's> {
  tag: v8::Local<'a, v8::Value>,
  payload: v8::Local<'a, v8::Value>,
  scope: &'b mut v8::HandleScope<'s>,
  // p1: std::marker::PhantomData<&'x ()>,
}

impl<'de, 'a, 'b, 's, 'x> de::EnumAccess<'de> for EnumAccess<'a, 'b, 's> {
  type Error = Error;
  type Variant = VariantDeserializer<'a, 'b, 's>;

  fn variant_seed<V: de::DeserializeSeed<'de>>(
    self,
    seed: V,
  ) -> Result<(V::Value, Self::Variant)> {
    let seed = {
      let mut dtag = Deserializer::new(self.scope, self.tag, None);
      seed.deserialize(&mut dtag)
    };
    let dpayload = VariantDeserializer::<'a, 'b, 's> {
      scope: self.scope,
      value: self.payload,
    };

    Ok((seed?, dpayload))
  }
}

struct VariantDeserializer<'a, 'b, 's> {
  value: v8::Local<'a, v8::Value>,
  scope: &'b mut v8::HandleScope<'s>,
}

impl<'de, 'a, 'b, 's> de::VariantAccess<'de>
  for VariantDeserializer<'a, 'b, 's>
{
  type Error = Error;

  fn unit_variant(self) -> Result<()> {
    let mut d = Deserializer::new(self.scope, self.value, None);
    de::Deserialize::deserialize(&mut d)
  }

  fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
    self,
    seed: T,
  ) -> Result<T::Value> {
    let mut d = Deserializer::new(self.scope, self.value, None);
    seed.deserialize(&mut d)
  }

  fn tuple_variant<V: de::Visitor<'de>>(
    self,
    len: usize,
    visitor: V,
  ) -> Result<V::Value> {
    let mut d = Deserializer::new(self.scope, self.value, None);
    de::Deserializer::deserialize_tuple(&mut d, len, visitor)
  }

  fn struct_variant<V: de::Visitor<'de>>(
    self,
    fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value> {
    let mut d = Deserializer::new(self.scope, self.value, None);
    de::Deserializer::deserialize_struct(&mut d, "", fields, visitor)
  }
}
