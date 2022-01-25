use crate::ffi::*;
use crate::r#async::AsyncWork;

#[napi_sym::napi_sym]
fn napi_queue_async_work(
  _env: napi_env,
  work: napi_async_work,
) -> Result<(), ()> {
  AsyncWork::queue(work);
  Ok(())
}
