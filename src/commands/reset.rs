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

  sqlx::query!("DELETE FROM FRONT WHERE Channel_ID=?", channel_id)
    .execute(&database)
    .await
    .expect("dump error");

  database.close().await;

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
