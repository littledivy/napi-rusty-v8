use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn node_api_create_syntax_error(
  env: napi_env,
  code: napi_value,
  msg: napi_value,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let code: v8::Local<v8::Value> = std::mem::transmute(code);
  let msg: v8::Local<v8::Value> = std::mem::transmute(msg);

  let msg = msg.to_string(env.scope).unwrap();

  let error = v8::Exception::syntax_error(env.scope, msg);
  *result = std::mem::transmute(error);

  napi_ok
}
