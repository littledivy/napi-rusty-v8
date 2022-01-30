use crate::env::Env;
use crate::ffi::*;
use crate::function::CallbackInfo;
use deno_core::v8;

#[napi_sym]
fn napi_get_new_target(
  env: napi_env,
  cbinfo: napi_callback_info,
  result: *mut napi_value,
) -> Result {
  let mut env = &mut *(env as *mut Env);

  let cbinfo: &CallbackInfo = &*(cbinfo as *const CallbackInfo);
  let args = &*(cbinfo.args as *const v8::FunctionCallbackArguments);

  // TODO: need v8 binding

  Ok(())
}
