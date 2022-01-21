use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_module_register() -> napi_status {
  // no-op.
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn node_module_register() -> napi_status {
  // no-op.
  napi_ok
}
