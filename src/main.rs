#![allow(non_camel_case_types)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]

#[cfg(unix)]
use libloading::os::unix::*;

#[cfg(windows)]
use libloading::os::windows::*;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_void;
use std::ptr;

struct Env<'a, 'b, 'c> {
  scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
}

type napi_status = i32;
type napi_env = *mut c_void;
type napi_value = *mut c_void;
type napi_callback_info = *mut c_void;

const napi_ok: napi_status = 0;

pub type napi_callback = Option<
  unsafe extern "C" fn(env: napi_env, info: napi_callback_info) -> napi_value,
>;

// default = 0
pub type napi_property_attributes = i32;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct napi_property_descriptor {
  pub utf8name: *const c_char,
  pub name: napi_value,
  pub method: napi_callback,
  pub getter: napi_callback,
  pub setter: napi_callback,
  pub value: napi_value,
  pub attributes: napi_property_attributes,
  pub data: *mut c_void,
}

#[no_mangle]
pub unsafe extern "C" fn napi_define_properties(
  env: napi_env,
  obj: napi_value,
  property_count: usize,
  properties: *const napi_property_descriptor,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let object: v8::Local<v8::Object> = *(obj as *mut v8::Local<v8::Object>);
  let properties = std::slice::from_raw_parts(properties, property_count);
  
  for property in properties {
    let name = CStr::from_ptr(property.utf8name).to_str().unwrap();
    let name = v8::String::new(env.scope, name).unwrap();

    if let Some(method) = property.method {
      let method_ptr =
        std::mem::transmute::<napi_callback, *mut c_void>(Some(method));

      let method_ptr = v8::External::new(env.scope, method_ptr);

      let function = v8::Function::builder(
        |handle_scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          let data = args.data().unwrap();
          let method_ptr = v8::Local::<v8::External>::try_from(data).unwrap();

          let method = std::mem::transmute::<*mut c_void, napi_callback>(
            method_ptr.value(),
          )
          .unwrap();

          let context = v8::Context::new(handle_scope);
          let scope = &mut v8::ContextScope::new(handle_scope, context);

          let mut env = Env { scope };
          let env_ptr = &mut env as *mut _ as *mut c_void;

          let value = unsafe { method(env_ptr, ptr::null_mut()) };
          let value = *(value as *mut v8::Local<v8::Value>);

          rv.set(value);
        },
      )
      .data(method_ptr.into())
      .build(env.scope)
      .unwrap();
      object.set(env.scope, name.into(), function.into()).unwrap();
    }
  }

  std::mem::forget(env);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_string_utf8(
  env: napi_env,
  string: *const u8,
  length: usize,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let string = std::slice::from_raw_parts(string, length);
  let string = std::str::from_utf8(string).unwrap();
  let v8str = v8::String::new(env.scope, string).unwrap();
  let mut value: v8::Local<v8::Value> = v8str.into();
  *result = &mut value as *mut _ as napi_value;
  std::mem::forget(env);
  napi_ok
}

// #[no_mangle]
// pub unsafe extern "C" fn napi_module_register() -> napi_status {
//   // no-op.
//   napi_ok
// }

#[no_mangle]
pub unsafe extern "C" fn napi_set_named_property(
  env: napi_env,
  object: napi_value,
  name: *const c_char,
  value: napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  env.scope.enter();
  let object: v8::Local<v8::Object> = *(object as *mut v8::Local<v8::Object>);
  let name = CStr::from_ptr(name).to_str().unwrap();
  let fnval = *(value as *const v8::Local<v8::Value>);
  let name = v8::String::new(env.scope, name).unwrap();
  // seg faults when both are commented out...?
  // println!("a"); // remove comment to get "TypeError: exports.hello is not a function"
  object.set(env.scope, name.into(), fnval).unwrap();
  // println!("b"); // remove comment to make it work
  env.scope.exit();
  std::mem::forget(env);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_function(
  env: napi_env,
  string: *const u8,
  length: usize,
  cb: napi_callback,
  cb_data: napi_callback_info,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  env.scope.enter();

  let fn_ptr = std::mem::transmute::<napi_callback, *mut c_void>(cb);
  let fn_ptr = v8::External::new(env.scope, fn_ptr);

  let function = v8::Function::builder(
    |handle_scope: &mut v8::HandleScope,
     args: v8::FunctionCallbackArguments,
     mut rv: v8::ReturnValue| {
      let data = args.data().unwrap();
      let fn_ptr = v8::Local::<v8::External>::try_from(data).unwrap();

      let method =
        std::mem::transmute::<*mut c_void, napi_callback>(fn_ptr.value())
          .unwrap();

      let context = v8::Context::new(handle_scope);
      let scope = &mut v8::ContextScope::new(handle_scope, context);

      let mut env = Env { scope };
      let env_ptr = &mut env as *mut _ as *mut c_void;

      // TODO: why cant i pass cb_data instead of null_mut??
      let value = unsafe { method(env_ptr, ptr::null_mut()) };
      let value = *(value as *mut v8::Local<v8::Value>);

      rv.set(value);
    },
  )
  .data(fn_ptr.into())
  .build(env.scope)
  .unwrap();

  let string = std::slice::from_raw_parts(string, length);
  let string = std::str::from_utf8(string).unwrap();
  let v8str = v8::String::new(env.scope, string).unwrap();
  function.set_name(v8str);

  let mut value: v8::Local<v8::Value> = function.into();
  // Why does it only work when I comment this out??
  // println!("a");
  *result = &mut value as *mut _ as napi_value;
  env.scope.exit();
  std::mem::forget(env);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_undefined(
  env: napi_env,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let mut value: v8::Local<v8::Value> = v8::undefined(env.scope).into();
  *result = &mut value as *mut _ as napi_value;
  std::mem::forget(env);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_throw(env: napi_env, error: napi_value) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  println!("stub: napi_throw");
  std::mem::forget(env);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_reference() {
  todo!()
}

#[no_mangle]
pub unsafe extern "C" fn napi_define_class() {
  todo!()
}

#[no_mangle]
pub unsafe extern "C" fn napi_throw_error() {
  todo!()
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_error(
  env: napi_env,
  code: napi_value,
  msg: napi_value,
  result: *mut napi_value,
) {
  todo!()
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_object() {
  todo!()
}

fn main() {
  // Initialize V8.
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

  let mut handle_scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(handle_scope);
  let scope = &mut v8::ContextScope::new(handle_scope, context);

  let mut exports = v8::Object::new(scope);
  let mut env = Env { scope };

  #[cfg(unix)]
  let flags = RTLD_LAZY;
  #[cfg(not(unix))]
  let flags = 0x00000008;

  // Initializer callback.
  #[cfg(unix)]
  let library = unsafe {
    Library::open(
      Some("./example_module/target/release/libexample_module.so"),
      flags,
    )
    .unwrap()
  };
  
  #[cfg(not(unix))]
  let library = unsafe {
    Library::load_with_flags(
      "./example_module/target/release/example_module.dll",
      flags,
    )
    .unwrap()
  };

  let init = unsafe {
    library.get::<unsafe extern "C" fn(env: napi_env, exports: napi_value) -> napi_value>(b"napi_register_module_v1").unwrap()
  };
  
  unsafe {
    init(
      &mut env as *mut _ as *mut c_void,
      &mut exports as *mut _ as *mut c_void,
    )
  };
  
  let exports_str = v8::String::new(scope, "exports").unwrap();

  context
    .global(scope)
    .set(scope, exports_str.into(), exports.into())
    .unwrap();

  let script = v8::String::new(scope, "exports.hello()").unwrap();

  let script =
    v8::Script::compile(scope, script, None).expect("failed to compile script");

  script.run(scope).unwrap();
}
