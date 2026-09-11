use serenity::all::{
  ChannelType, CommandDataOption, CommandDataOptionValue, CommandOptionType, CreateCommand,
  CreateCommandOption, Permissions,
};

pub async fn run(options: &[CommandDataOption]) -> String {
  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to database");

  let channel_id = match options.get(0).unwrap().value {
    CommandDataOptionValue::Channel(integer) => integer.get() as i64,
    _ => {
      panic!("Discord returned invalid command options.")
    },
  };

  let mut tx = match database.begin().await {
    Ok(tx) => tx,
    Err(_) => return "Failed to start database transaction.".to_string(),
  };

  let user_id: Option<String> =
    match sqlx::query_scalar!("SELECT UserID FROM FRONT WHERE Channel_ID = ?", channel_id)
      .fetch_optional(&mut *tx)
      .await
    {
      Ok(id) => id,
      Err(_) => return "Failed to look up channel config.".to_string(),
    };

  let Some(user_id) = user_id else {
    return "No config found for that channel.".to_string();
  };

  if sqlx::query!("DELETE FROM FRONT WHERE Channel_ID = ?", channel_id)
    .execute(&mut *tx)
    .await
    .is_err()
  {
    return "Failed to delete channel config.".to_string();
  }

  if sqlx::query!("DELETE FROM LIBRARY WHERE UserID = ?", user_id)
    .execute(&mut *tx)
    .await
    .is_err()
  {
    return "Failed to clear library cache.".to_string();
  }

  if tx.commit().await.is_err() {
    return "Failed to commit changes.".to_string();
  }

  "Successfully reset channel.".to_string()
}

pub fn register() -> CreateCommand {
  CreateCommand::new("reset")
    .description("Reset a channel")
    .add_option(
      CreateCommandOption::new(CommandOptionType::Channel, "channel", "Channel to reset")
        .channel_types([ChannelType::Text].to_vec())
        .required(true),
    )
    .default_member_permissions(Permissions::ADMINISTRATOR)
}
