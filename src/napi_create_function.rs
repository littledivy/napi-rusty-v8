use crate::callback_info::CallbackInfo;
use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

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
