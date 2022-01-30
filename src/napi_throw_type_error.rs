use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[napi_sym]
fn napi_throw_type_error(
  env: napi_env,
  code: *const c_char,
  msg: *const c_char,
) -> Result {
  let mut env = &mut *(env as *mut Env);

  // let code = CStr::from_ptr(code).to_str().unwrap();
  let msg = CStr::from_ptr(msg).to_str().unwrap();

  // let code = v8::String::new(env.scope, code).unwrap();
  let msg = v8::String::new(env.scope, msg).unwrap();

  let error = v8::Exception::type_error(env.scope, msg);
  env.scope.throw_exception(error);

  Ok(())
}
