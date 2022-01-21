// use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

pub const NAPI_VERSION: u32 = 8;

#[no_mangle]
pub unsafe extern "C" fn napi_get_version(env: napi_env, version: *mut u32) -> napi_status {
  *version = NAPI_VERSION;
  napi_ok
}
