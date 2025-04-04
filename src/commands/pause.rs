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

  sqlx::query!(
    "UPDATE FRONT SET Active_Channel = 0 WHERE Channel_ID=?",
    channel_id
  )
  .execute(&database)
  .await
  .expect("pause error");

  match sqlx::query!(
    "SELECT Active_Channel FROM FRONT WHERE Channel_ID=?",
    channel_id
  )
  .fetch_one(&database)
  .await
  {
    Ok(fd) => {
      if fd.Active_Channel == 1 {
        sqlx::query!(
          "UPDATE FRONT SET Active_Channel = 0 WHERE Channel_ID=?",
          channel_id
        )
        .execute(&database)
        .await
        .expect("pause error");
        database.close().await;
        "Successfully paused channel.".to_string()
      } else {
        sqlx::query!(
          "UPDATE FRONT SET Active_Channel = 1 WHERE Channel_ID=?",
          channel_id
        )
        .execute(&database)
        .await
        .expect("unpause error");
        database.close().await;
        "Successfully unpaused channel.".to_string()
      }
    },
    _ => "Internal error".to_string(),
  }
}

pub fn register() -> CreateCommand {
  CreateCommand::new("pause")
    .description("Un/Pause notifications for a channel")
    .add_option(
      CreateCommandOption::new(
        CommandOptionType::Channel,
        "channel",
        "Channel to un/pause the notifications",
      )
      .channel_types([ChannelType::Text].to_vec())
      .required(true),
    )
    .default_member_permissions(Permissions::ADMINISTRATOR)
}
