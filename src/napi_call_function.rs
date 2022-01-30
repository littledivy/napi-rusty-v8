use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[napi_sym]
fn napi_call_function(
  env: napi_env,
  recv: napi_value,
  func: napi_value,
  argc: usize,
  argv: *const napi_value,
  result: *mut napi_value,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  let recv: v8::Local<v8::Value> = std::mem::transmute(recv);
  let func: v8::Local<v8::Value> = std::mem::transmute(func);
  let func = v8::Local::<v8::Function>::try_from(func).unwrap();
  let args: &[v8::Local<v8::Value>] =
    std::mem::transmute(std::slice::from_raw_parts(argv, argc));
  let ret = func.call(env.scope, recv, args).unwrap();
  let value: v8::Local<v8::Value> = ret.into();
  *result = std::mem::transmute(value);
  Ok(())
}
