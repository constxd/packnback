extern crate rand;
use rand::OsRng;
use rand::RngCore;

#[allow(non_upper_case_globals)]
#[allow(dead_code)]
mod bindings;
use self::bindings::*;

pub const CRYPTO_SIGN_BYTES: usize = crypto_sign_ed25519_BYTES as usize;
pub const CRYPTO_BOX_ZEROBYTES: usize = crypto_box_curve25519xsalsa20poly1305_ZEROBYTES as usize;
pub const CRYPTO_BOX_BOXZEROBYTES: usize =
    crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES as usize;

#[derive(Clone)]
#[derive(Default)]
pub struct CryptoBoxNonce {
    pub bytes: [u8; crypto_box_curve25519xsalsa20poly1305_NONCEBYTES as usize],
}

impl CryptoBoxNonce {
    pub fn new() -> CryptoBoxNonce {
        let mut n: CryptoBoxNonce = Default::default();
        let mut rng = OsRng::new().expect("Error opening random number generator");
        rng.fill_bytes(&mut n.bytes[..]);
        n
    }

    pub fn inc(&mut self) {
        for x in 0..self.bytes.len() {
            let b = self.bytes[x].wrapping_add(1);
            self.bytes[x] = b;
            if b != 0 {
                break;
            }
        }
    }
}

#[derive(Clone)]
#[derive(Default)]
pub struct CryptoBoxPk {
    pub bytes: [u8; crypto_box_curve25519xsalsa20poly1305_PUBLICKEYBYTES as usize],
}

#[derive(Default)]
pub struct CryptoBoxSk {
    pub bytes: [u8; crypto_box_curve25519xsalsa20poly1305_SECRETKEYBYTES as usize],
}

impl Drop for CryptoBoxSk {
    fn drop(&mut self) {
        // XXX This may be optimized away, how to ensure wiping of memory
        // It is not totally critical but nice to have.
        self.bytes = [0; crypto_box_curve25519xsalsa20poly1305_SECRETKEYBYTES as usize];
    }
}

pub fn crypto_box_keypair(pk: &mut CryptoBoxPk, sk: &mut CryptoBoxSk) {
    unsafe {
        assert!(
            0 == crypto_box_curve25519xsalsa20poly1305_tweet_keypair(
                pk.bytes.as_mut_ptr(),
                sk.bytes.as_mut_ptr()
            )
        );
    }
}

pub fn boxed_crypto_box_keypair() -> (Box<CryptoBoxPk>, Box<CryptoBoxSk>) {
    let mut pk = Box::<CryptoBoxPk>::new(Default::default());
    let mut sk = Box::<CryptoBoxSk>::new(Default::default());
    crypto_box_keypair(&mut *pk, &mut *sk);
    (pk, sk)
}

#[derive(Clone)]
#[derive(Default)]
pub struct CryptoSignPk {
    pub bytes: [u8; crypto_sign_ed25519_PUBLICKEYBYTES as usize],
}

pub struct CryptoSignSk {
    pub bytes: [u8; crypto_sign_ed25519_SECRETKEYBYTES as usize],
}

impl Default for CryptoSignSk {
    fn default() -> CryptoSignSk {
        CryptoSignSk {
            bytes: [0; crypto_sign_ed25519_SECRETKEYBYTES as usize],
        }
    }
}

impl Drop for CryptoSignSk {
    fn drop(&mut self) {
        // XXX This may be optimized away, how to ensure wiping of memory
        // It is not totally critical but nice to have.
        self.bytes = [0; crypto_sign_ed25519_SECRETKEYBYTES as usize];
    }
}

pub fn crypto_sign_keypair(pk: &mut CryptoSignPk, sk: &mut CryptoSignSk) {
    unsafe {
        assert!(
            0 == crypto_sign_ed25519_tweet_keypair(pk.bytes.as_mut_ptr(), sk.bytes.as_mut_ptr())
        );
    }
}

pub fn boxed_crypto_sign_keypair() -> (Box<CryptoSignPk>, Box<CryptoSignSk>) {
    let mut pk = Box::<CryptoSignPk>::new(Default::default());
    let mut sk = Box::<CryptoSignSk>::new(Default::default());
    crypto_sign_keypair(&mut *pk, &mut *sk);
    (pk, sk)
}

pub fn crypto_sign(sm: &mut [u8], m: &[u8], sk: &CryptoSignSk) -> usize {
    // Contract from nacl api.
    assert!(sm.len() >= m.len() + crypto_sign_ed25519_BYTES as usize);

    let mut smsz: u64 = 0;

    unsafe {
        assert!(
            0 == crypto_sign_ed25519_tweet(
                sm.as_mut_ptr(),
                &mut smsz,
                m.as_ptr(),
                m.len() as u64,
                sk.bytes.as_ptr()
            )
        );
    }

    smsz as usize
}

pub fn crypto_sign_open(m: &mut [u8], sm: &[u8], pk: &CryptoSignPk) -> Option<usize> {
    assert!(m.len() >= sm.len());

    let mut msz: u64 = 0;

    let rc = unsafe {
        crypto_sign_ed25519_tweet_open(
            m.as_mut_ptr(),
            &mut msz,
            sm.as_ptr(),
            sm.len() as u64,
            pk.bytes.as_ptr(),
        )
    };

    if rc != 0 {
        None
    } else {
        Some(msz as usize)
    }
}

pub fn crypto_box(c: &mut [u8], m: &[u8], n: &CryptoBoxNonce, pk: &CryptoBoxPk, sk: &CryptoBoxSk) {
    // Contract from nacl api.
    assert!(c.len() >= m.len());
    assert!(m.len() >= crypto_box_curve25519xsalsa20poly1305_ZEROBYTES as usize);
    for i in 0..(crypto_box_curve25519xsalsa20poly1305_ZEROBYTES as usize) {
        assert!(m[i] == 0);
    }

    unsafe {
        assert!(
            0 == crypto_box_curve25519xsalsa20poly1305_tweet(
                c.as_mut_ptr(),
                m.as_ptr(),
                m.len() as u64,
                n.bytes.as_ptr(),
                pk.bytes.as_ptr(),
                sk.bytes.as_ptr()
            )
        );
    }
}

pub fn crypto_box_open(
    m: &mut [u8],
    c: &[u8],
    n: &CryptoBoxNonce,
    pk: &CryptoBoxPk,
    sk: &CryptoBoxSk,
) -> bool {
    // Contract from nacl api.
    assert!(m.len() >= c.len());
    assert!(c.len() >= crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES as usize);

    for i in 0..(crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES as usize) {
        m[i] = 0;
    }

    unsafe {
        0 == crypto_box_curve25519xsalsa20poly1305_tweet_open(
            m.as_mut_ptr(),
            c.as_ptr(),
            c.len() as u64,
            n.bytes.as_ptr(),
            pk.bytes.as_ptr(),
            sk.bytes.as_ptr(),
        )
    }
}

// Defined for tweetnacl to call.
#[no_mangle]
pub extern "C" fn randombytes(p: *mut u8, sz: usize) -> usize {
    let mut rng = OsRng::new().expect("Error opening random number generator");
    let buf = unsafe { std::slice::from_raw_parts_mut(p, sz) };
    rng.fill_bytes(buf);
    0
}

// Tests --------------------

#[test]
fn test_crypto_box() {
    const MSIZE: usize = (crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES + 128) as usize;
    let mut m1: [u8; MSIZE] = [3; MSIZE];
    let mut m2: [u8; MSIZE] = [0; MSIZE];
    let mut c: [u8; MSIZE] = [0; MSIZE];

    let (pk, sk) = boxed_crypto_box_keypair();
    let n = CryptoBoxNonce::new();

    for i in 0..crypto_box_curve25519xsalsa20poly1305_ZEROBYTES {
        m1[i as usize] = 0;
    }
    crypto_box(&mut c[..], &m1, &n, &pk, &sk);

    for i in 0..(crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES as usize) {
        assert!(c[i] == 0);
    }

    assert!(crypto_box_open(&mut m2[..], &c, &n, &pk, &sk));
    assert_eq!(
        m1[(crypto_box_curve25519xsalsa20poly1305_ZEROBYTES as usize)..],
        m2[(crypto_box_curve25519xsalsa20poly1305_ZEROBYTES as usize)..]
    )
}

#[test]
fn test_crypto_sign() {
    const MSIZE: usize = 32;
    const SMSIZE: usize = MSIZE + (crypto_sign_ed25519_BYTES as usize);
    let m1: [u8; MSIZE] = [3; MSIZE];
    let mut m2: [u8; SMSIZE] = [0; SMSIZE];
    let mut sm: [u8; SMSIZE] = [0; SMSIZE];
    let (pk, sk) = boxed_crypto_sign_keypair();
    let smsz = crypto_sign(&mut sm[..], &m1, &*sk);
    let m2sz = crypto_sign_open(&mut m2[..], &sm[..smsz], &*pk).unwrap();
    assert_eq!(m1, m2[0..m2sz]);
}

#[test]
fn test_nonce_inc() {
    let mut n = CryptoBoxNonce::new();
    n.bytes[0] = 0xff;
    n.bytes[1] = 0xff;
    n.bytes[2] = 0xfe;
    n.bytes[3] = 3;
    n.inc();
    assert!(n.bytes[0] == 0);
    assert!(n.bytes[1] == 0);
    assert!(n.bytes[2] == 0xff);
    assert!(n.bytes[3] == 3);
}
