use crate::ffi::*;
use crate::napi_create_async_work::AsyncWork;

#[napi_sym::napi_sym]
fn napi_queue_async_work(
  env: napi_env,
  work: napi_async_work,
) -> Result<(), ()> {
  let work = &mut *(work as *mut AsyncWork);
  // FIXME: Call this from tokio thread pool
  (work.execute)(env, work.data);

  // Note: Must be called from the loop thread.
  (work.complete)(env, napi_ok, work.data);

  Ok(())
}
