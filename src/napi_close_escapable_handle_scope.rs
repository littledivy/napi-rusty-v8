use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

#[napi_sym]
fn napi_close_escapable_handle_scope(
  env: napi_env,
  scope: napi_escapable_handle_scope,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  // TODO: do this properly
  Ok(())
}
