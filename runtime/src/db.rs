use rusqlite::Connection;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static POOLS: RefCell<HashMap<String, Connection>> = RefCell::new(HashMap::new());
}

pub struct Database;

impl Database {
    pub fn open(path: &str) -> Connection {
        Connection::open(path).expect("Failed to open database")
    }

    pub fn with<F, R>(path: &str, f: F) -> R
    where
        F: FnOnce(&Connection) -> R,
    {
        POOLS.with(|p| {
            let mut pools = p.borrow_mut();
            if !pools.contains_key(path) {
                let conn = Connection::open(path).expect("Failed to open database");
                pools.insert(path.to_string(), conn);
            }
            let conn = pools.get(path).unwrap();
            f(conn)
        })
    }

    pub fn with_mut<F, R>(path: &str, f: F) -> R
    where
        F: FnOnce(&mut Connection) -> R,
    {
        POOLS.with(|p| {
            let mut pools = p.borrow_mut();
            if !pools.contains_key(path) {
                let conn = Connection::open(path).expect("Failed to open database");
                pools.insert(path.to_string(), conn);
            }
            let conn = pools.get_mut(path).unwrap();
            f(conn)
        })
    }

    pub fn init(path: &str, init_sql: &str) {
        POOLS.with(|p| {
            let mut pools = p.borrow_mut();
            if !pools.contains_key(path) {
                let conn = Connection::open(path).expect("Failed to open database");
                conn.execute_batch(init_sql).ok();
                pools.insert(path.to_string(), conn);
            }
        });
    }
}
