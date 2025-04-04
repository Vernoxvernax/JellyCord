use sqlx::Row;

use crate::Instance;

pub async fn get_front_database() -> Vec<Instance> {
  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to database");
  let db = sqlx::query!("SELECT * FROM FRONT WHERE Active_Channel = 1")
    .fetch_all(&database)
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
    .collect();
  database.close().await;
  db
}

pub async fn get_library_by_user(user_id: String) -> Vec<String> {
  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to database");
  let db_fetch = sqlx::query(format!("SELECT {:?} FROM LIBRARY", &user_id).as_str())
    .fetch_all(&database)
    .await
    .unwrap();
  let mut items: Vec<String> = vec![];

  for row in db_fetch {
    let id = row.get_unchecked(0);
    items.append(&mut vec![id]);
  }

  items
}
