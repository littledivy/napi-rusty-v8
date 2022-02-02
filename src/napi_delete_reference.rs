use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

// TODO: properly implement ref counting stuff
#[napi_sym]
fn napi_delete_reference(env: napi_env, nref: napi_ref) -> Result {
  let mut _env = &mut *(env as *mut Env);
  Ok(())
}
