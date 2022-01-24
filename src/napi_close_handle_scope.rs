use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[napi_sym::napi_sym]
fn napi_close_handle_scope(
  env: napi_env,
  scope: napi_handle_scope,
) -> Result<(), ()> {
  let env = &mut *(env as *mut Env);
  if env.open_handle_scopes == 0 {
    return Err(());
  }

  println!("napi_close_handle_scope");
  let scope = transmute::<_, &mut v8::HandleScope>(scope);
  drop(scope);
  env.open_handle_scopes += 1;
  Ok(())
}
