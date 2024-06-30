use isahc::{ReadResponseExt, Request, RequestExt};
use serenity::all::{ChannelType, CommandDataOption, CommandDataOptionValue, CommandOptionType, CreateCommand, CreateCommandOption, Permissions};

use crate::{Instance, UserList};

pub async fn run(options: &[CommandDataOption]) -> String {
  let channel_id = match options.get(0).unwrap().value {
    CommandDataOptionValue::Channel(integer) => integer.get() as i64,
    _ => {
      panic!("Discord returned invalid command options.")
    }
  };
  let url = match &options.get(1).unwrap().value {
    CommandDataOptionValue::String(text) => text,
    _ => {
      panic!("Discord returned invalid command options.")
    }
  };
  let token = match &options.get(2).unwrap().value {
    CommandDataOptionValue::String(text) => text,
    _ => {
      panic!("Discord returned invalid command options.")
    }
  };
  let username = match &options.get(3).unwrap().value {
    CommandDataOptionValue::String(text) => text,
    _ => {
      panic!("Discord returned invalid command options.")
    }
  };

  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
  .expect("Couldn't connect to database");

  let domain = url.trim_end_matches('/').to_string();
  let users_request = Request::get(format!("{}/Users?api_key={}", &domain, &token)).body(());
  let users_response: Result<isahc::http::Response<_>, isahc::Error> = match users_request {
    Ok(response) => {
      response.send()
    },
    Err(_) => {
      database.close().await;
      return "The URL you've entered, seems to be of invalid format?\n- \"https://emby.yourdomain.com\"".to_string();
    }
  };

  let users: Result<Vec<UserList>, String> = match users_response {
    Ok(mut ok) => {
      let serde_attempt = serde_json::from_str::<Vec<UserList>>(&ok.text().unwrap());
      match serde_attempt {
        Ok(ok) => Ok(ok),
        Err(_) => {
          database.close().await;
          return "The request to retrieve available users failed.\nThis is likely due to an incorrect response or invalid api_key. Is this really a supported mediaserver?".to_string();
        }
      }
    },
    Err(err) => {
      database.close().await;
      return format!("The request to retrieve available users failed. Try to add \"https://\"\nError: {err}");
    }
  };
  
  let mut user_id_raw: Option<String> = None;
  for user in users.as_ref().unwrap().clone().into_iter() {
    if user.Name.to_lowercase() == username.to_lowercase().trim() {
      user_id_raw = Some(user.Id)
    }
  };
  if user_id_raw.is_none() {
    database.close().await;
    return "Username could not be found, please try again.".to_string();
  } else {
    let user_id = user_id_raw.clone().unwrap();

    if sqlx::query!(
      "SELECT UserID FROM FRONT WHERE UserID=? AND Channel_ID=?",
      user_id, channel_id,
    ).fetch_one(&database).await.is_ok() {
      database.close().await;
      return "This UserID has already been added.".to_string();
    };

    // If the table already exists in the database then just rename it.
    if sqlx::query(format!("SELECT {} FROM LIBRARY", &user_id).as_str()).fetch_one(&database).await.is_ok() {
      sqlx::query(
        format!("ALTER TABLE LIBRARY RENAME COLUMN {:?} TO \"{}_{}\"",
        &user_id, &user_id, chrono::offset::Utc::now().timestamp()
      ).as_str()).execute(&database).await
      .expect("couldn't rename database");
    };
  
    // Here, we can only create a new table for the database.
    // Previously, this segment also requested and inserted the library
    // from jellyfin into the database, but at least on my setup the
    // request alone greatly outlives the maximum timeout for discord's
    // command response, so we just leave it empty and fill it later
    // within the loop in `main.rs`.
    sqlx::query(
      format!("ALTER TABLE LIBRARY ADD {:?} VARCHAR(30)", &user_id).as_str())
      .execute(&database)
    .await.ok();

    let add = Instance {
      active_channel: 1,
      channel_id: channel_id as i64,
      domain,
      token: token.to_string(),
      user_id,
    };
    sqlx::query!(
      "INSERT INTO FRONT (Active_Channel, Channel_ID, Domain, Token, UserID) VALUES (?1, ?2, ?3, ?4, ?5)",
      add.active_channel, channel_id, add.domain, add.token, add.user_id).execute(&database)
    .await.expect("insert error");
    database.close().await;
  }
  
  "Setup successful.".to_string()
}

pub fn register() -> CreateCommand {
  CreateCommand::new("init")
    .description("Setup notifications for a channel")
    .add_option(
      CreateCommandOption::new(
        CommandOptionType::Channel,
        "channel",
        "Channel to receieve the notifications"
      )
      .channel_types([ChannelType::Text].to_vec())
      .required(true)
    )
    .add_option(
      CreateCommandOption::new(
        CommandOptionType::String,
        "url",
        "URL to JellyFin. Without `/web/*`"
      )
      .min_length(5)
      .required(true)
    )
    .add_option(
      CreateCommandOption::new(
        CommandOptionType::String,
        "api_key",
        "API key to access your mediacenter"
      )
      .required(true)
      .min_length(10)
    )
    .add_option(
      CreateCommandOption::new(
        CommandOptionType::String,
        "username",
        "An existing jellyfin/emby user. To limit the scope"
      )
      .required(true)
    )
  .default_member_permissions(Permissions::ADMINISTRATOR)
}
