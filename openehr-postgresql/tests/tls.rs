//! Verified PostgreSQL transport (`db:O10.7`, `db:O10.18`).
//!
//! The live cases are provisioned by `scripts/verify-store.sh`: one server
//! presents a certificate for `localhost`, and another offers plaintext only.

use openehr_postgresql::PostgresqlStore;
use openehr_store::StoreError;

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(
        std::env::var(name).unwrap_or_else(|_| panic!("set {name} or run verify-store.sh")),
    )
    .unwrap_or_else(|error| panic!("read {name}: {error}"))
}

fn params(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("set {name} or run verify-store.sh"))
}

fn refused(result: Result<PostgresqlStore, StoreError>, accepted: &str) -> StoreError {
    match result {
        Ok(_) => panic!("{accepted}"),
        Err(error) => error,
    }
}

#[test]
fn an_empty_or_malformed_ca_is_refused_before_a_socket_is_opened() {
    let unreachable = "host=127.0.0.1 port=1 user=none dbname=none";
    for malformed in [b"".as_slice(), b"not a certificate".as_slice()] {
        let error = refused(
            PostgresqlStore::connect_with_ca(unreachable, malformed),
            "an invalid trust store was accepted",
        );
        assert!(error.to_string().contains("TLS trust store"));
        assert!(!error.to_string().contains("127.0.0.1"));
    }
}

#[test]
#[ignore = "needs the TLS-enabled server provisioned by scripts/verify-store.sh"]
fn a_trusted_certificate_for_the_configured_name_connects() {
    let mut store = PostgresqlStore::connect_with_ca(
        &params("OPENEHR_POSTGRESQL_URL"),
        &bytes("OPENEHR_POSTGRESQL_CA"),
    )
    .expect("trusted TLS connection");
    store.client().simple_query("SELECT 1").expect("query");
}

#[test]
#[ignore = "needs the TLS-enabled server provisioned by scripts/verify-store.sh"]
fn a_certificate_for_another_server_name_is_refused() {
    let error = refused(
        PostgresqlStore::connect_with_ca(
            &params("OPENEHR_POSTGRESQL_WRONG_HOST_URL"),
            &bytes("OPENEHR_POSTGRESQL_CA"),
        ),
        "a certificate for localhost was accepted for wrong.example",
    );
    assert!(!error.to_string().contains("openehr_Passw0rd"));
}

#[test]
#[ignore = "needs the TLS-enabled server provisioned by scripts/verify-store.sh"]
fn a_certificate_from_an_unknown_issuer_is_refused() {
    let error = refused(
        PostgresqlStore::connect_with_ca(
            &params("OPENEHR_POSTGRESQL_URL"),
            &bytes("OPENEHR_POSTGRESQL_WRONG_CA"),
        ),
        "an untrusted issuer was accepted",
    );
    assert!(!error.to_string().contains("openehr_Passw0rd"));
}

#[test]
#[ignore = "needs the plaintext server provisioned by scripts/verify-store.sh"]
fn a_plaintext_only_server_is_refused_without_fallback() {
    let error = refused(
        PostgresqlStore::connect_with_ca(
            &params("OPENEHR_POSTGRESQL_PLAINTEXT_URL"),
            &bytes("OPENEHR_POSTGRESQL_CA"),
        ),
        "sslmode=disable overrode the connector's mandatory TLS policy",
    );
    assert!(!error.to_string().contains("openehr_Passw0rd"));
}
