mod common;

#[test]
fn test_creates_and_removes_db_files() {
    let test_db = common::TestDb::new();
    let pool = test_db.pool();
    let conn = pool.get();
    assert!(conn.is_ok());
}
