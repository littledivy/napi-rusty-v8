use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Poll;

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
    println!("AsyncWork queue start");
    let poller: usize = unsafe {
      let work = &*(work as *const AsyncWork);
      let env = &*(work.env as *mut Env);
      let shared = env.shared();
      transmute(shared.poller)
    };
    let data = Arc::new(Mutex::new(WorkWrapper(work)));
    let shared = Arc::clone(&data);

    println!("AsyncWork spawn");
    tokio::task::spawn_blocking(move || {
      println!("AsyncWork task start");
      let work = shared.lock().unwrap();
      let work_ptr = (*work).0;
      let work = unsafe { &*(work_ptr as *const AsyncWork) };
      println!("AsyncWork task execute");
      unsafe { (work.execute)(work.env, work.data) };
      println!("AsyncWork task complete");
      let poller = unsafe { &*(poller as *const Sender<napi_async_work>) };
      println!("AsyncWork task send");
      poller.send(work_ptr).unwrap();
      println!("AsyncWork task end");
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
  pub queue: Vec<napi_async_work>,
}

impl AsyncThreadPool {
  pub fn new() -> Self {
    let (tx, rx) = std::sync::mpsc::channel::<napi_async_work>();
    Self {
      tx,
      rx,
      queue: Vec::new(),
    }
  }

  pub fn poll(&mut self) -> Poll<()> {
    while let Ok(work_ptr) = self.rx.try_recv() {
      for i in 0..self.queue.len() {
        if self.queue[i] == work_ptr {
          self.queue.remove(i);
          break;
        }
      }
      let work = unsafe { &*(work_ptr as *const AsyncWork) };
      println!("complete work");
      unsafe {
        (work.complete)(work.env, napi_ok, work.data);
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
