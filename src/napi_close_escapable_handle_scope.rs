use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_close_escapable_handle_scope(
  env: napi_env,
  scope: napi_escapable_handle_scope,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  // TODO: do this properly
  napi_ok
}
