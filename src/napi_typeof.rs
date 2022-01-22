use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_typeof(
  env: napi_env,
  value: napi_value,
  result: *mut napi_valuetype,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  if value.is_null() {
    *result = napi_undefined;
    return napi_ok;
  }
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  if value.is_undefined() {
    *result = napi_undefined;
  } else if value.is_null() {
    *result = napi_null;
  } else if value.is_external() {
    *result = napi_external;
  } else if value.is_boolean() {
    *result = napi_boolean;
  } else if value.is_number() {
    *result = napi_number;
  } else if value.is_string() {
    *result = napi_string;
  } else if value.is_symbol() {
    *result = napi_symbol;
  } else if value.is_function() {
    *result = napi_function;
  } else if value.is_big_int() {
    *result = napi_bigint;
  } else if value.is_object() {
    *result = napi_object;
  } else {
    return napi_invalid_arg;
  }
  napi_ok
}
