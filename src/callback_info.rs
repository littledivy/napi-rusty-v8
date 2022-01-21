use crate::ffi::*;

#[repr(C)]
#[derive(Debug)]
pub struct CallbackInfo {
  pub env: napi_env,
  pub cb: napi_callback,
  pub cb_info: napi_callback_info,
  pub args: *const c_void,
}
