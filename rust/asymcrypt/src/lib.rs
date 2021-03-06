extern crate tweetnacl;
use std::error;
use std::fmt;
use tweetnacl::*;

#[derive(Default)]
pub struct Key {
    pub box_sk: CryptoBoxSk,
    pub box_pk: CryptoBoxPk,
    pub sign_sk: CryptoSignSk,
    pub sign_pk: CryptoSignPk,
}

#[derive(Default)]
pub struct PublicKey {
    pub box_pk: CryptoBoxPk,
    pub sign_pk: CryptoSignPk,
}

impl Key {
    pub fn new() -> Box<Key> {
        let mut k = Box::<Key>::new(Default::default());
        crypto_box_keypair(&mut k.box_pk, &mut k.box_sk);
        crypto_sign_keypair(&mut k.sign_pk, &mut k.sign_sk);
        k
    }

    pub fn pub_key(&self) -> PublicKey {
        PublicKey {
            box_pk: self.box_pk.clone(),
            sign_pk: self.sign_pk.clone(),
        }
    }

    pub fn write(&self, w: &mut std::io::Write) -> Result<(), std::io::Error> {
        write_header(w, KEYHEADER)?;
        w.write_all(&self.box_pk.bytes)?;
        w.write_all(&self.box_sk.bytes)?;
        w.write_all(&self.sign_pk.bytes)?;
        w.write_all(&self.sign_sk.bytes)?;
        Ok(())
    }

    pub fn read_boxed_from(r: &mut std::io::Read) -> Result<Box<Key>, AsymcryptError> {
        expect_header(r, KEYHEADER)?;
        let mut k = Box::<Key>::new(Default::default());
        r.read_exact(&mut k.box_pk.bytes)?;
        r.read_exact(&mut k.box_sk.bytes)?;
        r.read_exact(&mut k.sign_pk.bytes)?;
        r.read_exact(&mut k.sign_sk.bytes)?;
        Ok(k)
    }
}

impl PublicKey {
    pub fn write(&self, w: &mut std::io::Write) -> Result<(), std::io::Error> {
        write_header(w, PUBKEYHEADER)?;
        w.write_all(&self.box_pk.bytes)?;
        w.write_all(&self.sign_pk.bytes)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum AsymcryptError {
    InvalidDataError,
    UnsupportedVersionError,
    UnexpectedDataTypeError,
    DecryptKeyMismatchError,
    SignatureKeyMismatchError,
    SignatureFailedError,
    CorruptOrTamperedDataError,
    IOError(std::io::Error),
}

impl fmt::Display for AsymcryptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AsymcryptError::InvalidDataError => {
                write!(f, "The input data is not in the expected format.")
            }
            AsymcryptError::UnsupportedVersionError => {
                write!(f, "Unsupported encrypted/signed data version.")
            }
            AsymcryptError::UnexpectedDataTypeError => {
                write!(f, "The given data is of an unexpected cryptographic type.")
            }
            AsymcryptError::DecryptKeyMismatchError => {
                write!(f, "The given key cannot decrypt the given data.")
            }
            AsymcryptError::SignatureKeyMismatchError => {
                write!(f, "The given key did not create the given signature.")
            }
            AsymcryptError::SignatureFailedError => write!(f, "The digital signature has failed."),
            AsymcryptError::CorruptOrTamperedDataError => {
                write!(f, "Decrypting found corrupt or tampered with data.")
            }
            AsymcryptError::IOError(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for AsymcryptError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            AsymcryptError::IOError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AsymcryptError {
    fn from(err: std::io::Error) -> AsymcryptError {
        AsymcryptError::IOError(err)
    }
}

type AsymcryptHeaderType = u16;

const KEYHEADER: AsymcryptHeaderType = 0;
const PUBKEYHEADER: AsymcryptHeaderType = 1;
const SIGNATUREHEADER: AsymcryptHeaderType = 2;
const CIPHERTEXTHEADER: AsymcryptHeaderType = 3;
const HEADEREND: AsymcryptHeaderType = 4;

fn u16_to_header_type(t: u16) -> Option<AsymcryptHeaderType> {
    if t >= KEYHEADER && t < HEADEREND {
        Some(t as AsymcryptHeaderType)
    } else {
        None
    }
}

fn u16_be_bytes(v: u16) -> (u8, u8) {
    ((((v & 0xff00) >> 8) as u8), (v & 0xff) as u8)
}

fn be_bytes_to_u16(hi: u8, lo: u8) -> u16 {
    ((hi as u16) << 8) | (lo as u16)
}

const MAGIC_LEN: usize = 9;

fn write_header(
    w: &mut std::io::Write,
    val_type: AsymcryptHeaderType,
) -> Result<(), std::io::Error> {
    let magic = "asymcrypt";
    assert!(MAGIC_LEN == magic.len());
    w.write_all(magic.as_bytes())?;
    let (a, b) = u16_be_bytes(2);
    let (c, d) = u16_be_bytes(val_type as u16);
    let ver_and_val = [a, b, c, d];
    w.write_all(&ver_and_val[..])
}

fn read_header(r: &mut std::io::Read) -> Result<AsymcryptHeaderType, AsymcryptError> {
    let magic = "asymcrypt";
    let mut magic_buf: [u8; MAGIC_LEN] = [0; MAGIC_LEN];
    assert!(MAGIC_LEN == magic.len());

    let mut ver_and_val = [0; 4];
    r.read_exact(&mut magic_buf)?;
    if magic.as_bytes() != magic_buf {
        return Err(AsymcryptError::InvalidDataError);
    }
    r.read_exact(&mut ver_and_val)?;

    let ver = be_bytes_to_u16(ver_and_val[0], ver_and_val[1]);
    let val_type = be_bytes_to_u16(ver_and_val[2], ver_and_val[3]);

    match (ver, u16_to_header_type(val_type)) {
        (2, Some(t)) => Ok(t),
        (2, None) => Err(AsymcryptError::InvalidDataError),
        _ => Err(AsymcryptError::UnsupportedVersionError),
    }
}

fn expect_header(
    r: &mut std::io::Read,
    val_type: AsymcryptHeaderType,
) -> Result<(), AsymcryptError> {
    let read_val_type = read_header(r)?;
    if read_val_type == val_type {
        Ok(())
    } else {
        Err(AsymcryptError::UnexpectedDataTypeError)
    }
}

fn read_exact_or_eof(r: &mut std::io::Read, buf: &mut [u8]) -> Result<usize, std::io::Error> {
    let n: usize = 0;
    loop {
        match r.read(buf)? {
            0 => return Ok(n),
            n_read => {
                n += n_read;
                buf = &mut buf[n_read..];
            }
        }
    }
}

fn encrypt(
    in_data: &mut std::io::Read,
    out_data: &mut std::io::Write,
    toKey: &PublicKey,
) -> Result<(), std::io::Error> {
    const READ_SZ: usize = 16384;
    const BUF_SZ: usize = READ_SZ + CRYPTO_BOX_ZEROBYTES + 2;
    let mut plain_text: [u8; BUF_SZ] = [0; BUF_SZ];
    let mut cipher_text: [u8; BUF_SZ] = [0; BUF_SZ];
    let mut nonce = CryptoBoxNonce::new();
    let (ephemeral_pk, ephemeral_sk) = boxed_crypto_box_keypair();

    write_header(out_data, CIPHERTEXTHEADER)?;
    out_data.write_all(&ephemeral_pk.bytes)?;
    // XXX write key id.
    out_data.write_all(&nonce.bytes)?;

    loop {
        match read_exact_or_eof(&mut in_data, &mut plain_text[CRYPTO_BOX_ZEROBYTES + 2..])? {
            0 => {
                break;
            }
            n => {
                assert!(n <= 0xffff);
                let (sz_hi, sz_lo) = u16_be_bytes(n as u16);
                plain_text[CRYPTO_BOX_ZEROBYTES] = sz_hi;
                plain_text[CRYPTO_BOX_ZEROBYTES + 1] = sz_lo;
                crypto_box(
                    &mut cipher_text,
                    &plain_text,
                    &nonce,
                    &toKey.box_pk,
                    &ephemeral_sk,
                );
                out_data.write_all(&mut cipher_text[CRYPTO_BOX_BOXZEROBYTES..])?;
            }
        }

        nonce.inc();
    }

    Ok(())
}
