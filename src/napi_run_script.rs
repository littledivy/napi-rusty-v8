use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_run_script(
  env: napi_env,
  script: napi_value,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let script: v8::Local<v8::Value> = std::mem::transmute(script);
  if !script.is_string() {
    return napi_string_expected;
  }
  let script = script.to_string(env.scope).unwrap();

  let script = v8::Script::compile(env.scope, script, None);
  if script.is_none() {
    return napi_generic_failure;
  }
  let script = script.unwrap();
  let rv = script.run(env.scope);

  if let Some(rv) = rv {
    *result = std::mem::transmute(rv);
  } else {
    return napi_generic_failure;
  }

  napi_ok
}
