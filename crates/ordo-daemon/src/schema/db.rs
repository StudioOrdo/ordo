use rusqlite::{Connection, Params, Result as RusqliteResult, Row};

pub trait ConnectionExt {
    /// Ergonomically executes a query and collects the mapped results into a Vec.
    fn query_many<T, P, F>(&self, sql: &str, params: P, mapper: F) -> anyhow::Result<Vec<T>>
    where
        P: Params,
        F: FnMut(&Row<'_>) -> RusqliteResult<T>;

    /// Ergonomically executes a query, returning the first mapped result if it exists.
    fn query_one<T, P, F>(&self, sql: &str, params: P, mapper: F) -> anyhow::Result<Option<T>>
    where
        P: Params,
        F: FnOnce(&Row) -> rusqlite::Result<T>;
}

impl ConnectionExt for Connection {
    fn query_many<T, P, F>(&self, sql: &str, params: P, mapper: F) -> anyhow::Result<Vec<T>>
    where
        P: Params,
        F: FnMut(&Row<'_>) -> RusqliteResult<T>,
    {
        let mut statement = self.prepare(sql)?;
        let rows = statement.query_map(params, mapper)?;
        let result: RusqliteResult<Vec<T>> = rows.collect();
        Ok(result?)
    }

    fn query_one<T, P, F>(&self, sql: &str, params: P, mapper: F) -> anyhow::Result<Option<T>>
    where
        P: Params,
        F: FnOnce(&Row) -> rusqlite::Result<T>,
    {
        let mut statement = self.prepare(sql)?;
        let mut rows = statement.query(params)?;
        if let Some(row) = rows.next()? {
            Ok(Some(mapper(row)?))
        } else {
            Ok(None)
        }
    }
}
