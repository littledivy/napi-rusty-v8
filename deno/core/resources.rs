// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

// Think of Resources as File Descriptors. They are integers that are allocated
// by the privileged side of Deno which refer to various rust objects that need
// to be persisted between various ops. For example, network sockets are
// resources. Resources may or may not correspond to a real operating system
// file descriptor (hence the different name).

use crate::error::bad_resource_id;
use crate::error::not_supported;
use crate::ZeroCopyBuf;
use anyhow::Error;
use futures::Future;
use std::any::type_name;
use std::any::Any;
use std::any::TypeId;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::iter::Iterator;
use std::pin::Pin;
use std::rc::Rc;

/// Returned by resource read/write/shutdown methods
pub type AsyncResult<T> = Pin<Box<dyn Future<Output = Result<T, Error>>>>;

/// All objects that can be store in the resource table should implement the
/// `Resource` trait.
/// TODO(@AaronO): investigate avoiding alloc on read/write/shutdown
pub trait Resource: Any + 'static {
  /// Returns a string representation of the resource which is made available
  /// to JavaScript code through `op_resources`. The default implementation
  /// returns the Rust type name, but specific resource types may override this
  /// trait method.
  fn name(&self) -> Cow<str> {
    type_name::<Self>().into()
  }

  /// Resources may implement `read()` to be a readable stream
  fn read(self: Rc<Self>, _buf: ZeroCopyBuf) -> AsyncResult<usize> {
    Box::pin(futures::future::err(not_supported()))
  }

  /// Resources may implement `write()` to be a writable stream
  fn write(self: Rc<Self>, _buf: ZeroCopyBuf) -> AsyncResult<usize> {
    Box::pin(futures::future::err(not_supported()))
  }

  /// Resources may implement `shutdown()` for graceful async shutdowns
  fn shutdown(self: Rc<Self>) -> AsyncResult<()> {
    Box::pin(futures::future::err(not_supported()))
  }

  /// Resources may implement the `close()` trait method if they need to do
  /// resource specific clean-ups, such as cancelling pending futures, after a
  /// resource has been removed from the resource table.
  fn close(self: Rc<Self>) {}
}

impl dyn Resource {
  #[inline(always)]
  fn is<T: Resource>(&self) -> bool {
    self.type_id() == TypeId::of::<T>()
  }

  #[inline(always)]
  #[allow(clippy::needless_lifetimes)]
  pub fn downcast_rc<'a, T: Resource>(self: &'a Rc<Self>) -> Option<&'a Rc<T>> {
    if self.is::<T>() {
      let ptr = self as *const Rc<_> as *const Rc<T>;
      Some(unsafe { &*ptr })
    } else {
      None
    }
  }
}

/// A `ResourceId` is an integer value referencing a resource. It could be
/// considered to be the Deno equivalent of a `file descriptor` in POSIX like
/// operating systems. Elsewhere in the code base it is commonly abbreviated
/// to `rid`.
// TODO: use `u64` instead?
pub type ResourceId = u32;

/// Map-like data structure storing Deno's resources (equivalent to file
/// descriptors).
///
/// Provides basic methods for element access. A resource can be of any type.
/// Different types of resources can be stored in the same map, and provided
/// with a name for description.
///
/// Each resource is identified through a _resource ID (rid)_, which acts as
/// the key in the map.
#[derive(Default)]
pub struct ResourceTable {
  index: BTreeMap<ResourceId, Rc<dyn Resource>>,
  next_rid: ResourceId,
}

impl ResourceTable {
  /// Inserts resource into the resource table, which takes ownership of it.
  ///
  /// The resource type is erased at runtime and must be statically known
  /// when retrieving it through `get()`.
  ///
  /// Returns a unique resource ID, which acts as a key for this resource.
  pub fn add<T: Resource>(&mut self, resource: T) -> ResourceId {
    self.add_rc(Rc::new(resource))
  }

  /// Inserts a `Rc`-wrapped resource into the resource table.
  ///
  /// The resource type is erased at runtime and must be statically known
  /// when retrieving it through `get()`.
  ///
  /// Returns a unique resource ID, which acts as a key for this resource.
  pub fn add_rc<T: Resource>(&mut self, resource: Rc<T>) -> ResourceId {
    let resource = resource as Rc<dyn Resource>;
    let rid = self.next_rid;
    let removed_resource = self.index.insert(rid, resource);
    assert!(removed_resource.is_none());
    self.next_rid += 1;
    rid
  }

  /// Returns true if any resource with the given `rid` exists.
  pub fn has(&self, rid: ResourceId) -> bool {
    self.index.contains_key(&rid)
  }

  /// Returns a reference counted pointer to the resource of type `T` with the
  /// given `rid`. If `rid` is not present or has a type different than `T`,
  /// this function returns `None`.
  pub fn get<T: Resource>(&self, rid: ResourceId) -> Result<Rc<T>, Error> {
    self
      .index
      .get(&rid)
      .and_then(|rc| rc.downcast_rc::<T>())
      .map(Clone::clone)
      .ok_or_else(bad_resource_id)
  }

  pub fn get_any(&self, rid: ResourceId) -> Result<Rc<dyn Resource>, Error> {
    self
      .index
      .get(&rid)
      .map(Clone::clone)
      .ok_or_else(bad_resource_id)
  }

  /// Removes a resource of type `T` from the resource table and returns it.
  /// If a resource with the given `rid` exists but its type does not match `T`,
  /// it is not removed from the resource table. Note that the resource's
  /// `close()` method is *not* called.
  pub fn take<T: Resource>(&mut self, rid: ResourceId) -> Result<Rc<T>, Error> {
    let resource = self.get::<T>(rid)?;
    self.index.remove(&rid);
    Ok(resource)
  }

  /// Removes a resource from the resource table and returns it. Note that the
  /// resource's `close()` method is *not* called.
  pub fn take_any(
    &mut self,
    rid: ResourceId,
  ) -> Result<Rc<dyn Resource>, Error> {
    self.index.remove(&rid).ok_or_else(bad_resource_id)
  }

  /// Removes the resource with the given `rid` from the resource table. If the
  /// only reference to this resource existed in the resource table, this will
  /// cause the resource to be dropped. However, since resources are reference
  /// counted, therefore pending ops are not automatically cancelled. A resource
  /// may implement the `close()` method to perform clean-ups such as canceling
  /// ops.
  pub fn close(&mut self, rid: ResourceId) -> Result<(), Error> {
    self
      .index
      .remove(&rid)
      .ok_or_else(bad_resource_id)
      .map(|resource| resource.close())
  }

  /// Returns an iterator that yields a `(id, name)` pair for every resource
  /// that's currently in the resource table. This can be used for debugging
  /// purposes or to implement the `op_resources` op. Note that the order in
  /// which items appear is not specified.
  ///
  /// # Example
  ///
  /// ```
  /// # use deno_core::ResourceTable;
  /// # let resource_table = ResourceTable::default();
  /// let resource_names = resource_table.names().collect::<Vec<_>>();
  /// ```
  pub fn names(&self) -> impl Iterator<Item = (ResourceId, Cow<str>)> {
    self
      .index
      .iter()
      .map(|(&id, resource)| (id, resource.name()))
  }
}
