//! A throwaway PASETO v4 keypair and a token signed by it, for testing this
//! service by hand or with `openehr-loco/scripts/conformance.sh`.
//!
//! Not a credential-management tool: the secret key exists only in this
//! process's memory and is discarded when it exits. Anyone running this gets
//! a keypair nobody else has, which is exactly right for a throwaway test
//! token and exactly wrong for anything this crate would actually verify in
//! production — see [`crate::auth`]'s own module documentation for how a real
//! deployment is meant to hold its verification key.
//!
//! ```sh
//! cargo run --example generate_test_token
//! ```

use pasetors::{
    claims::Claims,
    keys::{AsymmetricKeyPair, Generate as _},
    paserk::FormatAsPaserk as _,
    public,
    version4::V4,
};

fn main() {
    let pair = AsymmetricKeyPair::<V4>::generate().expect("keypair");
    let mut paserk = String::new();
    pair.public.fmt(&mut paserk).expect("PASERK");

    let mut claims = Claims::new().expect("claims");
    claims.subject("clinician-4417").expect("subject");
    let token = public::sign(&pair.secret, &claims, None, None).expect("signs");

    println!("OPENEHR_PASETO_PUBLIC_KEYS={paserk}");
    println!("Authorization: Bearer {token}");
}
