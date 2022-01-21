use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_get_new_target(
  env: napi_env,
  result: *mut napi_value,
) -> napi_status {
  todo!()
}
