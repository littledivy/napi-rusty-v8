use crate::env::Env;
use crate::ffi::*;

use crate::util::get_array_buffer_ptr;

#[napi_sym]
fn napi_create_buffer(
  env: napi_env,
  len: usize,
  data: *mut *mut u8,
  result: *mut napi_value,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  let value = v8::ArrayBuffer::new(env.scope, len);
  if !data.is_null() {
    *data = get_array_buffer_ptr(value);
  }
  let value = v8::Uint8Array::new(env.scope, value, 0, len).unwrap();
  let value: v8::Local<v8::Value> = value.into();
  *result = std::mem::transmute(value);
  Ok(())
}
