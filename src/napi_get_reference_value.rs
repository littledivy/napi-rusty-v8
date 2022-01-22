use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_get_reference_value(
  env: napi_env,
  reference: napi_ref,
  result: *mut napi_value,
) -> napi_status {
  // TODO
  *result = std::mem::transmute(reference);
  napi_ok
}
