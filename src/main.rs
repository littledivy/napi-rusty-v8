#![allow(non_camel_case_types)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

#[cfg(unix)]
use libloading::os::unix::*;

#[cfg(windows)]
use libloading::os::windows::*;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_void;
use std::ptr;

use deno_core::v8;
use deno_core::JsRuntime;

struct Env<'a, 'b, 'c> {
  scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
}

type napi_status = i32;
type napi_env = *mut c_void;
type napi_value = *mut c_void;
type napi_callback_info = *mut c_void;

const napi_ok: napi_status = 0;
const napi_invalid_arg: napi_status = 1;
const napi_object_expected: napi_status = 2;
const napi_string_expected: napi_status = 3;
const napi_name_expected: napi_status = 4;
const napi_function_expected: napi_status = 5;
const napi_number_expected: napi_status = 6;
const napi_boolean_expected: napi_status = 7;
const napi_array_expected: napi_status = 8;
const napi_generic_failure: napi_status = 9;
const napi_pending_exception: napi_status = 10;
const napi_cancelled: napi_status = 11;
const napi_escape_called_twice: napi_status = 12;
const napi_handle_scope_mismatch: napi_status = 13;
const napi_callback_scope_mismatch: napi_status = 14;
const napi_queue_full: napi_status = 15;
const napi_closing: napi_status = 16;
const napi_bigint_expected: napi_status = 17;
const napi_date_expected: napi_status = 18;
const napi_arraybuffer_expected: napi_status = 19;
const napi_detachable_arraybuffer_expected: napi_status = 20;
const napi_would_deadlock: napi_status = 21;

pub type napi_callback =
  unsafe extern "C" fn(env: napi_env, info: napi_callback_info) -> napi_value;

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

#[repr(C)]
#[derive(Debug)]
struct CallbackInfo {
  env: napi_env,
  cb: napi_callback,
  cb_info: napi_callback_info,
  args: *const c_void,
}

#[no_mangle]
pub unsafe extern "C" fn napi_define_properties(
  env: napi_env,
  obj: napi_value,
  property_count: usize,
  properties: *const napi_property_descriptor,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  env.scope.enter();

  let object: v8::Local<v8::Object> = std::mem::transmute(obj);
  let properties = std::slice::from_raw_parts(properties, property_count);

  for property in properties {
    let name = CStr::from_ptr(property.utf8name).to_str().unwrap();

    let name = v8::String::new(env.scope, name).unwrap();

    let method_ptr: *mut c_void = std::mem::transmute(property.method);

    if !method_ptr.is_null() {
      let method_ptr = v8::External::new(env.scope, method_ptr);

      let function = v8::Function::builder(
        |handle_scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          let data = args.data().unwrap();
          let method_ptr = v8::Local::<v8::External>::try_from(data).unwrap();
          let method: napi_callback = std::mem::transmute(method_ptr.value());

          let context = v8::Context::new(handle_scope);
          let scope = &mut v8::ContextScope::new(handle_scope, context);

          let mut env = Env { scope };
          let env_ptr = &mut env as *mut _ as *mut c_void;

          let mut info = CallbackInfo {
            env: env_ptr,
            cb: method,
            cb_info: ptr::null_mut(),
            args: &args as *const _ as *const c_void,
          };

          let info_ptr = &mut info as *mut _ as *mut c_void;

          let value = unsafe { method(env_ptr, info_ptr) };
          let value = std::mem::transmute(value);

          rv.set(value);
        },
      )
      .data(method_ptr.into())
      .build(env.scope)
      .unwrap();

      object.set(env.scope, name.into(), function.into()).unwrap();
    }
  }

  env.scope.exit();
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
  let value: v8::Local<v8::Value> = v8str.into();
  *result = std::mem::transmute(value);

  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_module_register() -> napi_status {
  // no-op.
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_set_named_property(
  env: napi_env,
  object: napi_value,
  name: *const c_char,
  value: napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let object: v8::Local<v8::Object> = std::mem::transmute(object);
  let name = CStr::from_ptr(name).to_str().unwrap();
  let value: v8::Local<v8::Value> = std::mem::transmute(value);

  let name = v8::String::new(env.scope, name).unwrap();
  object.set(env.scope, name.into(), value).unwrap();
  
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_function(
  env: napi_env,
  string: *const u8,
  length: usize,
  cb: napi_callback,
  cb_info: napi_callback_info,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let method_ptr = v8::External::new(env.scope, std::mem::transmute(cb));

  let function = v8::Function::builder(
    |handle_scope: &mut v8::HandleScope,
     args: v8::FunctionCallbackArguments,
     mut rv: v8::ReturnValue| {
      let data = args.data().unwrap();
      let method_ptr = v8::Local::<v8::External>::try_from(data).unwrap();
      let method: napi_callback = std::mem::transmute(method_ptr.value());

      let context = v8::Context::new(handle_scope);
      let scope = &mut v8::ContextScope::new(handle_scope, context);

      let mut env = Env { scope };
      let env_ptr = &mut env as *mut _ as *mut c_void;

      let mut info = CallbackInfo {
        env: env_ptr,
        cb: method,
        // why does it not work..?
        // cb_info,
        // but this works
        cb_info: ptr::null_mut(),
        args: &args as *const _ as *const c_void,
      };

      let info_ptr = &mut info as *mut _ as *mut c_void;

      let value = unsafe { method(env_ptr, info_ptr) };
      let value = std::mem::transmute(value);
      rv.set(value);
    },
  )
  .data(method_ptr.into())
  .build(env.scope)
  .unwrap();

  if !string.is_null() {
    let string = std::slice::from_raw_parts(string, length);
    let string = std::str::from_utf8(string).unwrap();

    let v8str = v8::String::new(env.scope, string).unwrap();
    function.set_name(v8str);
  }

  let value: v8::Local<v8::Value> = function.into();
  *result = std::mem::transmute(value);

  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_undefined(
  env: napi_env,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::undefined(env.scope).into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_value_string_utf8(
  env: napi_env,
  value: napi_value,
  result: *mut c_char,
  result_len: usize,
  result_copied: *mut usize,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let value: v8::Local<v8::Value> = std::mem::transmute(value);

  if !value.is_string() && !value.is_string_object() {
    return napi_string_expected;
  }

  let v8str = value.to_string(env.scope).unwrap();
  let string_len = v8str.utf8_length(env.scope);

  let string = v8str.to_rust_string_lossy(env.scope);

  let string = string.as_bytes();
  let string = string.as_ptr();
  let string = string as *const c_char;

  *result_copied = string_len;

  if result_len < string_len {
    return napi_ok;
  }

  if !result.is_null() {
    std::ptr::copy_nonoverlapping(string, result, string_len as usize);
    *result.offset(string_len as isize) = 0;
  }

  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_throw(
  env: napi_env,
  error: napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let error = std::mem::transmute(error);
  env.scope.throw_exception(error);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_cb_info(
  env: napi_env,
  cbinfo: napi_callback_info,
  argc: *mut i32,
  argv: *mut napi_value,
  this_arg: *mut napi_value,
  cb_data: *mut *mut c_void,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let cbinfo: &CallbackInfo = &*(cbinfo as *const CallbackInfo);
  let args = &*(cbinfo.args as *const v8::FunctionCallbackArguments);

  if !cb_data.is_null() {
    *cb_data = cbinfo.cb_info;
  }

  if !this_arg.is_null() {
    let mut this: v8::Local<v8::Value> = args.this().into();
    *this_arg = std::mem::transmute(this);
  }

  let len = args.length();
  let mut v_argc = len;
  if !argc.is_null() {
    v_argc = *argc;
    *argc = len;
  }

  if !argv.is_null() {
    let mut v_argv = std::slice::from_raw_parts_mut(argv, v_argc as usize);
    for i in 0..v_argc {
      let mut arg = args.get(i);
      v_argv[i as usize] = std::mem::transmute(arg);
    }
  }

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
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  
  let code: v8::Local<v8::Value> = std::mem::transmute(code);
  let msg: v8::Local<v8::Value> = std::mem::transmute(msg);

  let msg = msg.to_string(env.scope).unwrap();

  let error = v8::Exception::error(env.scope, msg);
  *result = std::mem::transmute(error);

  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_object() {
  todo!()
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_int32(
  env: napi_env,
  value: i32,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::Number::new(env.scope, value as f64).into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_uint32(
  env: napi_env,
  value: u32,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::Number::new(env.scope, value as f64).into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_int64(
  env: napi_env,
  value: i64,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::Number::new(env.scope, value as f64).into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_double(
  env: napi_env,
  value: f64,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::Number::new(env.scope, value).into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_bigint_int64(
  env: napi_env,
  value: i64,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::BigInt::new_from_i64(env.scope, value).into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_bigint_uint64(
  env: napi_env,
  value: u64,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::BigInt::new_from_u64(env.scope, value).into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_bigint_words(
  env: napi_env,
  sign_bit: bool,
  words: *const u64,
  word_count: usize,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::BigInt::new_from_words(
    env.scope,
    sign_bit,
    std::slice::from_raw_parts(words, word_count),
  )
  .unwrap()
  .into();
  *result = std::mem::transmute(value);
  napi_ok
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_value_int32(
  env: napi_env,
  value: napi_value,
  result: *mut i32,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  *result = value.int32_value(env.scope).unwrap();
  napi_ok
}

fn main() {
  let mut runtime = JsRuntime::new(Default::default());

  let isolate = runtime.v8_isolate();

  let mut scope = &mut runtime.handle_scope();
  let context = scope.get_current_context();
  let inner_scope = &mut v8::ContextScope::new(scope, context);
  let global = context.global(inner_scope);

  let mut exports = v8::Object::new(inner_scope);
  let mut env = Env { scope: inner_scope };

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
      std::mem::transmute(exports),
    )
  };

  let exports_str = v8::String::new(inner_scope, "exports").unwrap();

  global
    .set(inner_scope, exports_str.into(), exports.into())
    .unwrap();

  let script = v8::String::new(
    inner_scope,
    r#"
    function print(txt) {
      Deno.core.print(txt + "\n");
    }

    print(exports.hello("Rust"));
    print(exports.add(1, 2));
    "#,
  )
  .unwrap();

  let script = v8::Script::compile(inner_scope, script, None)
    .expect("failed to compile script");

  script.run(inner_scope).unwrap();
}
