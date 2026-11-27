//! The `PostgreSQL` store.

use crate::PostgresqlDialect;
use openehr::base::{HierObjectId, ObjectId, ObjectRef, ObjectVersionId};
use openehr::rm::common::{CommitError, Contribution, Version};
use openehr::rm::data_types::DvDateTime;
use openehr::rm::ehr::{Composition, Ehr, EhrStatus};
use openehr::validation::Validate as _;
use openehr_store::record::{CompositionIndexRow, StoredInstant, VersionRow};
use openehr_store::store::{version_at_time_candidates_sql, version_uid_at_time};
use openehr_store::{
    CommitOutcome, Placeholder, Result, Store, StoreError, check_commit_rules, ddl_script,
};
use postgres::error::SqlState;
use postgres::fallible_iterator::FallibleIterator as _;
use postgres::types::ToSql;
use postgres::{Client, GenericClient, Row};
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject as _;
use rustls::{ClientConfig, RootCertStore};
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio_postgres_rustls::MakeRustlsConnect;

/// The engine name used in errors.
const ENGINE: &str = "PostgreSQL";

/// An openEHR repository in a `PostgreSQL` database.
///
/// # Why the client is behind a `RefCell`
///
/// [`Store`]'s read methods (`get_ehr`, `latest_version`, …) take `&self`,
/// the same shape `SqliteStore` has — `rusqlite::Connection` allows a shared
/// query because `SQLite`'s own C API serialises access per connection.
/// `postgres::Client`, the blocking handle this crate uses, has no such
/// interior synchronisation: every one of its methods, reads included,
/// takes `&mut self`, because it is driving a single background
/// `tokio-postgres` task and nothing about that is `Sync`. `RefCell` supplies
/// the interior mutability `Store`'s shared trait signature needs without
/// widening it — a second borrow while one is outstanding panics rather than
/// racing two queries over one connection, which is the behaviour a
/// single-threaded-at-a-time `Store` (this one, used from behind a `Mutex`
/// exactly as `SqliteStore` is in `openehr-axa`) is supposed to have anyway.
pub struct PostgresqlStore {
    client: RefCell<Client>,
    /// Set while [`Store::create_ehr_with_status`] holds a transaction open:
    /// the status commit then writes inside it instead of opening its own,
    /// which on this driver would be a second `BEGIN` and an early `COMMIT`
    /// of the caller's transaction (`db:H5.23`).
    creating: Cell<bool>,
}

impl PostgresqlStore {
    /// Connects to a `PostgreSQL` server.
    ///
    /// `params` is a `postgres` connection string
    /// (`host=... user=... password=... dbname=...`), not a URL — the same
    /// format `openehr-store/scripts/verify-schema.sh` exercises the DDL
    /// against, so a connection that works there works here.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Engine`] if the connection cannot be established.
    ///
    /// TLS is required even when `params` says `sslmode=prefer` or
    /// `sslmode=disable`. The certificate chain is verified against the
    /// platform trust store and the certificate must cover the configured
    /// server name. Negotiation failure never falls back to plaintext
    /// (`db:O10.7`, `db:O10.18`).
    pub fn connect(params: &str) -> Result<Self> {
        let (tls, errors) =
            MakeRustlsConnect::with_native_certs().map_err(|_| tls_roots_error())?;
        if !errors.is_empty() {
            return Err(tls_roots_error());
        }
        Self::connect_with(params, tls)
    }

    /// Connects with one or more PEM-encoded private CA certificates.
    ///
    /// The supplied bundle replaces the platform roots. TLS, chain validation,
    /// and server-name validation remain mandatory, so this is suitable for a
    /// private CA without creating an accept-invalid-certificate mode.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Engine`] if the bundle is empty or malformed, or
    /// if a verified TLS connection cannot be established.
    pub fn connect_with_ca(params: &str, ca_pem: &[u8]) -> Result<Self> {
        let certificates = CertificateDer::pem_slice_iter(ca_pem)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| tls_roots_error())?;
        if certificates.is_empty() {
            return Err(tls_roots_error());
        }
        let mut roots = RootCertStore::empty();
        for certificate in certificates {
            roots.add(certificate).map_err(|_| tls_roots_error())?;
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| tls_configuration_error())?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Self::connect_with(params, MakeRustlsConnect::new(config))
    }

    fn connect_with(params: &str, tls: MakeRustlsConnect) -> Result<Self> {
        let mut config: postgres::Config = params.parse().map_err(|e| engine(&e))?;
        config.ssl_mode(postgres::config::SslMode::Require);
        Ok(Self {
            client: RefCell::new(config.connect(tls).map_err(|e| engine(&e))?),
            creating: Cell::new(false),
        })
    }

    /// The underlying client, for callers that need a query this trait does
    /// not offer.
    ///
    /// `&mut self`, not the `RefCell` borrow every trait method uses
    /// internally: a caller holding `&mut PostgresqlStore` already has
    /// exclusive access, so this reaches straight through with
    /// [`RefCell::get_mut`] rather than adding a runtime borrow check that
    /// cannot fail here.
    #[must_use]
    pub fn client(&mut self) -> &mut Client {
        self.client.get_mut()
    }

    /// Reads a 32-byte digest column.
    ///
    /// A wrong length is a conversion failure rather than a silent
    /// truncation: a digest that is not 32 bytes did not come from SHA-256,
    /// and padding or clipping it would produce a value that compares
    /// cleanly against nothing.
    fn digest_column(row: &Row, name: &str) -> Result<[u8; 32]> {
        let raw: Vec<u8> = column(row, name)?;
        <[u8; 32]>::try_from(raw.as_slice()).map_err(|_| StoreError::Engine {
            engine: ENGINE,
            message: format!("{name} is not 32 bytes"),
        })
    }

    /// Reads an optional 32-byte digest column.
    fn optional_digest_column(row: &Row, name: &str) -> Result<Option<[u8; 32]>> {
        let raw: Option<Vec<u8>> = column(row, name)?;
        raw.map(|v| {
            <[u8; 32]>::try_from(v.as_slice()).map_err(|_| StoreError::Engine {
                engine: ENGINE,
                message: format!("{name} is not 32 bytes"),
            })
        })
        .transpose()
    }

    /// `utc_seconds` as `Option<i64>`, stored as `timestamptz`
    /// (`PostgresqlDialect::col_sql`).
    ///
    /// `postgres-types` has no `ToSql`/`FromSql` between a raw integer and
    /// `timestamptz` — the wire protocol represents the two differently, and
    /// nothing coerces between them the way `SQLite`'s loose typing does. This
    /// is the one column where this store must convert rather than bind or
    /// read a stored value directly: seconds-since-epoch through
    /// [`OffsetDateTime::from_unix_timestamp`] on the way in, and
    /// [`OffsetDateTime::unix_timestamp`] on the way out. The conversion is
    /// exact — both are integer seconds, and neither direction can lose or
    /// invent a fraction — so the two stored columns (`db:M3.28`'s own
    /// "authoritative text plus derived UTC" pair) still agree with each
    /// other after the round trip.
    fn utc_seconds_to_timestamptz(utc_seconds: Option<i64>) -> Result<Option<OffsetDateTime>> {
        utc_seconds
            .map(|secs| {
                OffsetDateTime::from_unix_timestamp(secs).map_err(|_| StoreError::Engine {
                    engine: ENGINE,
                    message: format!("{secs} is not a representable instant"),
                })
            })
            .transpose()
    }

    /// The inverse of [`Self::utc_seconds_to_timestamptz`].
    fn timestamptz_to_utc_seconds(at: Option<OffsetDateTime>) -> Option<i64> {
        at.map(OffsetDateTime::unix_timestamp)
    }

    /// Reads a version row from a query row.
    /// The composition-index columns, in the order every index read names them.
    const INDEX_COLUMNS: &'static str = "version_uid, ehr_id, archetype_id, template_id, \
        category_code, composer_name, language_code, territory_code, setting_code, \
        context_start_text, context_start_utc, context_end_text, context_end_utc";

    /// One composition-index row, as both the search and its paged form read
    /// it.
    fn read_index_row(row: &Row) -> Result<CompositionIndexRow> {
        let context_start_utc: Option<OffsetDateTime> = column(row, "context_start_utc")?;
        let context_end_utc: Option<OffsetDateTime> = column(row, "context_end_utc")?;
        let instant = |text: Option<String>, utc: Option<OffsetDateTime>| {
            text.map(|text| StoredInstant {
                text,
                utc_seconds: Self::timestamptz_to_utc_seconds(utc),
            })
        };
        Ok(CompositionIndexRow {
            version_uid: column(row, "version_uid")?,
            ehr_id: column(row, "ehr_id")?,
            archetype_id: column(row, "archetype_id")?,
            template_id: column(row, "template_id")?,
            category_code: column(row, "category_code")?,
            composer_name: column(row, "composer_name")?,
            language_code: column(row, "language_code")?,
            territory_code: column(row, "territory_code")?,
            setting_code: column(row, "setting_code")?,
            context_start: instant(column(row, "context_start_text")?, context_start_utc),
            context_end: instant(column(row, "context_end_text")?, context_end_utc),
        })
    }

    /// A read-only `REPEATABLE READ` transaction: every statement in it reads
    /// one snapshot, so a count and the page it describes agree (`db:P6.25`).
    fn snapshot(client: &mut Client) -> Result<postgres::Transaction<'_>> {
        client
            .build_transaction()
            .isolation_level(postgres::IsolationLevel::RepeatableRead)
            .read_only(true)
            .start()
            .map_err(|e| engine(&e))
    }

    fn read_version(row: &Row) -> Result<VersionRow> {
        let audit_time_committed_utc: Option<OffsetDateTime> =
            column(row, "audit_time_committed_utc")?;
        Ok(VersionRow {
            uid: column(row, "uid")?,
            versioned_object_uid: column(row, "versioned_object_uid")?,
            creating_system_id: column(row, "creating_system_id")?,
            trunk_version: column(row, "trunk_version")?,
            branch_number: match column::<i64>(row, "branch_number")? {
                0 => None,
                value => Some(value),
            },
            branch_version: match column::<i64>(row, "branch_version")? {
                0 => None,
                value => Some(value),
            },
            preceding_version_uid: column(row, "preceding_version_uid")?,
            lifecycle_state_code: column(row, "lifecycle_state_code")?,
            is_deleted: column(row, "is_deleted")?,
            contribution_uid: column(row, "contribution_uid")?,
            audit_system_id: column(row, "audit_system_id")?,
            audit_change_type_code: column(row, "audit_change_type_code")?,
            audit_committer_name: column(row, "audit_committer_name")?,
            audit_committer_json: column(row, "audit_committer_json")?,
            audit_time_committed: StoredInstant {
                text: column(row, "audit_time_committed_text")?,
                utc_seconds: Self::timestamptz_to_utc_seconds(audit_time_committed_utc),
            },
            data_json: column(row, "data_json")?,
            audit_description: column(row, "audit_description")?,
            signature: column(row, "signature")?,
            attestations_json: column(row, "attestations_json")?,
            other_input_version_uids_json: column(row, "other_input_version_uids_json")?,
            chain: openehr_store::record::ChainColumns {
                previous: Self::digest_column(row, "chain_previous")?,
                content: Self::digest_column(row, "chain_content")?,
                digest: Self::digest_column(row, "chain_digest")?,
                tag_key_id: column(row, "chain_tag_key_id")?,
                tag_mac: Self::optional_digest_column(row, "chain_tag_mac")?,
            },
        })
    }

    /// Every column of `openehr_version`, in one place so the two read paths
    /// cannot select different sets.
    const VERSION_COLUMNS: &'static str = "uid, versioned_object_uid, creating_system_id, \
        trunk_version, branch_number, branch_version, preceding_version_uid, \
        lifecycle_state_code, is_deleted, contribution_uid, audit_system_id, \
        audit_change_type_code, audit_committer_name, audit_committer_json, \
        audit_time_committed_text, audit_time_committed_utc, data_json, audit_description, signature, \
        attestations_json, other_input_version_uids_json, chain_previous, \
        chain_content, chain_digest, chain_tag_key_id, chain_tag_mac";

    /// Refuses a database installed under a different schema version.
    ///
    /// Three states, and the third is the one that matters — see
    /// `SqliteStore::check_schema_version`, which this mirrors exactly.
    fn check_schema_version(&self) -> Result<()> {
        // Only "the table does not exist" (SQLSTATE 42P01) means "nothing
        // recorded": on a genuinely fresh database the table is not there yet.
        // Any other error propagates (RCA-IN-6).
        let recorded: Option<i64> = self
            .client
            .borrow_mut()
            .query_opt("SELECT version FROM openehr_schema_version LIMIT 1", &[])
            .or_else(|e| absent_if_undefined_table(&e))?
            .map(|row| row.get(0));

        if let Some(found) = recorded {
            if found != openehr_store::SCHEMA_VERSION {
                return Err(StoreError::SchemaVersionMismatch {
                    found,
                    expected: openehr_store::SCHEMA_VERSION,
                });
            }
            return Ok(());
        }

        // No version recorded. Either fresh, or older than versioning itself
        // — told apart, as `SqliteStore` does, by whether `openehr_ehr` holds
        // anything. A missing table (42P01) is "fresh"; other errors propagate.
        let legacy: Option<i64> = self
            .client
            .borrow_mut()
            .query_opt("SELECT count(*) FROM openehr_ehr", &[])
            .or_else(|e| absent_if_undefined_table(&e))?
            .map(|row| row.get(0));
        if legacy.is_some_and(|n| n > 0) {
            return Err(StoreError::SchemaVersionMismatch {
                found: 0,
                expected: openehr_store::SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    /// Records the schema version, once.
    fn record_schema_version(&self) -> Result<()> {
        let now = StoredInstant::from_date_time(&"1970-01-01T00:00:00Z".parse().expect("literal"));
        let utc = Self::utc_seconds_to_timestamptz(now.utc_seconds)?;
        self.client
            .borrow_mut()
            .execute(
                "INSERT INTO openehr_schema_version (version, applied_text, applied_utc) \
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                &[&openehr_store::SCHEMA_VERSION, &now.text, &utc],
            )
            .map(|_| ())
            .map_err(|e| engine(&e))
    }

    /// The chain digest of one version, for linking the next.
    fn chain_digest_of(&self, uid: &str) -> Result<[u8; 32]> {
        let row = self
            .client
            .borrow_mut()
            .query_one(
                "SELECT chain_digest FROM openehr_version WHERE uid = $1",
                &[&uid],
            )
            .map_err(|e| engine(&e))?;
        Self::digest_column(&row, "chain_digest")
    }
}

fn tls_roots_error() -> StoreError {
    StoreError::Engine {
        engine: ENGINE,
        message: "could not load a complete TLS trust store".to_owned(),
    }
}

fn tls_configuration_error() -> StoreError {
    StoreError::Engine {
        engine: ENGINE,
        message: "could not configure the required TLS protocols".to_owned(),
    }
}

/// Translates a uniqueness violation on the version table into the commit
/// refusal it actually is.
///
/// Mirrors `SqliteStore::commit_conflict` for the same reason (`db:H5.9`
/// requires refusals to be distinguishable), against `PostgreSQL`'s own,
/// more structured error: a constraint **name** (`DbError::constraint`)
/// rather than a message to search. `PostgreSQL` names an unnamed
/// `PRIMARY KEY (uid)` clause `<table>_pkey` by its own convention, so
/// `openehr_version_pkey` is a duplicate version identity;
/// `ix_version_container_trunk` is the named index `db:H5.10` requires, and
/// firing means a *different* identity took that position in the tree —
/// a concurrent modification, not a duplicate.
fn commit_conflict(error: &postgres::Error) -> Option<StoreError> {
    let db_error = error.as_db_error()?;
    if *db_error.code() != SqlState::UNIQUE_VIOLATION {
        return None;
    }
    match db_error.constraint() {
        Some("openehr_version_pkey") => Some(StoreError::Commit(CommitError::DuplicateVersion)),
        Some("ix_version_container_trunk") => Some(StoreError::Commit(CommitError::NotLatest)),
        _ => None,
    }
}

/// Reads one column, propagating a wire or type-mismatch error rather than
/// panicking the way [`Row::get`] does.
///
/// A genuine generic **function**, not a closure bound at its own call site:
/// a closure infers one concrete signature from its first use, and every
/// read path here reads several different column types from the same row.
fn column<'a, T: postgres::types::FromSql<'a>>(row: &'a Row, name: &str) -> Result<T> {
    row.try_get(name).map_err(|e| engine(&e))
}

/// Lower-case hex for a digest, matching `Digest256`'s own rendering.
fn hex32(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Maps `undefined_table` (SQLSTATE 42P01) to "absent" (`Ok(None)`) and
/// propagates every other error (RCA-IN-6).
fn absent_if_undefined_table<T>(error: &postgres::Error) -> Result<Option<T>> {
    if error.code() == Some(&postgres::error::SqlState::UNDEFINED_TABLE) {
        Ok(None)
    } else {
        Err(engine(error))
    }
}

/// Wraps a driver error without letting row data into the message.
fn engine(error: &postgres::Error) -> StoreError {
    if let Some(conflict) = commit_conflict(error) {
        return conflict;
    }
    StoreError::Engine {
        engine: ENGINE,
        // `to_string` on a `postgres::Error` gives the server's own message,
        // which names constraints and columns and not values — the same
        // property `SqliteStore::engine` relies on for `SQLite`.
        message: error.to_string(),
    }
}

impl Store for PostgresqlStore {
    fn engine(&self) -> &'static str {
        ENGINE
    }

    fn install(&mut self) -> Result<()> {
        // Check *before* creating anything, for the reason `SqliteStore`
        // states: running the DDL first would create the version table on an
        // old database and make the mismatch look like a fresh install.
        self.check_schema_version()?;
        self.client
            .borrow_mut()
            .batch_execute(&ddl_script(&PostgresqlDialect))
            .map_err(|e| engine(&e))?;
        self.record_schema_version()
    }

    fn create_ehr(&mut self, ehr: &Ehr) -> Result<()> {
        // `lib:A-23`, as `SqliteStore`: `Ehr::new` checks two of these six
        // reference types and deserialization checks none.
        ehr.validate_ok()?;
        let id = ehr.ehr_id().to_string();
        let existing = self
            .client
            .borrow_mut()
            .query_opt("SELECT ehr_id FROM openehr_ehr WHERE ehr_id = $1", &[&id])
            .map_err(|e| engine(&e))?;
        if existing.is_some() {
            return Err(StoreError::Conflict { kind: "ehr", id });
        }
        let created = StoredInstant::from_date_time(ehr.time_created().value());
        let created_utc = Self::utc_seconds_to_timestamptz(created.utc_seconds)?;
        self.client
            .borrow_mut()
            .execute(
                "INSERT INTO openehr_ehr \
                 (ehr_id, system_id, time_created_text, time_created_utc, ehr_status_uid, \
                  ehr_access_uid) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    &id,
                    &ehr.system_id().to_string(),
                    &created.text,
                    &created_utc,
                    &ehr.ehr_status().id().to_string(),
                    &ehr.ehr_access().id().to_string(),
                ],
            )
            .map_err(|e| engine(&e))?;
        Ok(())
    }

    fn get_ehr(&self, ehr_id: &HierObjectId) -> Result<Ehr> {
        let id = ehr_id.to_string();
        let row = self
            .client
            .borrow_mut()
            .query_opt(
                "SELECT system_id, time_created_text, ehr_status_uid, ehr_access_uid \
                 FROM openehr_ehr WHERE ehr_id = $1",
                &[&id],
            )
            .map_err(|e| engine(&e))?;
        let Some(row) = row else {
            return Err(StoreError::NotFound { kind: "ehr", id });
        };
        let system_id: String = row.get(0);
        let created: String = row.get(1);
        let status_uid: String = row.get(2);
        let access_uid: String = row.get(3);
        let reference = |uid: &str, ty: &'static str| -> Result<ObjectRef> {
            Ok(ObjectRef::new(
                "local",
                ty,
                ObjectId::HierObjectId(uid.parse()?),
            )?)
        };
        Ok(Ehr::new(
            system_id.parse()?,
            ehr_id.clone(),
            reference(&status_uid, "VERSIONED_EHR_STATUS")?,
            reference(&access_uid, "VERSIONED_EHR_ACCESS")?,
            DvDateTime::new(&created)?,
        )?)
    }

    fn create_contribution(
        &mut self,
        ehr_id: &HierObjectId,
        contribution: &Contribution,
    ) -> Result<()> {
        // The same projection `SqliteStore` uses (`db:PR12.25`, `db:H5.22`).
        let row = openehr_store::ContributionRow::project(&ehr_id.to_string(), contribution)?;
        let uid = row.uid.clone();
        let committed_utc = Self::utc_seconds_to_timestamptz(row.audit_time_committed.utc_seconds)?;
        self.client
            .borrow_mut()
            .execute(
                "INSERT INTO openehr_contribution \
                 (uid, ehr_id, audit_change_type_code, audit_system_id, audit_committer_name, \
                  audit_committer_json, audit_time_committed_text, audit_time_committed_utc, \
                  versions_json) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &row.uid,
                    &row.ehr_id,
                    &row.audit_change_type_code,
                    &row.audit_system_id,
                    &row.audit_committer_name,
                    &row.audit_committer_json,
                    &row.audit_time_committed.text,
                    &committed_utc,
                    &row.versions_json,
                ],
            )
            .map_err(|e| {
                if e.as_db_error()
                    .is_some_and(|d| *d.code() == SqlState::UNIQUE_VIOLATION)
                {
                    StoreError::Conflict {
                        kind: "contribution",
                        id: uid.clone(),
                    }
                } else {
                    engine(&e)
                }
            })?;
        Ok(())
    }

    // Long for the reason `SqliteStore::commit_composition` states: it is the
    // whole commit path, and the order is the safety property.
    #[allow(clippy::too_many_lines)]
    fn commit_composition(
        &mut self,
        ehr_id: &HierObjectId,
        version: &Version<Composition>,
        contribution_uid: &str,
    ) -> Result<CommitOutcome> {
        version.validate_ok()?;
        let ehr = self.get_ehr(ehr_id)?;

        // `db:H5.17`, read fresh every call — see `SqliteStore`'s own gate.
        // `db:H5.20`: one gate for every engine, scoped to the owner.
        if !openehr_store::ehr_is_modifiable(self, &ehr)? {
            return Err(StoreError::NotModifiable {
                ehr_id: ehr_id.to_string(),
            });
        }

        let container_uid = version.uid().object_id().to_string();
        let head: Option<(String, i64)> = self
            .client
            .borrow_mut()
            .query_opt(
                "SELECT uid, trunk_version FROM openehr_version \
                 WHERE versioned_object_uid = $1 \
                 ORDER BY trunk_version DESC, branch_number DESC, branch_version DESC LIMIT 1",
                &[&container_uid],
            )
            .map_err(|e| engine(&e))?
            .map(|row| (row.get(0), row.get(1)));

        // `db:H5.19`: the `EHR` named here must own the container this version
        // joins; refused as an absent container is, before anything is written
        // (`db:D-19`). Same rule, same placement, as `SqliteStore`.
        if head.is_some() {
            let container = HierObjectId::new(version.uid().object_id().clone(), None)?;
            openehr_store::require_owned(
                self,
                ehr_id,
                &container,
                "COMPOSITION",
                "versioned_object",
                &container_uid,
            )?;
        }

        let uid = version.uid().to_string();
        let already = self
            .client
            .borrow_mut()
            .query_opt("SELECT uid FROM openehr_version WHERE uid = $1", &[&uid])
            .map_err(|e| engine(&e))?;
        check_commit_rules(
            head.as_ref(),
            already.is_some(),
            version.preceding_version_uid(),
        )?;
        // `db:H5.22`, as `SqliteStore` checks it: after the commit rules.
        let declared = self
            .client
            .borrow_mut()
            .query_opt(
                "SELECT ehr_id, versions_json FROM openehr_contribution WHERE uid = $1",
                &[&contribution_uid],
            )
            .map_err(|e| engine(&e))?
            .map(|row| -> Result<(String, String)> {
                Ok((column(&row, "ehr_id")?, column(&row, "versions_json")?))
            })
            .transpose()?;
        openehr_store::check_membership(declared, &ehr_id.to_string(), contribution_uid, &uid)?;

        let previous_digest = head
            .as_ref()
            .map(|(uid, _)| self.chain_digest_of(uid))
            .transpose()?;
        let row = VersionRow::project(version, contribution_uid, previous_digest, None)?;
        let created_container = head.is_none();

        let mut client = self.client.borrow_mut();
        let mut transaction = client.transaction().map_err(|e| engine(&e))?;

        if created_container {
            let created =
                StoredInstant::from_date_time(version.commit_audit().time_committed().value());
            let created_utc = Self::utc_seconds_to_timestamptz(created.utc_seconds)?;
            transaction
                .execute(
                    "INSERT INTO openehr_versioned_object \
                     (uid, ehr_id, rm_type, time_created_text, time_created_utc) \
                     VALUES ($1, $2, $3, $4, $5)",
                    &[
                        &container_uid,
                        &ehr_id.to_string(),
                        &"COMPOSITION",
                        &created.text,
                        &created_utc,
                    ],
                )
                .map_err(|e| engine(&e))?;
        }

        insert_version(&mut transaction, &row)?;

        if let Some(composition) = version.data() {
            let index = CompositionIndexRow::project(&row.uid, &ehr_id.to_string(), composition)?;
            insert_composition_index(&mut transaction, &index)?;
        }

        transaction.commit().map_err(|e| engine(&e))?;
        Ok(CommitOutcome {
            version_uid: version.uid().clone(),
            created_container,
        })
    }

    // Mirrors `commit_composition` deliberately rather than sharing its
    // transaction — see `SqliteStore::commit_ehr_status`'s own doc for why:
    // the logic (`check_commit_rules`, `VersionRow::project`) is already
    // shared, and only the SQL that drives it repeats.
    #[allow(clippy::too_many_lines)]
    fn commit_ehr_status(
        &mut self,
        ehr_id: &HierObjectId,
        version: &Version<EhrStatus>,
        contribution_uid: &str,
    ) -> Result<CommitOutcome> {
        version.validate_ok()?;
        self.get_ehr(ehr_id)?;

        let container_uid = version.uid().object_id().to_string();
        let head: Option<(String, i64)> = self
            .client
            .borrow_mut()
            .query_opt(
                "SELECT uid, trunk_version FROM openehr_version \
                 WHERE versioned_object_uid = $1 \
                 ORDER BY trunk_version DESC, branch_number DESC, branch_version DESC LIMIT 1",
                &[&container_uid],
            )
            .map_err(|e| engine(&e))?
            .map(|row| (row.get(0), row.get(1)));

        // `db:H5.19`: the `EHR` named here must own the container this version
        // joins; refused as an absent container is, before anything is written
        // (`db:D-19`). Same rule, same placement, as `SqliteStore`.
        if head.is_some() {
            let container = HierObjectId::new(version.uid().object_id().clone(), None)?;
            openehr_store::require_owned(
                self,
                ehr_id,
                &container,
                "EHR_STATUS",
                "versioned_object",
                &container_uid,
            )?;
        }

        let uid = version.uid().to_string();
        let already = self
            .client
            .borrow_mut()
            .query_opt("SELECT uid FROM openehr_version WHERE uid = $1", &[&uid])
            .map_err(|e| engine(&e))?;
        check_commit_rules(
            head.as_ref(),
            already.is_some(),
            version.preceding_version_uid(),
        )?;
        // `db:H5.22`, as `SqliteStore` checks it: after the commit rules.
        let declared = self
            .client
            .borrow_mut()
            .query_opt(
                "SELECT ehr_id, versions_json FROM openehr_contribution WHERE uid = $1",
                &[&contribution_uid],
            )
            .map_err(|e| engine(&e))?
            .map(|row| -> Result<(String, String)> {
                Ok((column(&row, "ehr_id")?, column(&row, "versions_json")?))
            })
            .transpose()?;
        openehr_store::check_membership(declared, &ehr_id.to_string(), contribution_uid, &uid)?;

        let previous_digest = head
            .as_ref()
            .map(|(uid, _)| self.chain_digest_of(uid))
            .transpose()?;
        let row = VersionRow::project(version, contribution_uid, previous_digest, None)?;
        let created_container = head.is_none();

        let container_created = created_container.then(|| {
            StoredInstant::from_date_time(version.commit_audit().time_committed().value())
        });
        let mut client = self.client.borrow_mut();
        if self.creating.get() {
            // Inside `create_ehr_with_status`'s transaction (`db:H5.23`).
            write_status(
                &mut *client,
                ehr_id,
                &container_uid,
                container_created,
                &row,
            )?;
        } else {
            let mut transaction = client.transaction().map_err(|e| engine(&e))?;
            write_status(
                &mut transaction,
                ehr_id,
                &container_uid,
                container_created,
                &row,
            )?;
            transaction.commit().map_err(|e| engine(&e))?;
        }
        Ok(CommitOutcome {
            version_uid: version.uid().clone(),
            created_container,
        })
    }

    // `db:H5.23`: one transaction around the three steps. `create_ehr` and
    // `create_contribution` write on the client directly, so they join it;
    // the status commit is told to (`creating`). Any error rolls all back.
    fn create_ehr_with_status(
        &mut self,
        ehr: &Ehr,
        contribution: &Contribution,
        status: &Version<EhrStatus>,
    ) -> Result<CommitOutcome> {
        self.client
            .borrow_mut()
            .batch_execute("BEGIN")
            .map_err(|e| engine(&e))?;
        self.creating.set(true);
        let ehr_id = ehr.ehr_id().clone();
        let outcome = self
            .create_ehr(ehr)
            .and_then(|()| self.create_contribution(&ehr_id, contribution))
            .and_then(|()| {
                self.commit_ehr_status(&ehr_id, status, &contribution.uid().to_string())
            });
        self.creating.set(false);
        let end = if outcome.is_ok() {
            "COMMIT"
        } else {
            "ROLLBACK"
        };
        let ended = self
            .client
            .borrow_mut()
            .batch_execute(end)
            .map_err(|e| engine(&e));
        // The step's own refusal outranks a failure to roll it back.
        let outcome = outcome?;
        ended?;
        Ok(outcome)
    }

    fn get_version(&self, uid: &ObjectVersionId) -> Result<VersionRow> {
        let id = uid.to_string();
        let row = self
            .client
            .borrow_mut()
            .query_opt(
                &format!(
                    "SELECT {} FROM openehr_version WHERE uid = $1",
                    Self::VERSION_COLUMNS
                ),
                &[&id],
            )
            .map_err(|e| engine(&e))?;
        row.ok_or(StoreError::NotFound {
            kind: "version",
            id,
        })
        .and_then(|r| Self::read_version(&r))
    }

    fn latest_version(&self, versioned_object_uid: &HierObjectId) -> Result<VersionRow> {
        let id = versioned_object_uid.to_string();
        let row = self
            .client
            .borrow_mut()
            .query_opt(
                &format!(
                    "SELECT {} FROM openehr_version WHERE versioned_object_uid = $1 \
                     ORDER BY trunk_version DESC, branch_number DESC, branch_version DESC LIMIT 1",
                    Self::VERSION_COLUMNS
                ),
                &[&id],
            )
            .map_err(|e| engine(&e))?;
        row.ok_or(StoreError::NotFound {
            kind: "versioned_object",
            id,
        })
        .and_then(|r| Self::read_version(&r))
    }

    fn versioned_object_owner(
        &self,
        versioned_object_uid: &HierObjectId,
    ) -> Result<(HierObjectId, String)> {
        let id = versioned_object_uid.to_string();
        let row = self
            .client
            .borrow_mut()
            .query_opt(
                "SELECT ehr_id, rm_type FROM openehr_versioned_object WHERE uid = $1",
                &[&id],
            )
            .map_err(|e| engine(&e))?
            .ok_or(StoreError::NotFound {
                kind: "versioned_object",
                id,
            })?;
        let ehr: String = row.get(0);
        Ok((ehr.parse()?, row.get(1)))
    }

    fn version_at_time(
        &self,
        versioned_object_uid: &HierObjectId,
        at: &DvDateTime,
    ) -> Result<VersionRow> {
        let id = versioned_object_uid.to_string();
        let Some(at_seconds) = StoredInstant::from_date_time(at.value()).utc_seconds else {
            // Not established — see `SqliteStore::version_at_time`'s own
            // doc: answering with *some* version would be a guess about the
            // zone (`V8.6`).
            return Err(StoreError::NotFound {
                kind: "version",
                id,
            });
        };
        let at_ts = Self::utc_seconds_to_timestamptz(Some(at_seconds))?;
        let sql = version_at_time_candidates_sql(Placeholder::Dollar);
        let uid = {
            let mut client = self.client.borrow_mut();
            let params: [&(dyn ToSql + Sync); 4] = [&id, &at_ts, &id, &at_ts];
            let candidates = client.query_raw(&sql, params).map_err(|e| engine(&e))?;
            version_uid_at_time(
                at,
                candidates.iterator().map(|row| {
                    let row = row.map_err(|e| engine(&e))?;
                    Ok((
                        column(&row, "uid")?,
                        column(&row, "audit_time_committed_text")?,
                        column(&row, "trunk_version")?,
                    ))
                }),
            )?
        };
        self.get_version(&uid.ok_or(StoreError::NotFound {
            kind: "version",
            id,
        })?)
    }

    fn all_versions(&self, versioned_object_uid: &HierObjectId) -> Result<Vec<VersionRow>> {
        let rows = self
            .client
            .borrow_mut()
            .query(
                &format!(
                    "SELECT {} FROM openehr_version WHERE versioned_object_uid = $1 \
                     ORDER BY trunk_version ASC, branch_number ASC, branch_version ASC",
                    Self::VERSION_COLUMNS
                ),
                &[&versioned_object_uid.to_string()],
            )
            .map_err(|e| engine(&e))?;
        rows.iter().map(Self::read_version).collect()
    }

    fn chain_checkpoint(&self, versioned_object_uid: &HierObjectId) -> Result<String> {
        // Computed and formatted exactly as `SqliteStore::chain_checkpoint`
        // does, so a checkpoint taken from either engine is the same string
        // for the same history.
        let versions = self.all_versions(versioned_object_uid)?;
        let head = versions
            .last()
            .map_or_else(|| "0".repeat(64), |v| hex32(&v.chain.digest));
        Ok(format!(
            "entries={} head={} last_version={}",
            versions.len(),
            head,
            versions.last().map_or("-", |v| v.uid.as_str())
        ))
    }

    fn find_compositions_by_archetype(
        &self,
        ehr_id: &HierObjectId,
        archetype_id: &str,
    ) -> Result<Vec<CompositionIndexRow>> {
        let rows = self
            .client
            .borrow_mut()
            .query(
                &format!(
                    "SELECT {} FROM openehr_composition_index \
                     WHERE ehr_id = $1 AND archetype_id = $2 \
                     ORDER BY version_uid",
                    Self::INDEX_COLUMNS
                ),
                &[&ehr_id.to_string(), &archetype_id],
            )
            .map_err(|e| engine(&e))?;
        rows.iter().map(Self::read_index_row).collect()
    }

    // `db:P6.25`: the page is cut by the engine. Unlike SQLite's one shared
    // connection, other clients can commit between two statements here, so
    // the count and the page are read in one read-only `REPEATABLE READ`
    // transaction: both see the same snapshot, and `total` describes the
    // result the page was cut from.
    fn versions_page_in(
        &self,
        ehr: &HierObjectId,
        versioned_object_uid: &HierObjectId,
        rm_type: &str,
        offset: usize,
        count: usize,
    ) -> Result<openehr_store::RowPage<VersionRow>> {
        openehr_store::require_owned(
            self,
            ehr,
            versioned_object_uid,
            rm_type,
            "versioned_object",
            &versioned_object_uid.to_string(),
        )?;
        let id = versioned_object_uid.to_string();
        let mut client = self.client.borrow_mut();
        let mut snapshot = Self::snapshot(&mut client)?;
        let total: i64 = snapshot
            .query_one(
                "SELECT count(*) FROM openehr_version WHERE versioned_object_uid = $1",
                &[&id],
            )
            .and_then(|row| row.try_get(0))
            .map_err(|e| engine(&e))?;
        let rows = snapshot
            .query(
                &format!(
                    "SELECT {} FROM openehr_version WHERE versioned_object_uid = $1 \
                     ORDER BY trunk_version ASC, branch_number ASC, branch_version ASC \
                     LIMIT $2 OFFSET $3",
                    Self::VERSION_COLUMNS
                ),
                &[&id, &sql_count(count), &sql_count(offset)],
            )
            .map_err(|e| engine(&e))?;
        snapshot.commit().map_err(|e| engine(&e))?;
        Ok(openehr_store::RowPage {
            rows: rows.iter().map(Self::read_version).collect::<Result<_>>()?,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }

    fn find_compositions_by_archetype_page(
        &self,
        ehr_id: &HierObjectId,
        archetype_id: &str,
        offset: usize,
        count: usize,
    ) -> Result<openehr_store::RowPage<CompositionIndexRow>> {
        let ehr = ehr_id.to_string();
        let mut client = self.client.borrow_mut();
        let mut snapshot = Self::snapshot(&mut client)?;
        let total: i64 = snapshot
            .query_one(
                "SELECT count(*) FROM openehr_composition_index \
                 WHERE ehr_id = $1 AND archetype_id = $2",
                &[&ehr, &archetype_id],
            )
            .and_then(|row| row.try_get(0))
            .map_err(|e| engine(&e))?;
        let rows = snapshot
            .query(
                &format!(
                    "SELECT {} FROM openehr_composition_index \
                     WHERE ehr_id = $1 AND archetype_id = $2 \
                     ORDER BY version_uid LIMIT $3 OFFSET $4",
                    Self::INDEX_COLUMNS
                ),
                &[&ehr, &archetype_id, &sql_count(count), &sql_count(offset)],
            )
            .map_err(|e| engine(&e))?;
        snapshot.commit().map_err(|e| engine(&e))?;
        Ok(openehr_store::RowPage {
            rows: rows
                .iter()
                .map(Self::read_index_row)
                .collect::<Result<_>>()?,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }
}

/// A `LIMIT` or `OFFSET` for PostgreSQL, which takes a `bigint`. A value
/// past `i64::MAX` saturates: as a limit it means "all of them", and as an
/// offset "past the end". Both are what the caller asked for, and neither can
/// wrap into a negative, which PostgreSQL refuses.
fn sql_count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// The rows a status commit writes: its container when it is new, and the
/// version. On whichever client the caller holds: its own transaction, or the
/// one `create_ehr_with_status` has open (`db:H5.23`).
fn write_status(
    client: &mut impl GenericClient,
    ehr_id: &HierObjectId,
    container_uid: &str,
    container_created: Option<StoredInstant>,
    row: &VersionRow,
) -> Result<()> {
    if let Some(created) = container_created {
        let created_utc = PostgresqlStore::utc_seconds_to_timestamptz(created.utc_seconds)?;
        client
            .execute(
                "INSERT INTO openehr_versioned_object \
                 (uid, ehr_id, rm_type, time_created_text, time_created_utc) \
                 VALUES ($1, $2, $3, $4, $5)",
                &[
                    &container_uid,
                    &ehr_id.to_string(),
                    &"EHR_STATUS",
                    &created.text,
                    &created_utc,
                ],
            )
            .map_err(|e| engine(&e))?;
    }
    insert_version(client, row)
}

/// Inserts one `openehr_version` row, shared by `commit_composition` and
/// `commit_ehr_status` — the one piece of SQL identical between them.
fn insert_version(transaction: &mut impl GenericClient, row: &VersionRow) -> Result<()> {
    let committed_utc =
        PostgresqlStore::utc_seconds_to_timestamptz(row.audit_time_committed.utc_seconds)?;
    let tag_mac = row.chain.tag_mac.map(|m| m.to_vec());
    // Fallback: 0 — `db:H5.6` stores an absent trunk branch component as zero.
    let branch_number = row.branch_number.unwrap_or(0);
    let branch_version = row.branch_version.unwrap_or(0);
    // Bound to a slice with its element type stated up front: each `&row.…`
    // has a different concrete type (`&String`, `&i64`, `&bool`, `&[u8]`, …),
    // and Rust only coerces each to `&dyn ToSql + Sync` while constructing an
    // array literal whose element type is already known — an `as` cast
    // applied *after* the literal is built is too late; every element has
    // already been unified to one concrete type by then, and it fails to
    // compile with the other twenty-five.
    let params: &[&(dyn ToSql + Sync)] = &[
        &row.uid,
        &row.versioned_object_uid,
        &row.creating_system_id,
        &row.trunk_version,
        &branch_number,
        &branch_version,
        &row.preceding_version_uid,
        &row.lifecycle_state_code,
        &row.is_deleted,
        &row.contribution_uid,
        &row.audit_system_id,
        &row.audit_change_type_code,
        &row.audit_committer_name,
        &row.audit_committer_json,
        &row.audit_time_committed.text,
        &committed_utc,
        &row.data_json,
        &row.audit_description,
        &row.signature,
        &row.attestations_json,
        &row.other_input_version_uids_json,
        &row.chain.previous.as_slice(),
        &row.chain.content.as_slice(),
        &row.chain.digest.as_slice(),
        &row.chain.tag_key_id,
        &tag_mac,
    ];
    transaction
        .execute(
            "INSERT INTO openehr_version \
             (uid, versioned_object_uid, creating_system_id, trunk_version, branch_number, \
              branch_version, preceding_version_uid, lifecycle_state_code, is_deleted, \
              contribution_uid, audit_system_id, audit_change_type_code, \
              audit_committer_name, audit_committer_json, audit_time_committed_text, \
              audit_time_committed_utc, data_json, audit_description, signature, \
              attestations_json, other_input_version_uids_json, chain_previous, \
              chain_content, chain_digest, chain_tag_key_id, chain_tag_mac) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
                     $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)",
            params,
        )
        .map_err(|e| engine(&e))?;
    Ok(())
}

/// Inserts one `openehr_composition_index` row.
fn insert_composition_index(
    transaction: &mut postgres::Transaction<'_>,
    index: &CompositionIndexRow,
) -> Result<()> {
    let start_utc = PostgresqlStore::utc_seconds_to_timestamptz(
        index.context_start.as_ref().and_then(|i| i.utc_seconds),
    )?;
    let end_utc = PostgresqlStore::utc_seconds_to_timestamptz(
        index.context_end.as_ref().and_then(|i| i.utc_seconds),
    )?;
    transaction
        .execute(
            "INSERT INTO openehr_composition_index \
             (version_uid, ehr_id, archetype_id, template_id, category_code, \
              composer_name, language_code, territory_code, setting_code, \
              context_start_text, context_start_utc, context_end_text, context_end_utc) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            &[
                &index.version_uid,
                &index.ehr_id,
                &index.archetype_id,
                &index.template_id,
                &index.category_code,
                &index.composer_name,
                &index.language_code,
                &index.territory_code,
                &index.setting_code,
                &index.context_start.as_ref().map(|i| i.text.clone()),
                &start_utc,
                &index.context_end.as_ref().map(|i| i.text.clone()),
                &end_utc,
            ],
        )
        .map_err(|e| engine(&e))?;
    Ok(())
}
