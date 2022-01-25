use crate::env::Env;
use crate::ffi::*;
use crate::r#async::AsyncWork;
use deno_core::v8;

#[napi_sym::napi_sym]
fn napi_create_async_work(
  env_ptr: napi_env,
  async_resource: napi_value,
  async_resource_name: napi_value,
  execute: napi_async_execute_callback,
  complete: napi_async_complete_callback,
  data: *mut c_void,
  result: *mut napi_async_work,
) -> Result<(), ()> {
  let env = &mut *(env_ptr as *mut Env);
  let resource = if async_resource.is_null() {
    v8::Object::new(env.scope)
  } else {
    transmute(async_resource)
  };

  let resource_name: v8::Local<v8::String> = transmute(async_resource_name);
  let work_ptr = std::alloc::alloc(std::alloc::Layout::new::<AsyncWork>());
  let mut work = AsyncWork {
    env: env_ptr,
    data,
    execute,
    complete,
  };
  (work_ptr as *mut AsyncWork).write(work);
  *result = work_ptr as napi_async_work;
  Ok(())
}
