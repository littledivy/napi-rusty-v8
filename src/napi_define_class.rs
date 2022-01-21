use crate::callback_info::CallbackInfo;
use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_define_class(
  env: napi_env,
  utf8name: *const c_char,
  length: usize,
  constructor: napi_callback,
  callback_data: *mut c_void,
  property_count: usize,
  properties: *const napi_property_descriptor,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let constructor =
    v8::External::new(env.scope, std::mem::transmute(constructor));
  let tpl = v8::FunctionTemplate::builder(
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

      // TODO(@littledivy): callback_data
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
  .data(constructor.into())
  .build(env.scope);

  let name_string = std::ffi::CStr::from_ptr(utf8name).to_str().unwrap();
  let name = v8::String::new(env.scope, name_string).unwrap();
  tpl.set_class_name(name);

  let napi_properties = std::slice::from_raw_parts(properties, property_count);
  for p in napi_properties {
    println!("{:?}", p);
  }

  napi_ok
}
