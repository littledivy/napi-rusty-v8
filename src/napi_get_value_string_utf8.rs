use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_get_value_string_utf8(
  env: napi_env,
  value: napi_value,
  result: *mut c_char,
  result_len: usize,
  result_copied: *mut usize,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let value: v8::Local<v8::Value> = std::mem::transmute(value);

  if !value.is_string() && !value.is_string_object() {
    return napi_string_expected;
  }

  let v8str = value.to_string(env.scope).unwrap();
  let string_len = v8str.utf8_length(env.scope);

  let string = v8str.to_rust_string_lossy(env.scope);

  let string = string.as_bytes();
  let string = string.as_ptr();
  let string = string as *const c_char;

  *result_copied = string_len;

  if result_len < string_len {
    return napi_ok;
  }

  if !result.is_null() {
    std::ptr::copy_nonoverlapping(string, result, string_len as usize);
    *result.offset(string_len as isize) = 0;
  }

  napi_ok
}
