use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_create_promise(
  env: napi_env,
  deferred: *mut napi_deferred,
  promise_out: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let resolver = v8::PromiseResolver::new(env.scope).unwrap();
  let promise: v8::Local<v8::Value> = resolver.get_promise(env.scope).into();
  *deferred = std::mem::transmute(resolver);
  *promise_out = std::mem::transmute(promise);
  napi_ok
}
