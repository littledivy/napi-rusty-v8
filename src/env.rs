use crate::ffi::*;
use deno_core::v8;

#[repr(C)]
#[derive(Debug)]
/// Env that is shared between all contexts in same native module.
pub struct EnvShared {
  pub instance_data: *mut c_void,
  pub data_finalize: Option<napi_finalize>,
  pub data_finalize_hint: *mut c_void,
  pub napi_wrap: v8::Global<v8::Private>,
  pub finalize: Option<napi_finalize>,
  pub finalize_hint: *mut c_void,
  pub filename: *const c_char,
}

impl EnvShared {
  pub fn new(napi_wrap: v8::Global<v8::Private>) -> Self {
    Self {
      instance_data: std::ptr::null_mut(),
      data_finalize: None,
      data_finalize_hint: std::ptr::null_mut(),
      napi_wrap,
      finalize: None,
      finalize_hint: std::ptr::null_mut(),
      filename: std::ptr::null(),
    }
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct Env<'a, 'b, 'c> {
  pub scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
  pub open_handle_scopes: usize,
  pub shared: *mut EnvShared,
}

impl<'a, 'b, 'c> Env<'a, 'b, 'c> {
  pub fn new(scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>) -> Self {
    Self {
      scope,
      shared: std::ptr::null_mut(),
      open_handle_scopes: 0,
    }
  }

  pub fn with_new_scope(
    &self,
    scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
  ) -> Self {
    Self {
      scope,
      shared: self.shared,
      open_handle_scopes: self.open_handle_scopes,
    }
  }

  pub fn shared(&self) -> &EnvShared {
    unsafe { &*self.shared }
  }

  pub fn shared_mut(&self) -> &mut EnvShared {
    unsafe { &mut *self.shared }
  }
}
