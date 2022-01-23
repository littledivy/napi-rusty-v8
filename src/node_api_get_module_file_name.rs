use crate::env::Env;
use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn node_api_get_module_file_name(
  env: napi_env,
  result: *mut *const c_char,
) -> napi_status {
  let env = &mut *(env as *mut Env);
  let shared = env.shared();
  *result = shared.filename;
  napi_ok
}
