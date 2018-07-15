extern crate rusqlite;

use self::rusqlite::Connection;
use errors::Result;
use ServerReply;

use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use std::fs;

pub struct Cache {
    conn: Connection
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

impl Cache {
    pub fn new(cache_dir: &PathBuf) -> Result<Cache> {
        fs::create_dir_all(cache_dir)?;
        let conn = Connection::open(cache_dir.join("anidb-rs.sqlite"))?;
        conn.execute("PRAGMA encoding=\"UTF-8\"", &[])?;
        conn.execute("CREATE TABLE IF NOT EXISTS apicall (
                      query TEXT PRIMARY KEY,
                      code INTEGER NOT NULL,
                      answer TEXT NOT NULL,
                      time_created INTEGER NOT NULL
                      )", &[])?;
        Ok(Cache {
            conn: conn
        })
    }

    pub fn get(&self, query: &str) -> Result<ServerReply> {
        let answer = self.conn.query_row("SELECT code, answer FROM apicall WHERE query = ?1",
                                         &[&query], |row| {
                                             ServerReply {
                                                 code: row.get(0),
                                                 data: row.get(1)
                                             }
                                         })?;
        Ok(answer)
    }

    pub fn put(&self, query: &str, reply: &ServerReply) -> Result<()> {
        
        self.conn.execute("INSERT INTO apicall (query, code, answer, time_created) VALUES(?, ?, ?, ?)",
                          &[&query, &reply.code, &reply.data, &now()])?;
        Ok(())
    }
}
