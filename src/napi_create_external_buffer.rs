use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[napi_sym]
fn napi_create_external_buffer(
  env: napi_env,
  data: *mut c_void,
  byte_length: usize,
  finalize_cb: napi_finalize,
  finalize_hint: *mut c_void,
  result: *mut napi_value,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  let slice = std::slice::from_raw_parts(data as *mut u8, byte_length);
  // TODO: make this not copy the slice
  // TODO: finalization
  let store = v8::ArrayBuffer::new_backing_store_from_boxed_slice(
    slice.to_vec().into_boxed_slice(),
  );
  let ab = v8::ArrayBuffer::with_backing_store(env.scope, &store.make_shared());
  let value = v8::Uint8Array::new(env.scope, ab, 0, byte_length).unwrap();
  let value: v8::Local<v8::Value> = value.into();
  *result = std::mem::transmute(value);
  Ok(())
}
