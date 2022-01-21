use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

// TODO: properly implement
#[no_mangle]
pub unsafe extern "C" fn napi_add_env_cleanup_hook(
  env: napi_env,
  hook: extern "C" fn(*const c_void),
  data: *const c_void,
) -> napi_status {
  let mut _env = &mut *(env as *mut Env);
  napi_ok
}
