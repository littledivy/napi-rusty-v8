use crate::env::Env;
use crate::ffi::*;

#[napi_sym]
fn napi_get_instance_data(env: napi_env, result: *mut *mut c_void) -> Result {
  let env = &mut *(env as *mut Env);
  let shared = env.shared();
  *result = shared.instance_data;
  Ok(())
}
