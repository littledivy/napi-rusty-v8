// use std::mem::ManuallyDrop;

use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_open_handle_scope(
  env: napi_env,
  result: *mut napi_value,
) -> napi_status {
  println!("napi_open_handle_scope");
  let mut env = &mut *(env as *mut Env);
  // TODO: do this properly
  // uncommenting this causes panic while panicking
  // let scope = ManuallyDrop::new(v8::HandleScope::new(env.scope));
  // *result = std::mem::transmute(scope);
  napi_ok
}
