// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
use serde::Deserialize;

use serde_v8::utils::{js_exec, v8_do};
use serde_v8::Buffer;
use serde_v8::Error;

#[derive(Debug, Deserialize, PartialEq)]
struct MathOp {
  pub a: u64,
  pub b: u64,
  pub operator: Option<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
enum EnumUnit {
  A,
  B,
  C,
}

#[derive(Debug, PartialEq, Deserialize)]
enum EnumPayloads {
  UInt(u64),
  Int(i64),
  Float(f64),
  Point { x: i64, y: i64 },
  Tuple(bool, i64, ()),
}

fn dedo(
  code: &str,
  f: impl FnOnce(&mut v8::HandleScope, v8::Local<v8::Value>),
) {
  v8_do(|| {
    let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);
    let v = js_exec(scope, code);

    f(scope, v);
  })
}

macro_rules! detest {
  ($fn_name:ident, $t:ty, $src:expr, $rust:expr) => {
    #[test]
    fn $fn_name() {
      #[allow(clippy::bool_assert_comparison)]
      dedo($src, |scope, v| {
        let rt = serde_v8::from_v8(scope, v);
        assert!(rt.is_ok(), "from_v8(\"{}\"): {:?}", $src, rt.err());
        let t: $t = rt.unwrap();
        assert_eq!(t, $rust);
      });
    }
  };
}

macro_rules! defail {
  ($fn_name:ident, $t:ty, $src:expr, $failcase:expr) => {
    #[test]
    fn $fn_name() {
      #[allow(clippy::bool_assert_comparison)]
      dedo($src, |scope, v| {
        let rt: serde_v8::Result<$t> = serde_v8::from_v8(scope, v);
        let rtstr = format!("{:?}", rt);
        let failed_as_expected = $failcase(rt);
        assert!(
          failed_as_expected,
          "expected failure on deserialize(\"{}\"), got: {}",
          $src, rtstr
        );
      });
    }
  };
}

detest!(de_option_some, Option<bool>, "true", Some(true));
detest!(de_option_null, Option<bool>, "null", None);
detest!(de_option_undefined, Option<bool>, "undefined", None);
detest!(de_unit_null, (), "null", ());
detest!(de_unit_undefined, (), "undefined", ());
detest!(de_bool, bool, "true", true);
detest!(de_u64, u64, "32", 32);
detest!(de_string, String, "'Hello'", "Hello".to_owned());
detest!(de_vec_u64, Vec<u64>, "[1,2,3,4,5]", vec![1, 2, 3, 4, 5]);
detest!(
  de_vec_str,
  Vec<String>,
  "['hello', 'world']",
  vec!["hello".to_owned(), "world".to_owned()]
);
detest!(
  de_tuple,
  (u64, bool, ()),
  "[123, true, null]",
  (123, true, ())
);
detest!(
  de_mathop,
  MathOp,
  "({a: 1, b: 3, c: 'ignored'})",
  MathOp {
    a: 1,
    b: 3,
    operator: None
  }
);

// Unit enums
detest!(de_enum_unit_a, EnumUnit, "'A'", EnumUnit::A);
detest!(de_enum_unit_b, EnumUnit, "'B'", EnumUnit::B);
detest!(de_enum_unit_c, EnumUnit, "'C'", EnumUnit::C);

// Enums with payloads (tuples & struct)
detest!(
  de_enum_payload_int,
  EnumPayloads,
  "({ Int: -123 })",
  EnumPayloads::Int(-123)
);
detest!(
  de_enum_payload_uint,
  EnumPayloads,
  "({ UInt: 123 })",
  EnumPayloads::UInt(123)
);
detest!(
  de_enum_payload_float,
  EnumPayloads,
  "({ Float: 1.23 })",
  EnumPayloads::Float(1.23)
);
detest!(
  de_enum_payload_point,
  EnumPayloads,
  "({ Point: { x: 1, y: 2 } })",
  EnumPayloads::Point { x: 1, y: 2 }
);
detest!(
  de_enum_payload_tuple,
  EnumPayloads,
  "({ Tuple: [true, 123, null ] })",
  EnumPayloads::Tuple(true, 123, ())
);

#[test]
fn de_f64() {
  dedo("12345.0", |scope, v| {
    let x: f64 = serde_v8::from_v8(scope, v).unwrap();
    assert!((x - 12345.0).abs() < f64::EPSILON);
  });
}

#[test]
fn de_map() {
  use std::collections::HashMap;

  dedo("({a: 1, b: 2, c: 3})", |scope, v| {
    let map: HashMap<String, u64> = serde_v8::from_v8(scope, v).unwrap();
    assert_eq!(map.get("a").cloned(), Some(1));
    assert_eq!(map.get("b").cloned(), Some(2));
    assert_eq!(map.get("c").cloned(), Some(3));
    assert_eq!(map.get("nada"), None);
  })
}

#[test]
fn de_string_or_buffer() {
  dedo("'hello'", |scope, v| {
    let sob: serde_v8::StringOrBuffer = serde_v8::from_v8(scope, v).unwrap();
    assert_eq!(sob.as_slice(), &[0x68, 0x65, 0x6C, 0x6C, 0x6F]);
  });

  dedo("new Uint8Array([97])", |scope, v| {
    let sob: serde_v8::StringOrBuffer = serde_v8::from_v8(scope, v).unwrap();
    assert_eq!(sob.as_slice(), &[97]);
  });

  dedo("new Uint8Array([128])", |scope, v| {
    let sob: serde_v8::StringOrBuffer = serde_v8::from_v8(scope, v).unwrap();
    assert_eq!(sob.as_slice(), &[128]);
  });

  dedo(
    "(Uint8Array.from([0x68, 0x65, 0x6C, 0x6C, 0x6F]))",
    |scope, v| {
      let sob: serde_v8::StringOrBuffer = serde_v8::from_v8(scope, v).unwrap();
      assert_eq!(sob.as_slice(), &[0x68, 0x65, 0x6C, 0x6C, 0x6F]);
    },
  );
}

#[test]
fn de_buffers() {
  // ArrayBufferView
  dedo("new Uint8Array([97])", |scope, v| {
    let buf: Buffer = serde_v8::from_v8(scope, v).unwrap();
    assert_eq!(&*buf, &[97]);
  });

  // ArrayBuffer
  dedo("(new Uint8Array([97])).buffer", |scope, v| {
    let buf: Buffer = serde_v8::from_v8(scope, v).unwrap();
    assert_eq!(&*buf, &[97]);
  });

  dedo(
    "(Uint8Array.from([0x68, 0x65, 0x6C, 0x6C, 0x6F]))",
    |scope, v| {
      let buf: Buffer = serde_v8::from_v8(scope, v).unwrap();
      assert_eq!(&*buf, &[0x68, 0x65, 0x6C, 0x6C, 0x6F]);
    },
  );
}

////
// JSON tests: serde_json::Value compatibility
////

detest!(
  de_json_null,
  serde_json::Value,
  "null",
  serde_json::Value::Null
);
detest!(
  de_json_bool,
  serde_json::Value,
  "true",
  serde_json::Value::Bool(true)
);
detest!(
  de_json_int,
  serde_json::Value,
  "123",
  serde_json::Value::Number(serde_json::Number::from(123))
);
detest!(
  de_json_float,
  serde_json::Value,
  "123.45",
  serde_json::Value::Number(serde_json::Number::from_f64(123.45).unwrap())
);
detest!(
  de_json_string,
  serde_json::Value,
  "'Hello'",
  serde_json::Value::String("Hello".to_string())
);
detest!(
  de_json_vec_string,
  serde_json::Value,
  "['Hello', 'World']",
  serde_json::Value::Array(vec![
    serde_json::Value::String("Hello".to_string()),
    serde_json::Value::String("World".to_string())
  ])
);
detest!(
  de_json_tuple,
  serde_json::Value,
  "[true, 'World', 123.45, null]",
  serde_json::Value::Array(vec![
    serde_json::Value::Bool(true),
    serde_json::Value::String("World".to_string()),
    serde_json::Value::Number(serde_json::Number::from_f64(123.45).unwrap()),
    serde_json::Value::Null,
  ])
);
detest!(
  de_json_object,
  serde_json::Value,
  "({a: 1, b: 'hello', c: true})",
  serde_json::Value::Object(
    vec![
      (
        "a".to_string(),
        serde_json::Value::Number(serde_json::Number::from(1)),
      ),
      (
        "b".to_string(),
        serde_json::Value::String("hello".to_string()),
      ),
      ("c".to_string(), serde_json::Value::Bool(true),),
    ]
    .drain(..)
    .collect()
  )
);
detest!(de_bigint_u64, u64, "BigInt(2**59)", 1 << 59);
detest!(de_bigint_i64, i64, "BigInt(-(2**59))", -(1 << 59));

defail!(defail_struct, MathOp, "123", |e| e
  == Err(Error::ExpectedObject));

#[derive(PartialEq, Debug, Deserialize)]
pub struct SomeThing {
  pub a: String,
  #[serde(default)]
  pub b: String,
}
detest!(
  de_struct_defaults,
  SomeThing,
  "({ a: 'hello' })",
  SomeThing {
    a: "hello".into(),
    b: "".into()
  }
);
