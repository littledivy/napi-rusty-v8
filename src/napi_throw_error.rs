use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_throw_error(
  env: napi_env,
  code: *const c_char,
  msg: *const c_char,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  // let code = CStr::from_ptr(code).to_str().unwrap();
  let msg = CStr::from_ptr(msg).to_str().unwrap();

  println!("napi_throw_error: {:?}", msg);

  // let code = v8::String::new(env.scope, code).unwrap();
  let msg = v8::String::new(env.scope, msg).unwrap();

  let error = v8::Exception::error(env.scope, msg);
  env.scope.throw_exception(error);

  napi_ok
}
