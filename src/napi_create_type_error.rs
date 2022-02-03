use crate::env::Env;
use crate::ffi::*;

#[napi_sym]
fn napi_create_type_error(
  env: napi_env,
  code: napi_value,
  msg: napi_value,
  result: *mut napi_value,
) -> Result {
  let mut env = &mut *(env as *mut Env);

  let code: v8::Local<v8::Value> = std::mem::transmute(code);
  let msg: v8::Local<v8::Value> = std::mem::transmute(msg);

  let msg = msg.to_string(env.scope).unwrap();

  let error = v8::Exception::type_error(env.scope, msg);
  *result = std::mem::transmute(error);

  Ok(())
}
