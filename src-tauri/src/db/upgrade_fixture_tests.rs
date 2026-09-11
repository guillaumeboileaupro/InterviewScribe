use rusqlite::Connection;

const SCHEMA_V0: &str = include_str!("../../../tests/fixtures/db/schema-v0.sql");

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(Result::ok)
        .any(|name| name == column)
}

#[test]
fn schema_v0_fixture_migrates_without_losing_project_data() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(SCHEMA_V0).unwrap();

    super::schema::init(&conn).unwrap();

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
    assert!(has_column(&conn, "interview", "notes"));
    assert!(has_column(&conn, "interview", "transcription_cursor_ms"));
    assert!(has_column(&conn, "edit", "reverted_at"));

    let interview: (String, Option<String>) = conn
        .query_row(
            "SELECT title, notes FROM interview WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(interview, ("Projet synthetique N-1".into(), None));
    let segment: (String, String) = conn
        .query_row(
            "SELECT segment.raw_text, edit.after_text
             FROM segment JOIN edit ON edit.segment_id = segment.id
             WHERE segment.id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        segment,
        (
            "Texte synthetique immuable.".into(),
            "Texte synthetique edite.".into()
        )
    );

    for table in ["interview", "speaker", "segment", "edit", "setting"] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1, "fixture row lost from {table}");
    }

    super::schema::init(&conn).unwrap();
    let edit_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM edit", [], |row| row.get(0))
        .unwrap();
    assert_eq!(edit_count, 1, "idempotent migration duplicated data");
}
