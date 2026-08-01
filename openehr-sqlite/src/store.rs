//! The `SQLite` store.

use crate::dialect::SqliteDialect;
use openehr::base::{HierObjectId, ObjectId, ObjectRef, ObjectVersionId};
use openehr::rm::common::{CommitError, Contribution, Version};
use openehr::rm::data_types::DvDateTime;
use openehr::rm::ehr::{Composition, Ehr};
use openehr::validation::Validate as _;
use openehr_store::record::{CompositionIndexRow, StoredInstant, VersionRow};
use openehr_store::{CommitOutcome, Result, Store, StoreError, ddl_script};
use rusqlite::{Connection, OptionalExtension as _, params};

/// The engine name used in errors.
const ENGINE: &str = "SQLite";

/// An openEHR repository in a `SQLite` database.
///
/// # Foreign keys are switched on explicitly
///
/// `SQLite` disables foreign-key enforcement by default, per connection, for
/// backward compatibility. A store that did not enable it would accept a
/// version pointing at a container that does not exist — and would do so
/// silently, which is worse than not having the constraint, because the schema
/// says it is there.
pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    /// Opens an in-memory database.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Engine`] if `SQLite` cannot be opened or configured.
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory().map_err(|e| engine(&e))?)
    }

    /// Opens a database file, creating it if absent.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Engine`] if the file cannot be opened.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        Self::from_connection(Connection::open(path).map_err(|e| engine(&e))?)
    }

    /// Wraps an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Engine`] if the connection cannot be configured.
    pub fn from_connection(connection: Connection) -> Result<Self> {
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| engine(&e))?;
        Ok(Self { connection })
    }

    /// The underlying connection, for callers that need a query this trait does
    /// not offer.
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Reads a version row from a query row.
    fn read_version(row: &rusqlite::Row<'_>) -> rusqlite::Result<VersionRow> {
        Ok(VersionRow {
            uid: row.get("uid")?,
            versioned_object_uid: row.get("versioned_object_uid")?,
            creating_system_id: row.get("creating_system_id")?,
            trunk_version: row.get("trunk_version")?,
            branch_number: row.get("branch_number")?,
            branch_version: row.get("branch_version")?,
            preceding_version_uid: row.get("preceding_version_uid")?,
            lifecycle_state_code: row.get("lifecycle_state_code")?,
            is_deleted: row.get::<_, i64>("is_deleted")? != 0,
            contribution_uid: row.get("contribution_uid")?,
            audit_system_id: row.get("audit_system_id")?,
            audit_change_type_code: row.get("audit_change_type_code")?,
            audit_committer_name: row.get("audit_committer_name")?,
            audit_time_committed: StoredInstant {
                text: row.get("audit_time_committed_text")?,
                utc_seconds: row.get("audit_time_committed_utc")?,
            },
            data_json: row.get("data_json")?,
        })
    }

    /// Every column of `openehr_version`, in one place so the two read paths
    /// cannot select different sets.
    const VERSION_COLUMNS: &'static str = "uid, versioned_object_uid, creating_system_id, \
        trunk_version, branch_number, branch_version, preceding_version_uid, \
        lifecycle_state_code, is_deleted, contribution_uid, audit_system_id, \
        audit_change_type_code, audit_committer_name, audit_time_committed_text, \
        audit_time_committed_utc, data_json";
}

/// Translates a uniqueness violation on the version table into the commit
/// refusal it actually is.
///
/// The single-threaded path checks the commit rules before inserting, so this
/// only fires under **concurrency**: two writers both read the same head, both
/// pass the check, and the database refuses the second. That is the unique
/// index of `db:H5.10` doing its job — the rule holds in the database and not
/// only in the library.
///
/// Reporting it as `Engine` would satisfy the guarantee and fail the caller.
/// `db:H5.9` requires refusals to be **distinguishable**: a caller told
/// `Commit` knows another writer won and can re-read the head and retry, while
/// a caller told "UNIQUE constraint failed" knows only that something went
/// wrong — and a version tree is precisely where guessing is not allowed.
///
/// The two indexes mean different things and map differently:
///
/// - `openehr_version.uid` — the same version identity was committed twice.
/// - `ix_version_container_trunk` — a *different* identity took that position
///   in the tree, which is a concurrent modification rather than a duplicate.
fn commit_conflict(error: &rusqlite::Error) -> Option<StoreError> {
    use rusqlite::ErrorCode::ConstraintViolation;
    let rusqlite::Error::SqliteFailure(code, Some(message)) = error else {
        return None;
    };
    if code.code != ConstraintViolation {
        return None;
    }
    if message.contains("openehr_version.uid") {
        Some(StoreError::Commit(CommitError::DuplicateVersion))
    } else if message.contains("ix_version_container_trunk") {
        Some(StoreError::Commit(CommitError::NotLatest))
    } else {
        None
    }
}

/// Wraps a driver error without letting row data into the message.
fn engine(error: &rusqlite::Error) -> StoreError {
    if let Some(conflict) = commit_conflict(error) {
        return conflict;
    }
    StoreError::Engine {
        engine: ENGINE,
        // `to_string` on a rusqlite error gives the SQLite message, which names
        // constraints and columns and not values. The one exception SQLite
        // makes is a `CHECK` message, which is why this schema's constraints
        // carry no interpolated values.
        message: error.to_string(),
    }
}

impl Store for SqliteStore {
    fn engine(&self) -> &'static str {
        ENGINE
    }

    fn install(&mut self) -> Result<()> {
        self.connection
            .execute_batch(&ddl_script(&SqliteDialect))
            .map_err(|e| engine(&e))
    }

    fn create_ehr(&mut self, ehr: &Ehr) -> Result<()> {
        let id = ehr.ehr_id().to_string();
        let existing: Option<String> = self
            .connection
            .query_row(
                "SELECT ehr_id FROM openehr_ehr WHERE ehr_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| engine(&e))?;
        if existing.is_some() {
            return Err(StoreError::Conflict { kind: "ehr", id });
        }
        let created = StoredInstant::from_date_time(ehr.time_created().value());
        self.connection
            .execute(
                "INSERT INTO openehr_ehr \
                 (ehr_id, system_id, time_created_text, time_created_utc, ehr_status_uid, ehr_access_uid) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    id,
                    ehr.system_id().to_string(),
                    created.text,
                    created.utc_seconds,
                    ehr.ehr_status().id().to_string(),
                    ehr.ehr_access().id().to_string(),
                ],
            )
            .map_err(|e| engine(&e))?;
        Ok(())
    }

    fn get_ehr(&self, ehr_id: &HierObjectId) -> Result<Ehr> {
        let id = ehr_id.to_string();
        let row: Option<(String, String, String, String)> = self
            .connection
            .query_row(
                "SELECT system_id, time_created_text, ehr_status_uid, ehr_access_uid \
                 FROM openehr_ehr WHERE ehr_id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|e| engine(&e))?;
        let Some((system_id, created, status_uid, access_uid)) = row else {
            return Err(StoreError::NotFound { kind: "ehr", id });
        };
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
        ))
    }

    fn create_contribution(
        &mut self,
        ehr_id: &HierObjectId,
        contribution: &Contribution,
    ) -> Result<()> {
        let uid = contribution.uid().to_string();
        let audit = contribution.audit();
        let committed = StoredInstant::from_date_time(audit.time_committed().value());
        self.connection
            .execute(
                "INSERT INTO openehr_contribution \
                 (uid, ehr_id, audit_change_type_code, audit_system_id, audit_committer_name, \
                  audit_time_committed_text, audit_time_committed_utc) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    uid,
                    ehr_id.to_string(),
                    audit.change_type_code(),
                    audit.system_id(),
                    audit.committer().name(),
                    committed.text,
                    committed.utc_seconds,
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _)
                    if f.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    StoreError::Conflict {
                        kind: "contribution",
                        id: uid.clone(),
                    }
                }
                ref other => engine(other),
            })?;
        Ok(())
    }

    // Long because it is the whole commit path: two gates, the head lookup, the
    // container, the version, and the index — in one transaction. The order is
    // the safety property, so it stays visible in one place.
    #[allow(clippy::too_many_lines)]
    fn commit_composition(
        &mut self,
        ehr_id: &HierObjectId,
        version: &Version<Composition>,
        contribution_uid: &str,
    ) -> Result<CommitOutcome> {
        // Gate one: the content must satisfy the Reference Model. A store that
        // accepted an invalid composition would make every later reader's
        // `validate()` fail on data it cannot fix.
        if let Some(composition) = version.data() {
            composition.validate_ok()?;
        }

        // The EHR must exist. Without this the foreign key would fire on the
        // container insert with a message about a constraint rather than about
        // a missing record.
        self.get_ehr(ehr_id)?;

        let container_uid = version.uid().object_id().to_string();
        let head: Option<(String, i64)> = self
            .connection
            .query_row(
                "SELECT uid, trunk_version FROM openehr_version \
                 WHERE versioned_object_uid = ?1 \
                 ORDER BY trunk_version DESC, branch_number DESC, branch_version DESC LIMIT 1",
                params![container_uid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| engine(&e))?;

        // Gate two: the same commit rules the library enforces, in the same
        // order, so a caller sees the same refusal whether the history is in
        // memory or in a database (V8.1–V8.5).
        let uid = version.uid().to_string();
        let already: Option<String> = self
            .connection
            .query_row(
                "SELECT uid FROM openehr_version WHERE uid = ?1",
                params![uid],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| engine(&e))?;
        if already.is_some() {
            return Err(StoreError::Commit(CommitError::DuplicateVersion));
        }
        match (&head, version.preceding_version_uid()) {
            (None, None) => {}
            (None, Some(_)) | (Some(_), None) => {
                return Err(StoreError::Commit(CommitError::PrecedingVersionMismatch));
            }
            (Some((latest, _)), Some(preceding)) => {
                if latest != &preceding.to_string() {
                    return Err(StoreError::Commit(CommitError::NotLatest));
                }
            }
        }

        let row = VersionRow::project(version, contribution_uid)?;
        let created_container = head.is_none();
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|e| engine(&e))?;

        if created_container {
            let created =
                StoredInstant::from_date_time(version.commit_audit().time_committed().value());
            transaction
                .execute(
                    "INSERT INTO openehr_versioned_object \
                     (uid, ehr_id, rm_type, time_created_text, time_created_utc) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        container_uid,
                        ehr_id.to_string(),
                        "COMPOSITION",
                        created.text,
                        created.utc_seconds
                    ],
                )
                .map_err(|e| engine(&e))?;
        }

        transaction
            .execute(
                "INSERT INTO openehr_version \
                 (uid, versioned_object_uid, creating_system_id, trunk_version, branch_number, \
                  branch_version, preceding_version_uid, lifecycle_state_code, is_deleted, \
                  contribution_uid, audit_system_id, audit_change_type_code, \
                  audit_committer_name, audit_time_committed_text, audit_time_committed_utc, \
                  data_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    row.uid,
                    row.versioned_object_uid,
                    row.creating_system_id,
                    row.trunk_version,
                    row.branch_number,
                    row.branch_version,
                    row.preceding_version_uid,
                    row.lifecycle_state_code,
                    i64::from(row.is_deleted),
                    row.contribution_uid,
                    row.audit_system_id,
                    row.audit_change_type_code,
                    row.audit_committer_name,
                    row.audit_time_committed.text,
                    row.audit_time_committed.utc_seconds,
                    row.data_json,
                ],
            )
            .map_err(|e| engine(&e))?;

        if let Some(composition) = version.data() {
            let index = CompositionIndexRow::project(&row.uid, &ehr_id.to_string(), composition)?;
            transaction
                .execute(
                    "INSERT INTO openehr_composition_index \
                     (version_uid, ehr_id, archetype_id, template_id, category_code, \
                      composer_name, language_code, territory_code, setting_code, \
                      context_start_text, context_start_utc, context_end_text, context_end_utc) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        index.version_uid,
                        index.ehr_id,
                        index.archetype_id,
                        index.template_id,
                        index.category_code,
                        index.composer_name,
                        index.language_code,
                        index.territory_code,
                        index.setting_code,
                        index.context_start.as_ref().map(|i| i.text.clone()),
                        index.context_start.as_ref().and_then(|i| i.utc_seconds),
                        index.context_end.as_ref().map(|i| i.text.clone()),
                        index.context_end.as_ref().and_then(|i| i.utc_seconds),
                    ],
                )
                .map_err(|e| engine(&e))?;
        }

        transaction.commit().map_err(|e| engine(&e))?;
        Ok(CommitOutcome {
            version_uid: version.uid().clone(),
            created_container,
        })
    }

    fn get_version(&self, uid: &ObjectVersionId) -> Result<VersionRow> {
        let id = uid.to_string();
        self.connection
            .query_row(
                &format!(
                    "SELECT {} FROM openehr_version WHERE uid = ?1",
                    Self::VERSION_COLUMNS
                ),
                params![id],
                Self::read_version,
            )
            .optional()
            .map_err(|e| engine(&e))?
            .ok_or(StoreError::NotFound {
                kind: "version",
                id,
            })
    }

    fn latest_version(&self, versioned_object_uid: &HierObjectId) -> Result<VersionRow> {
        let id = versioned_object_uid.to_string();
        self.connection
            .query_row(
                &format!(
                    "SELECT {} FROM openehr_version WHERE versioned_object_uid = ?1 \
                     ORDER BY trunk_version DESC, branch_number DESC, branch_version DESC LIMIT 1",
                    Self::VERSION_COLUMNS
                ),
                params![id],
                Self::read_version,
            )
            .optional()
            .map_err(|e| engine(&e))?
            .ok_or(StoreError::NotFound {
                kind: "versioned_object",
                id,
            })
    }

    fn version_at_time(
        &self,
        versioned_object_uid: &HierObjectId,
        at: &DvDateTime,
    ) -> Result<VersionRow> {
        let id = versioned_object_uid.to_string();
        let Some(at_seconds) = StoredInstant::from_date_time(at.value()).utc_seconds else {
            // The query instant is not established — a local time with no
            // offset. Answering with *some* version would be a guess about the
            // zone, so this refuses, exactly as
            // `VersionedObject::version_at_time` returns `None` (V8.6).
            return Err(StoreError::NotFound {
                kind: "version",
                id,
            });
        };
        // `audit_time_committed_utc IS NOT NULL` is not redundant: a version
        // whose commit time carries no offset has a NULL here, and SQLite's
        // comparison would exclude it anyway — but stating it makes the
        // skipping deliberate rather than incidental.
        self.connection
            .query_row(
                &format!(
                    "SELECT {} FROM openehr_version \
                     WHERE versioned_object_uid = ?1 \
                       AND audit_time_committed_utc IS NOT NULL \
                       AND audit_time_committed_utc <= ?2 \
                     ORDER BY audit_time_committed_utc DESC, trunk_version DESC LIMIT 1",
                    Self::VERSION_COLUMNS
                ),
                params![id, at_seconds],
                Self::read_version,
            )
            .optional()
            .map_err(|e| engine(&e))?
            .ok_or(StoreError::NotFound {
                kind: "version",
                id,
            })
    }

    fn all_versions(&self, versioned_object_uid: &HierObjectId) -> Result<Vec<VersionRow>> {
        let mut statement = self
            .connection
            .prepare(&format!(
                "SELECT {} FROM openehr_version WHERE versioned_object_uid = ?1 \
                 ORDER BY trunk_version ASC, branch_number ASC, branch_version ASC",
                Self::VERSION_COLUMNS
            ))
            .map_err(|e| engine(&e))?;
        let rows = statement
            .query_map(
                params![versioned_object_uid.to_string()],
                Self::read_version,
            )
            .map_err(|e| engine(&e))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| engine(&e))
    }

    fn find_compositions_by_archetype(
        &self,
        ehr_id: &HierObjectId,
        archetype_id: &str,
    ) -> Result<Vec<CompositionIndexRow>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT version_uid, ehr_id, archetype_id, template_id, category_code, \
                        composer_name, language_code, territory_code, setting_code, \
                        context_start_text, context_start_utc, context_end_text, context_end_utc \
                 FROM openehr_composition_index \
                 WHERE ehr_id = ?1 AND archetype_id = ?2 \
                 ORDER BY version_uid",
            )
            .map_err(|e| engine(&e))?;
        let rows = statement
            .query_map(params![ehr_id.to_string(), archetype_id], |row| {
                let instant = |text: Option<String>, utc: Option<i64>| {
                    text.map(|text| StoredInstant {
                        text,
                        utc_seconds: utc,
                    })
                };
                Ok(CompositionIndexRow {
                    version_uid: row.get("version_uid")?,
                    ehr_id: row.get("ehr_id")?,
                    archetype_id: row.get("archetype_id")?,
                    template_id: row.get("template_id")?,
                    category_code: row.get("category_code")?,
                    composer_name: row.get("composer_name")?,
                    language_code: row.get("language_code")?,
                    territory_code: row.get("territory_code")?,
                    setting_code: row.get("setting_code")?,
                    context_start: instant(
                        row.get("context_start_text")?,
                        row.get("context_start_utc")?,
                    ),
                    context_end: instant(row.get("context_end_text")?, row.get("context_end_utc")?),
                })
            })
            .map_err(|e| engine(&e))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| engine(&e))
    }
}
