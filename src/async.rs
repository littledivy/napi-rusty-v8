use std::collections::HashSet;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Poll;

use deno_core::v8;

use crate::env::Env;
use crate::ffi::*;

#[repr(C)]
#[derive(Debug)]
pub struct AsyncWork {
  pub env: napi_env,
  pub data: *mut c_void,
  pub execute: napi_async_execute_callback,
  pub complete: napi_async_complete_callback,
}

unsafe impl Send for AsyncWork {}
unsafe impl Sync for AsyncWork {}

struct WorkWrapper(napi_async_work);

unsafe impl Send for WorkWrapper {}
unsafe impl Sync for WorkWrapper {}

impl AsyncWork {
  pub fn queue(work: napi_async_work) {
    let thread_pool: usize = unsafe {
      let work_ptr = work;
      let work = &*(work as *const AsyncWork);
      let env = &*(work.env as *mut Env);
      let shared = env.shared();
      let thread_pool = &mut *shared.thread_pool;
      thread_pool.queue.insert(work_ptr);
      transmute(shared.thread_pool)
    };
    let data = Arc::new(Mutex::new(WorkWrapper(work)));
    let shared = Arc::clone(&data);

    tokio::task::spawn_blocking(move || {
      let work = shared.lock().unwrap();
      let work_ptr = (*work).0;
      let work = unsafe { &*(work_ptr as *const AsyncWork) };
      let thread_pool = unsafe { &mut *(thread_pool as *mut AsyncThreadPool) };
      unsafe { (work.execute)(work.env, work.data) };
      thread_pool.tx.send(work_ptr).unwrap();
    });
  }

  pub fn cancel() {
    // TODO
  }
}

#[repr(C)]
#[derive(Debug)]
pub struct AsyncThreadPool {
  pub tx: Sender<napi_async_work>,
  pub rx: Receiver<napi_async_work>,
  pub queue: HashSet<napi_async_work>,
}

impl AsyncThreadPool {
  pub fn new() -> Self {
    let (tx, rx) = std::sync::mpsc::channel::<napi_async_work>();
    Self {
      tx,
      rx,
      queue: HashSet::new(),
    }
  }

  pub fn poll(&mut self, scope: &mut v8::ContextScope<v8::HandleScope>) -> Poll<()> {
    while let Ok(work_ptr) = self.rx.try_recv() {
      self.queue.remove(&work_ptr);
      let work = unsafe { &*(work_ptr as *const AsyncWork) };
      println!("{:?}", work);
      unsafe {
        let env = &*(work.env as *mut Env);
        let mut env = env.with_new_scope(scope);
        (work.complete)(transmute(&mut env), napi_ok, work.data);
        println!("complete cb");
        std::alloc::dealloc(work_ptr as *mut u8, std::alloc::Layout::new::<AsyncWork>());
      }
    }
    if self.queue.is_empty() {
      Poll::Ready(())
    } else {
      Poll::Pending
    }
  }
}
