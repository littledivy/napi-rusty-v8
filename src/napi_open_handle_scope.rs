use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[napi_sym::napi_sym]
fn napi_open_handle_scope(
  env: napi_env,
  result: *mut napi_handle_scope,
) -> Result<(), ()> {
  let env = &mut *(env as *mut Env);
  println!("napi_open_handle_scope");
  let scope = &mut v8::HandleScope::new(env.scope);
  *result = transmute(scope);
  println!("napi_open_handle_scope done");

  env.open_handle_scopes += 1;
  Ok(())
}
