use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

// TODO: properly implement
#[napi_sym]
fn napi_remove_env_cleanup_hook(
  env: napi_env,
  hook: extern "C" fn(*const c_void),
  data: *const c_void,
) -> Result {
  let mut _env = &mut *(env as *mut Env);
  Ok(())
}
