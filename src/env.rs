use deno_core::v8;
use crate::ffi::*;

#[repr(C)]
#[derive(Debug)]
/// Env that is shared between all contexts in same native module.
pub struct EnvShared {
  pub status: napi_status,
  pub error_message: Option<String>,
  pub instance_data: *mut c_void,
  pub data_finalize: Option<napi_finalize>,
  pub data_finalize_hint: *mut c_void,
  pub napi_wrap: v8::Global<v8::Private>,
  pub finalize: Option<napi_finalize>,
  pub finalize_hint: *mut c_void,
}

impl EnvShared {
  pub fn new(napi_wrap: v8::Global<v8::Private>) -> Self {
    Self {
      status: napi_ok,
      error_message: None,
      instance_data: std::ptr::null_mut(),
      data_finalize: None,
      data_finalize_hint: std::ptr::null_mut(),
      napi_wrap,
      finalize: None,
      finalize_hint: std::ptr::null_mut(),
    }
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct Env<'a, 'b, 'c> {
  pub scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
  pub shared: *mut EnvShared,
}

impl<'a, 'b, 'c> Env<'a, 'b, 'c> {
  pub fn new(scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>) -> Self {
    Self { scope, shared: std::ptr::null_mut() }
  }

  pub fn with_new_scope(
    &self,
    scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
  ) -> Self {
    Self { scope, shared: self.shared }
  }

  pub fn shared(&self) -> &EnvShared {
    unsafe { &*self.shared }
  }
  
  pub fn shared_mut(&self) -> &mut EnvShared {
    unsafe { &mut *self.shared }
  }

  pub fn ok(&self) -> napi_status {
    let shared = self.shared_mut();
    shared.status = napi_ok;
    shared.error_message = None;
    shared.status
  }

  pub fn set_status(&mut self, status: napi_status) -> napi_status {
    let shared = self.shared_mut();
    shared.status = status;
    shared.status
  }

  pub fn error(&mut self, message: &str) -> napi_status {
    let shared = self.shared_mut();
    shared.status = napi_generic_failure;
    shared.error_message = Some(message.to_string());
    shared.status
  }
}
