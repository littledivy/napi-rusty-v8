// use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_get_named_property(
  env: napi_env,
) -> napi_status {
  napi_ok
}