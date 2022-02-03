use std::cell::RefCell;
use std::rc::Rc;

use crate::shared::*;

use aes::cipher::NewCipher;
use aes::BlockEncrypt;
use aes::NewBlockCipher;
use aes_gcm::aead::generic_array::typenum::U12;
use aes_gcm::aes::Aes192;
use aes_gcm::AeadInPlace;
use aes_gcm::Aes128Gcm;
use aes_gcm::Aes256Gcm;
use aes_gcm::NewAead;
use aes_gcm::Nonce;
use ctr::Ctr;

use block_modes::BlockMode;
use ctr::cipher::StreamCipher;
use ctr::flavors::Ctr128BE;

use ctr::flavors::Ctr32BE;
use ctr::flavors::Ctr64BE;
use ctr::flavors::CtrFlavor;
use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;
use rand::rngs::OsRng;
use rsa::pkcs1::FromRsaPublicKey;
use rsa::PaddingScheme;
use rsa::PublicKey;
use serde::Deserialize;
use sha1::Digest;
use sha1::Sha1;
use sha2::Sha256;
use sha2::Sha384;
use sha2::Sha512;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptOptions {
  key: RawKeyData,
  #[serde(flatten)]
  algorithm: EncryptAlgorithm,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "algorithm")]
pub enum EncryptAlgorithm {
  #[serde(rename = "RSA-OAEP")]
  RsaOaep {
    hash: ShaHash,
    #[serde(with = "serde_bytes")]
    label: Vec<u8>,
  },
  #[serde(rename = "AES-CBC", rename_all = "camelCase")]
  AesCbc {
    #[serde(with = "serde_bytes")]
    iv: Vec<u8>,
    length: usize,
  },
  #[serde(rename = "AES-GCM", rename_all = "camelCase")]
  AesGcm {
    #[serde(with = "serde_bytes")]
    iv: Vec<u8>,
    #[serde(with = "serde_bytes")]
    additional_data: Option<Vec<u8>>,
    length: usize,
    tag_length: usize,
  },
  #[serde(rename = "AES-CTR", rename_all = "camelCase")]
  AesCtr {
    #[serde(with = "serde_bytes")]
    counter: Vec<u8>,
    ctr_length: usize,
    key_length: usize,
  },
}
pub async fn op_crypto_encrypt(
  _state: Rc<RefCell<OpState>>,
  opts: EncryptOptions,
  data: ZeroCopyBuf,
) -> Result<ZeroCopyBuf, AnyError> {
  let key = opts.key;
  let fun = move || match opts.algorithm {
    EncryptAlgorithm::RsaOaep { hash, label } => {
      encrypt_rsa_oaep(key, hash, label, &data)
    }
    EncryptAlgorithm::AesCbc { iv, length } => {
      encrypt_aes_cbc(key, length, iv, &data)
    }
    EncryptAlgorithm::AesGcm {
      iv,
      additional_data,
      length,
      tag_length,
    } => encrypt_aes_gcm(key, length, tag_length, iv, additional_data, &data),
    EncryptAlgorithm::AesCtr {
      counter,
      ctr_length,
      key_length,
    } => encrypt_aes_ctr(key, key_length, &counter, ctr_length, &data),
  };
  let buf = tokio::task::spawn_blocking(fun).await.unwrap()?;
  Ok(buf.into())
}

fn encrypt_rsa_oaep(
  key: RawKeyData,
  hash: ShaHash,
  label: Vec<u8>,
  data: &[u8],
) -> Result<Vec<u8>, AnyError> {
  let label = String::from_utf8_lossy(&label).to_string();

  let public_key = key.as_rsa_public_key()?;
  let public_key = rsa::RsaPublicKey::from_pkcs1_der(&public_key)
    .map_err(|_| operation_error("failed to decode public key"))?;
  let mut rng = OsRng;
  let padding = match hash {
    ShaHash::Sha1 => PaddingScheme::OAEP {
      digest: Box::new(Sha1::new()),
      mgf_digest: Box::new(Sha1::new()),
      label: Some(label),
    },
    ShaHash::Sha256 => PaddingScheme::OAEP {
      digest: Box::new(Sha256::new()),
      mgf_digest: Box::new(Sha256::new()),
      label: Some(label),
    },
    ShaHash::Sha384 => PaddingScheme::OAEP {
      digest: Box::new(Sha384::new()),
      mgf_digest: Box::new(Sha384::new()),
      label: Some(label),
    },
    ShaHash::Sha512 => PaddingScheme::OAEP {
      digest: Box::new(Sha512::new()),
      mgf_digest: Box::new(Sha512::new()),
      label: Some(label),
    },
  };
  let encrypted = public_key
    .encrypt(&mut rng, padding, data)
    .map_err(|_| operation_error("Encryption failed"))?;
  Ok(encrypted)
}

fn encrypt_aes_cbc(
  key: RawKeyData,
  length: usize,
  iv: Vec<u8>,
  data: &[u8],
) -> Result<Vec<u8>, AnyError> {
  let key = key.as_secret_key()?;
  let ciphertext = match length {
    128 => {
      // Section 10.3 Step 2 of RFC 2315 https://www.rfc-editor.org/rfc/rfc2315
      type Aes128Cbc =
        block_modes::Cbc<aes::Aes128, block_modes::block_padding::Pkcs7>;

      let cipher = Aes128Cbc::new_from_slices(key, &iv)?;
      cipher.encrypt_vec(data)
    }
    192 => {
      // Section 10.3 Step 2 of RFC 2315 https://www.rfc-editor.org/rfc/rfc2315
      type Aes192Cbc =
        block_modes::Cbc<aes::Aes192, block_modes::block_padding::Pkcs7>;

      let cipher = Aes192Cbc::new_from_slices(key, &iv)?;
      cipher.encrypt_vec(data)
    }
    256 => {
      // Section 10.3 Step 2 of RFC 2315 https://www.rfc-editor.org/rfc/rfc2315
      type Aes256Cbc =
        block_modes::Cbc<aes::Aes256, block_modes::block_padding::Pkcs7>;

      let cipher = Aes256Cbc::new_from_slices(key, &iv)?;
      cipher.encrypt_vec(data)
    }
    _ => return Err(type_error("invalid length")),
  };
  Ok(ciphertext)
}

fn encrypt_aes_gcm(
  key: RawKeyData,
  length: usize,
  tag_length: usize,
  iv: Vec<u8>,
  additional_data: Option<Vec<u8>>,
  data: &[u8],
) -> Result<Vec<u8>, AnyError> {
  let key = key.as_secret_key()?;
  let additional_data = additional_data.unwrap_or_default();

  // Fixed 96-bit nonce
  if iv.len() != 12 {
    return Err(type_error("iv length not equal to 12"));
  }

  let nonce = Nonce::from_slice(&iv);

  let mut ciphertext = data.to_vec();
  let tag = match length {
    128 => {
      let cipher = Aes128Gcm::new_from_slice(key)
        .map_err(|_| operation_error("Encryption failed"))?;
      cipher
        .encrypt_in_place_detached(nonce, &additional_data, &mut ciphertext)
        .map_err(|_| operation_error("Encryption failed"))?
    }
    192 => {
      type Aes192Gcm = aes_gcm::AesGcm<Aes192, U12>;

      let cipher = Aes192Gcm::new_from_slice(key)
        .map_err(|_| operation_error("Encryption failed"))?;
      cipher
        .encrypt_in_place_detached(nonce, &additional_data, &mut ciphertext)
        .map_err(|_| operation_error("Encryption failed"))?
    }
    256 => {
      let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| operation_error("Encryption failed"))?;
      cipher
        .encrypt_in_place_detached(nonce, &additional_data, &mut ciphertext)
        .map_err(|_| operation_error("Encryption failed"))?
    }
    _ => return Err(type_error("invalid length")),
  };

  // Truncated tag to the specified tag length.
  // `tag` is fixed to be 16 bytes long and (tag_length / 8) is always <= 16
  let tag = &tag[..(tag_length / 8)];

  // C | T
  ciphertext.extend_from_slice(tag);

  Ok(ciphertext)
}

fn encrypt_aes_ctr_gen<B, F>(
  key: &[u8],
  counter: &[u8],
  data: &[u8],
) -> Result<Vec<u8>, AnyError>
where
  B: BlockEncrypt + NewBlockCipher,
  F: CtrFlavor<B::BlockSize>,
{
  let mut cipher = Ctr::<B, F>::new(key.into(), counter.into());

  let mut ciphertext = data.to_vec();
  cipher
    .try_apply_keystream(&mut ciphertext)
    .map_err(|_| operation_error("tried to encrypt too much data"))?;

  Ok(ciphertext)
}

fn encrypt_aes_ctr(
  key: RawKeyData,
  key_length: usize,
  counter: &[u8],
  ctr_length: usize,
  data: &[u8],
) -> Result<Vec<u8>, AnyError> {
  let key = key.as_secret_key()?;

  match ctr_length {
    32 => match key_length {
      128 => encrypt_aes_ctr_gen::<aes::Aes128, Ctr32BE>(key, counter, data),
      192 => encrypt_aes_ctr_gen::<aes::Aes192, Ctr32BE>(key, counter, data),
      256 => encrypt_aes_ctr_gen::<aes::Aes256, Ctr32BE>(key, counter, data),
      _ => Err(type_error("invalid length")),
    },
    64 => match key_length {
      128 => encrypt_aes_ctr_gen::<aes::Aes128, Ctr64BE>(key, counter, data),
      192 => encrypt_aes_ctr_gen::<aes::Aes192, Ctr64BE>(key, counter, data),
      256 => encrypt_aes_ctr_gen::<aes::Aes256, Ctr64BE>(key, counter, data),
      _ => Err(type_error("invalid length")),
    },
    128 => match key_length {
      128 => encrypt_aes_ctr_gen::<aes::Aes128, Ctr128BE>(key, counter, data),
      192 => encrypt_aes_ctr_gen::<aes::Aes192, Ctr128BE>(key, counter, data),
      256 => encrypt_aes_ctr_gen::<aes::Aes256, Ctr128BE>(key, counter, data),
      _ => Err(type_error("invalid length")),
    },
    _ => Err(type_error(
      "invalid counter length. Currently supported 32/64/128 bits",
    )),
  }
}
