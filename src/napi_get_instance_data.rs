use crate::env::Env;
use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_get_instance_data(
  env: napi_env,
  result: *mut *mut c_void,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let shared = env.shared();
  *result = shared.instance_data;
  env.ok()
}
