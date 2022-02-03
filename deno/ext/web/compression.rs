// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_core::error::AnyError;
use deno_core::OpState;
use deno_core::Resource;
use deno_core::ResourceId;
use deno_core::ZeroCopyBuf;
use flate2::write::GzDecoder;
use flate2::write::GzEncoder;
use flate2::write::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::borrow::Cow;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

#[derive(Debug)]
struct CompressionResource(RefCell<Inner>);

#[derive(Debug)]
enum Inner {
  DeflateDecoder(ZlibDecoder<Vec<u8>>),
  DeflateEncoder(ZlibEncoder<Vec<u8>>),
  GzDecoder(GzDecoder<Vec<u8>>),
  GzEncoder(GzEncoder<Vec<u8>>),
}

impl Resource for CompressionResource {
  fn name(&self) -> Cow<str> {
    "compression".into()
  }
}

pub fn op_compression_new(
  state: &mut OpState,
  format: String,
  is_decoder: bool,
) -> Result<ResourceId, AnyError> {
  let w = Vec::new();
  let inner = match (format.as_str(), is_decoder) {
    ("deflate", true) => Inner::DeflateDecoder(ZlibDecoder::new(w)),
    ("deflate", false) => {
      Inner::DeflateEncoder(ZlibEncoder::new(w, Compression::default()))
    }
    ("gzip", true) => Inner::GzDecoder(GzDecoder::new(w)),
    ("gzip", false) => {
      Inner::GzEncoder(GzEncoder::new(w, Compression::default()))
    }
    _ => unreachable!(),
  };
  let resource = CompressionResource(RefCell::new(inner));
  Ok(state.resource_table.add(resource))
}

pub fn op_compression_write(
  state: &mut OpState,
  rid: ResourceId,
  input: ZeroCopyBuf,
) -> Result<ZeroCopyBuf, AnyError> {
  let resource = state.resource_table.get::<CompressionResource>(rid)?;
  let mut inner = resource.0.borrow_mut();
  let out: Vec<u8> = match &mut *inner {
    Inner::DeflateDecoder(d) => {
      d.write_all(&input)?;
      d.flush()?;
      d.get_mut().drain(..)
    }
    Inner::DeflateEncoder(d) => {
      d.write_all(&input)?;
      d.flush()?;
      d.get_mut().drain(..)
    }
    Inner::GzDecoder(d) => {
      d.write_all(&input)?;
      d.flush()?;
      d.get_mut().drain(..)
    }
    Inner::GzEncoder(d) => {
      d.write_all(&input)?;
      d.flush()?;
      d.get_mut().drain(..)
    }
  }
  .collect();
  Ok(out.into())
}

pub fn op_compression_finish(
  state: &mut OpState,
  rid: ResourceId,
  _: (),
) -> Result<ZeroCopyBuf, AnyError> {
  let resource = state.resource_table.take::<CompressionResource>(rid)?;
  let resource = Rc::try_unwrap(resource).unwrap();
  let inner = resource.0.into_inner();
  let out: Vec<u8> = match inner {
    Inner::DeflateDecoder(d) => d.finish()?,
    Inner::DeflateEncoder(d) => d.finish()?,
    Inner::GzDecoder(d) => d.finish()?,
    Inner::GzEncoder(d) => d.finish()?,
  };
  Ok(out.into())
}
