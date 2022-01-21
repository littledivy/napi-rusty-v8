use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_create_external_arraybuffer(
  env: napi_env,
  data: *mut c_void,
  byte_length: usize,
  finalize_cb: napi_finalize_callback,
  finalize_hint: *mut c_void,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let slice = std::slice::from_raw_parts(data as *mut u8, byte_length);
  let store = v8::ArrayBuffer::new_backing_store_from_boxed_slice(slice.to_vec().into_boxed_slice());
  // TODO
  napi_ok
}
