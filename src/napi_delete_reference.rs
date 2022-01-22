use crate::env::Env;
use crate::ffi::*;
// use deno_core::v8;

// TODO: properly implement ref counting stuff
#[no_mangle]
pub unsafe extern "C" fn napi_delete_reference(
  env: napi_env,
  nref: napi_ref,
) -> napi_status {
  let mut _env = &mut *(env as *mut Env);
  napi_ok
}
