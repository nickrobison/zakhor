//! Integration tests for decision semantics (#96).
//!
//! These exercise the real SPARQL store (tinysparql via Tracker's API) through
//! the same `init_db` path the application uses, so query behaviour cannot
//! drift from what the server actually does.
//!
//! Coverage mirrors the acceptance criteria:
//! - every valid status filter returns its matching rows,
//! - supersede transitions write internally consistent tombstones,
//! - cleanup backfills missing tombstones and reports stale decisions
//!   according to its 30-day window.

use gio::Cancellable;
use oxiri::Iri;
use tracker::prelude::{SparqlConnectionExtManual, SparqlCursorExtManual};
use zakhor_model::background::cleanup_stale_data;
use zakhor_model::decision::{CreateDecisionArgs, DecisionModel};

/// A tombstone value that is always more than 30 days in the past relative
/// to `NOW()` at test time.
const ANCIENT_TOMBSTONE: &str = "2026-01-01T00:00:00Z";

fn iri(value: &str) -> Iri<String> {
    Iri::parse(value.to_owned()).expect("test IRIs should be valid")
}

/// Build a real Tracker store in a temp dir (same as the app's test harness).
fn test_store(tag: &str) -> tracker::SparqlConnection {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "zakhor-decision-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&path);
    zakhor_storage::tracker_db::init_db(
        path.to_str().expect("utf-8 temp path"),
        &zakhor_common::config::extraction_defaults::default_relation_labels(),
    )
    .expect("init test db")
}

fn args(context: &str) -> CreateDecisionArgs {
    CreateDecisionArgs {
        context: context.into(),
        outcome: format!("outcome-{context}"),
        alternatives: vec![],
        rationale: "rationale".into(),
        affects: vec![],
        derived_from: vec![],
        supersedes: None,
        conflicts_with: vec![],
        depends_on: vec![],
        project_uri: None,
    }
}

/// Run a SELECT, returning each row's cells as strings (NULL cells become
/// empty strings, matching how the API handlers read cursors).
fn ask(conn: &tracker::SparqlConnection, sparql: &str) -> Vec<Vec<String>> {
    let cursor = conn
        .query(sparql, None::<&Cancellable>)
        .unwrap_or_else(|e| panic!("query failed: {e}\n{sparql}"));
    let mut rows = Vec::new();
    while cursor
        .next(None::<&Cancellable>)
        .unwrap_or_else(|e| panic!("cursor: {e}"))
    {
        let mut row = Vec::new();
        for i in 0..cursor.n_columns() {
            row.push(cursor.string(i).map(|s| s.to_string()).unwrap_or_default());
        }
        rows.push(row);
    }
    rows
}

fn status_of(conn: &tracker::SparqlConnection, uri: &str) -> Option<String> {
    let rows = ask(
        conn,
        &format!(
            "PREFIX zakhor: <http://zakhor/ns/> \
             SELECT ?status WHERE {{ <{uri}> zakhor:decisionStatus ?status . }}"
        ),
    );
    rows.first().map(|r| r[0].clone())
}

fn tombstone_of(conn: &tracker::SparqlConnection, uri: &str) -> Option<String> {
    let rows = ask(
        conn,
        &format!(
            "PREFIX zakhor: <http://zakhor/ns/> \
             SELECT ?t WHERE {{ <{uri}> zakhor:tombstone ?t . }}"
        ),
    );
    rows.first().map(|r| r[0].clone())
}

// ---------------------------------------------------------------------------
// Status filtering — every valid status must return its matching rows (#96)
// ---------------------------------------------------------------------------

#[test]
fn each_valid_status_returns_its_matching_rows() {
    let conn = test_store("status-filter");
    let first = DecisionModel::create(&conn, args("first")).expect("create first");
    let second = DecisionModel::create(&conn, args("second")).expect("create second");

    // Retire the first decision the documented way: supersede it.
    DecisionModel::supersede(&conn, &first.decision_uri).expect("supersede first");

    let active = DecisionModel::query_by_status(&conn, "active", 100).expect("query active");
    let superseded =
        DecisionModel::query_by_status(&conn, "superseded", 100).expect("query superseded");

    assert_eq!(
        active,
        vec![second.decision_uri.clone()],
        "active filter must return the still-active decision"
    );
    assert_eq!(
        superseded,
        vec![first.decision_uri],
        "superseded filter must return the retired decision"
    );
}

#[test]
fn status_filter_uses_plain_literals_that_match_the_write_path() {
    let conn = test_store("plain-literal");
    let d = DecisionModel::create(&conn, args("plain")).expect("create");
    DecisionModel::supersede(&conn, &d.decision_uri).expect("supersede");

    // This is the query shape the HTTP list handler generates: a plain
    // (untagged) literal comparison against the bound ?status. If the write
    // path ever drifts to a tagged or typed literal, this returns 0 rows and
    // the test fails.
    let rows = ask(
        &conn,
        "PREFIX zakhor: <http://zakhor/ns/> \
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
         SELECT ?d WHERE { \
           ?d rdf:type zakhor:Decision . \
           ?d zakhor:decisionStatus ?status . \
           FILTER(?status = \"superseded\") \
         }",
    );
    assert_eq!(rows.len(), 1, "plain-literal filter must match");
    assert_eq!(rows[0][0], d.decision_uri.as_str());
}

// ---------------------------------------------------------------------------
// Supersede transitions (#96)
// ---------------------------------------------------------------------------

#[test]
fn supersede_transitions_status_and_writes_tombstone() {
    let conn = test_store("supersede");
    let d = DecisionModel::create(&conn, args("to-supersede")).expect("create");

    DecisionModel::supersede(&conn, &d.decision_uri).expect("supersede");

    assert_eq!(
        status_of(&conn, d.decision_uri.as_str()).as_deref(),
        Some("superseded"),
        "status must transition to superseded"
    );
    let tombstone = tombstone_of(&conn, d.decision_uri.as_str())
        .expect("supersede must record the transition time");
    assert!(
        tombstone.ends_with('Z') && tombstone.contains('T'),
        "tombstone must be an xsd:dateTime lexical value, got {tombstone}"
    );
}

#[test]
fn supersede_is_idempotent_and_keeps_the_original_tombstone() {
    let conn = test_store("idempotent");
    let d = DecisionModel::create(&conn, args("idempotent")).expect("create");
    DecisionModel::supersede(&conn, &d.decision_uri).expect("first supersede");
    let original = tombstone_of(&conn, d.decision_uri.as_str()).expect("tombstone");

    // Second supersede must succeed without rewriting history.
    DecisionModel::supersede(&conn, &d.decision_uri).expect("second supersede is a no-op");

    assert_eq!(
        tombstone_of(&conn, d.decision_uri.as_str()).as_deref(),
        Some(original.as_str()),
        "an already-superseded decision keeps its original tombstone"
    );
    assert_eq!(
        status_of(&conn, d.decision_uri.as_str()).as_deref(),
        Some("superseded")
    );
}

#[test]
fn supersede_rejects_targets_without_status() {
    let conn = test_store("not-a-decision");
    // An entity is not a decision: it has no decisionStatus.
    let uri = iri("http://zakhor/ns/entity/definitely-not-a-decision");
    let err =
        DecisionModel::supersede(&conn, &uri).expect_err("superseding a non-decision must fail");
    assert!(
        err.contains("no zakhor:decisionStatus"),
        "error must explain the missing status, got: {err}"
    );
}

#[test]
fn create_with_supersedes_retires_the_prior_decision() {
    let conn = test_store("create-supersedes");
    let old = DecisionModel::create(&conn, args("old")).expect("create old");

    let new_args = CreateDecisionArgs {
        supersedes: Some(old.decision_uri.clone()),
        ..args("new")
    };
    let new = DecisionModel::create(&conn, new_args).expect("create new");

    assert_eq!(new.status, "active", "the new decision is active");
    assert_eq!(
        status_of(&conn, old.decision_uri.as_str()).as_deref(),
        Some("superseded"),
        "create(supersedes) must retire the prior decision"
    );
    assert!(
        tombstone_of(&conn, old.decision_uri.as_str()).is_some(),
        "create(supersedes) must record the transition time"
    );

    // The supersedes edge is queryable.
    let rows = ask(
        &conn,
        &format!(
            "PREFIX zakhor: <http://zakhor/ns/> \
             SELECT ?o WHERE {{ <{}> zakhor:supersedes ?o . }}",
            new.decision_uri.as_str()
        ),
    );
    assert_eq!(rows, vec![vec![old.decision_uri.as_str().to_string()]]);
}

// ---------------------------------------------------------------------------
// Cleanup — tombstone backfill and the 30-day staleness window (#96)
// ---------------------------------------------------------------------------

#[test]
fn cleanup_backfills_missing_tombstones() {
    let conn = test_store("cleanup-backfill");
    let d = DecisionModel::create(&conn, args("backfill")).expect("create");
    // Flip the status directly so the tombstone is missing — the state a
    // pre-tombstone installation (or a manual SPARQL edit) would leave behind.
    conn.update(
        &format!(
            "PREFIX zakhor: <http://zakhor/ns/> \
             DELETE {{ <{}> zakhor:decisionStatus ?old . }} \
             INSERT {{ <{}> zakhor:decisionStatus \"superseded\" . }} \
             WHERE {{ <{}> zakhor:decisionStatus ?old . }}",
            d.decision_uri.as_str(),
            d.decision_uri.as_str(),
            d.decision_uri.as_str()
        ),
        None::<&Cancellable>,
    )
    .expect("manual supersede");

    let outcome = cleanup_stale_data(&conn).expect("cleanup");
    assert_eq!(outcome.tombstoned, 1, "the missing tombstone is backfilled");
    assert_eq!(outcome.stale, 0, "a fresh tombstone is not stale");

    let tombstone =
        tombstone_of(&conn, d.decision_uri.as_str()).expect("cleanup must write the tombstone");
    assert!(
        tombstone.contains('T') && tombstone.ends_with('Z'),
        "backfilled tombstone must be an xsd:dateTime, got {tombstone}"
    );
}

#[test]
fn cleanup_reports_stale_decisions_but_never_touches_active_ones() {
    let conn = test_store("cleanup-stale");
    let stale = DecisionModel::create(&conn, args("stale")).expect("create stale");
    let active = DecisionModel::create(&conn, args("active")).expect("create active");
    DecisionModel::supersede(&conn, &stale.decision_uri).expect("supersede");

    // Force the tombstone beyond the 30-day window.
    conn.update(
        &format!(
            "PREFIX zakhor: <http://zakhor/ns/> \
             PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> \
             DELETE {{ <{}> zakhor:tombstone ?t . }} \
             INSERT {{ <{}> zakhor:tombstone \"{ANCIENT_TOMBSTONE}\"^^xsd:dateTime . }} \
             WHERE {{ <{}> zakhor:tombstone ?t . }}",
            stale.decision_uri.as_str(),
            stale.decision_uri.as_str(),
            stale.decision_uri.as_str()
        ),
        None::<&Cancellable>,
    )
    .expect("age the tombstone");

    let outcome = cleanup_stale_data(&conn).expect("cleanup");
    assert_eq!(outcome.tombstoned, 0, "nothing to backfill");
    assert_eq!(
        outcome.stale, 1,
        "the >30-day-old superseded decision is stale"
    );

    // Active decisions are never tombstoned or reported stale.
    assert!(
        tombstone_of(&conn, active.decision_uri.as_str()).is_none(),
        "active decisions must not be tombstoned"
    );
    assert_eq!(
        status_of(&conn, active.decision_uri.as_str()).as_deref(),
        Some("active")
    );
}

// ---------------------------------------------------------------------------
// Ordering backing for `sort=created` — nrl:added is real, sortable data
// ---------------------------------------------------------------------------

#[test]
fn created_sort_is_backed_by_nrl_added() {
    let conn = test_store("created-sort");
    let d = DecisionModel::create(&conn, args("created")).expect("create");

    // The exact ORDER BY shape the HTTP list handler emits.
    let rows = ask(
        &conn,
        "PREFIX zakhor: <http://zakhor/ns/> \
             PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> \
             PREFIX nrl: <http://tracker.api.gnome.org/ontology/v3/nrl#> \
             SELECT ?id ?status ?added WHERE { \
               ?id rdf:type zakhor:Decision . \
               OPTIONAL { ?id zakhor:decisionStatus ?status . } \
               OPTIONAL { ?id nrl:added ?added . } \
             } ORDER BY DESC(?added) LIMIT 1",
    );
    let [row] = rows.as_slice() else {
        panic!("expected exactly one decision, got {rows:?}")
    };
    assert_eq!(row[0], d.decision_uri.as_str());
    assert_eq!(row[1], "active", "the real status is bound, not a default");
    assert!(
        !row[2].is_empty(),
        "nrl:added (the created sort key) must be bound"
    );
}
