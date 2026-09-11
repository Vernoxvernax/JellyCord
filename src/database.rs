use std::{collections::HashSet, fmt, process::exit};

use sqlx::{AssertSqlSafe, Row, SqlitePool};

use crate::{Instance, jellyfin::Item};

#[derive(Debug)]
pub enum UnsafeSQLError {
  Empty,
  TooLong,
  InvalidChar,
}

impl fmt::Display for UnsafeSQLError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UnsafeSQLError::Empty => write!(f, "String is empty"),
      UnsafeSQLError::TooLong => write!(f, "String is too long"),
      UnsafeSQLError::InvalidChar => write!(f, "String contains invalid char"),
    }
  }
}

impl std::error::Error for UnsafeSQLError {}

pub fn safe_sql_string(name: &str) -> Result<String, UnsafeSQLError> {
  if name.is_empty() {
    return Err(UnsafeSQLError::Empty);
  } else if name.len() > 63 {
    return Err(UnsafeSQLError::TooLong);
  } else if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
    return Err(UnsafeSQLError::InvalidChar);
  }

  Ok(name.to_string())
}

pub async fn get_front_database(pool: &SqlitePool) -> Vec<Instance> {
  sqlx::query!("SELECT * FROM FRONT WHERE Active_Channel = 1")
    .fetch_all(pool)
    .await
    .unwrap()
    .iter()
    .map(|row| Instance {
      active_channel: row.Active_Channel,
      channel_id: row.Channel_ID,
      domain: row.Domain.clone(),
      token: row.Token.clone(),
      user_id: row.UserID.clone(),
    })
    .collect()
}

pub async fn get_library_by_user(pool: &SqlitePool, user_id: String) -> Vec<String> {
  let safe_userid = safe_sql_string(&user_id).unwrap();
  // unless someone tampered with the sql database, we should be able to assume that the user_id which we retrieved from it is not an injection
  // more of the same can be found in main.rs
  let db_fetch = sqlx::query(AssertSqlSafe(format!(
    "SELECT \"{}\" FROM LIBRARY",
    safe_userid
  )))
  .fetch_all(pool)
  .await
  .unwrap();
  let mut items: Vec<String> = vec![];

  for row in db_fetch {
    let id = row.get_unchecked(0);
    items.append(&mut vec![id]);
  }

  items
}

// saves item ids and returns new item ids in a vector
pub async fn get_new_items<'a>(
  pool: &SqlitePool,
  user_id: &str,
  items: &'a [Item],
) -> Result<Vec<&'a Item>, sqlx::Error> {
  let known: HashSet<String> = sqlx::query_scalar("SELECT item_id FROM LIBRARY WHERE UserID = ?")
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();

  Ok(
    items
      .iter()
      .filter(|item| !known.contains(&item.Id))
      .collect(),
  )
}

pub async fn mark_items_saved(
  pool: &SqlitePool,
  user_id: &str,
  items: &[Item],
) -> Result<(), sqlx::Error> {
  let mut item_ids: Vec<String> = vec![];
  for item in items {
    let safe_id = if let Ok(id) = safe_sql_string(&item.Id) {
      id
    } else {
      eprintln!(
        "Error: New item has an invalid id: \"{}\"!\nExiting...",
        item.Id
      );
      safe_sql_string(&item.Id).unwrap();
      exit(1);
    };

    item_ids.push(safe_id);
  }

  if item_ids.is_empty() {
    return Ok(());
  }

  let mut qb: sqlx::QueryBuilder<sqlx::Sqlite> =
    sqlx::QueryBuilder::new("INSERT OR IGNORE INTO library (UserID, item_id) ");

  qb.push_values(item_ids, |mut b, id| {
    b.push_bind(user_id).push_bind(id);
  });

  qb.build().execute(pool).await?;
  Ok(())
}
