#![allow(non_snake_case)]
use config::{Config, File};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;
use std::time::Duration;
use std::env;
use chrono;
use http::StatusCode;
use isahc::Request;
use isahc::prelude::*;
use serde_derive::{Serialize, Deserialize};
use serenity::async_trait;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::prelude::*;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};

mod database;
use database::*;

#[derive(Deserialize)]
struct ConfigFile {
  discord_token: String,
  command_prefix: Option<char>
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Instance {
  pub active_channel: i64,
  pub channel_id: i64,
  pub domain: String,
  pub token: String,
  pub user_id: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct UserList {
  Name: String,
  Id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MediaResponse {
  Items: Vec<Library>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Library {
  Name: Option<String>,
  SeriesId: Option<String>,
  Id: String,
  Type: String,
  MediaStreams: Option<Vec<MediaStream>>,
  CommunityRating: Option<f64>,
  RunTimeTicks: Option<u64>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MediaStream {
  Type: String,
  Language: Option<String>,
  Height: Option<u64>,
}


#[group]
#[commands(ping, help, init, dump, pause, unpause)]
struct General;

struct Handler {
  is_loop_running: AtomicBool,
}


#[async_trait]
impl EventHandler for Handler {
  async fn ready(&self, ctx: Context, ready: Ready) {
    println!("{} is connected!", ready.user.name);
    ctx.set_activity(Activity::watching("the internet.")).await;
  }
  
  async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
    println!("Cache built successfully!");
    if !self.is_loop_running.load(Ordering::Relaxed) {
      tokio::spawn(async move {
        loop {
          let front_db = get_front_database().await;
          for server in front_db {
            let timed_response_obj = get_serialized_page(
              format!("{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series,Episode&Fields=MediaStreams",
              server.domain, server.user_id, server.token)
            );
            if timed_response_obj.is_err() {
              eprintln!("Failed to connect to the server. {}", server.domain);
              continue
            } else {
              let serialized_server: MediaResponse = timed_response_obj.unwrap();

              let lib = get_library_by_user(server.clone().user_id).await;

              let mut library_stringed: Vec<String> = vec![];
              for item in lib {
                library_stringed.push(item);
              }
              let mut pre_new_items: Vec<Library> = vec![];
              let mut fullseries: Vec<Library> = vec![];
              for item in serialized_server.Items {
                if ! library_stringed.contains(&item.Id) {
                  pre_new_items.append(&mut vec![item.clone()]);
                  if item.Type == "Series".to_string() {
                    fullseries.append(&mut vec![item]);
                  }
                }
              }

              let mut new_items: Vec<Library> = vec![];
              for item in pre_new_items {
                if item.Type == "Episode".to_string() {
                  let mut skip: bool = false;
                  for series in fullseries.clone() {
                    if item.Id == series.Id {
                      skip = true;
                      break;
                    }
                  }
                  if ! skip {
                    new_items.append(&mut vec![item.clone()]);
                  }
                } else {
                  new_items.append(&mut vec![item.clone()]);
                }
              }

              if ! new_items.is_empty() {
                for x in new_items.clone() {
                  let image = format!("{}/Items/{}/Images/Primary?api_key={}&Quality=100", server.domain, x.clone().SeriesId.unwrap_or(x.clone().Id), server.token);
                  let (resolution, languages) = if x.MediaStreams.is_some() {
                    let mut height: String = String::new();
                    let mut languages: String = String::new();
                    for x in x.MediaStreams.unwrap() {
                      if x.Type == "Video" {
                        height = x.Height.unwrap().to_string();
                      }
                      if x.Type == "Audio" {
                        languages.push_str(&(x.Language.unwrap_or("?".to_string())+", "))
                      }
                    }
                    if height.is_empty() {
                      height = "?".to_string()
                    }
                    if languages.is_empty() {
                      languages = "?".to_string()
                    } else if languages != "?".to_string() {
                      languages = languages.strip_suffix(", ").unwrap().to_string();
                    }
                    (height+"p", languages)
                  } else {
                    ("?".to_string(), "?".to_string())
                  };
                  let runtime: String = if x.RunTimeTicks.is_some() {
                    let time = (x.RunTimeTicks.unwrap() as f64) / 10000000.0;
                    let formated: String = if time > 60.0 {
                      if (time / 60.0) > 60.0 {
                          format!("{:02}:{:02}:{:02}", ((time / 60.0) / 60.0).trunc(), ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                      } else {
                          format!("00:{:02}:{:02}", (time / 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                      }
                    } else {
                        format!("00:00:{:02}", time)
                    };
                    formated
                  } else {
                    "?".to_string()
                  };

                  let res = ChannelId(server.channel_id as u64)
                    .send_message(&ctx, |m| {
                      m.embed(|e| {
                        e.title(x.Name.unwrap())
                        .image(image)
                      }).add_embed(|e| {
                        e.field(":star: — Rating".to_string(), if x.CommunityRating.is_some() { x.CommunityRating.unwrap().to_string() } else { "?".to_string() }, true)
                        .field(":film_frames: — Runtime".to_string(), runtime, true)
                        .field(":frame_photo: — Resolution".to_string(), resolution, true)
                        .field(":loud_sound: — Languages".to_string(), languages, false)
                      })
                  }).await;
                  if let Err(why) = res {
                    eprintln!("Error sending message: {:?}", why);
                  } else {
                    let database = sqlx::sqlite::SqlitePoolOptions::new()
                      .max_connections(5)
                      .connect_with(
                      sqlx::sqlite::SqliteConnectOptions::new()
                      .filename("jellycord.sqlite")
                      .create_if_missing(true),
                    ).await
                    .expect("Couldn't connect to database");
                    sqlx::query(format!("INSERT INTO LIBRARY ({:?}) VALUES (\"{}\")", &server.user_id, &x.Id).as_str()).execute(&database)
                    .await.expect("insert error");
                  }
                };
              }
            }
          };
          tokio::time::sleep(Duration::from_secs(300)).await;
        };
      });
      self.is_loop_running.swap(true, Ordering::Relaxed);
    }
  }
}


#[tokio::main]
async fn main() {
  // Update / Create database structure
  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to database");
  sqlx::migrate!("./migrations").run(&database).await.expect("Couldn't run database migrations");
  database.close().await;
  if env::var("SETUP") == Ok("1".to_string()) {
    exit(0x100);
  };
  let settings_file_raw = Config::builder().add_source(File::from(Path::new(&"./jellycord.yaml".to_string()))).build().unwrap();
  let serialized = settings_file_raw.try_deserialize::<ConfigFile>().expect("Reading config file.");
  let framework = StandardFramework::new()
    .configure(|c| c.prefix(serialized.command_prefix.unwrap_or('~'))) 
    .group(&GENERAL_GROUP);
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(serialized.discord_token, intents)
    .event_handler(Handler {
      is_loop_running: AtomicBool::new(false),
    })
    .framework(framework)
    .await
    .expect("Error creating client");
  if let Err(why) = client.start().await {
    println!("An error occurred while running the client: {:?}", why);
  }
}


#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
  let _help_ = "```\
[JellyCord]

Commands:
  \"init\" - Initialize current channel and setup jellyfin connection
  \"dump\" - Break jellyfin connection for the current channel
  \"pause\" - Send all announcements to 127.0.0.1 instead
  \"unpause\" - Reactivate announcements
```";
  msg.reply(ctx, _help_).await?;

  Ok(())
}


#[command]
async fn init(ctx: &Context, msg: &Message) -> CommandResult {
  let channel_id = msg.channel_id.0 as i64;
  let thread = msg.channel_id.create_public_thread(ctx, msg.id, |t| t.name("JellyCord - Initialize")).await?;
  thread.say(ctx, "Please enter your jellyfin/emby address.\nYou can stop this process by typing \"quit\" right now.").await?;
  loop {
    let user_reply = msg.author.await_reply(&ctx).await.unwrap();
    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
          sqlx::sqlite::SqliteConnectOptions::new()
            .filename("jellycord.sqlite")
            .create_if_missing(true),
        )
        .await
        .expect("Couldn't connect to database");
    let domain: Result<String, ()> = if user_reply.content == "quit" {
      thread.delete(ctx).await?;
      break
    } else {
      let end = if user_reply.content.ends_with("/") {
        &user_reply.content[0..&user_reply.content.len() - 1]
      } else {
        &user_reply.content
      };
      Ok(end.to_string())
    };
    let domain = domain.unwrap();
    let api_question = user_reply.reply(&ctx, "Please create an api token and enter it here.").await?;
    let api_token = &msg.author.await_reply(&ctx).await.unwrap();
    api_token.delete(&ctx).await?;
    api_question.delete(&ctx).await?;
    let users_request = Request::get(format!("{}/Users?api_key={}", &domain, &api_token.content)).body(());
    let users_response: Result<http::Response<_>, isahc::Error> = match users_request {
      Ok(response) => {
        response.send()
      },
      Err(_) => {
        thread.say(&ctx, format!("The URL you've entered, seems to be of invalid format?\n- \"https://emby.yourdomain.com\"").as_str()).await?;
        thread.say(ctx, "Please try again by entering your jellyfin/emby address.").await?;
        continue
      }
    };
    let users: Result<Vec<UserList>, String> = match users_response {
      Ok(mut ok) => {
        let serde_attempt = serde_json::from_str::<Vec<UserList>>(&ok.text().unwrap());
        match serde_attempt {
          Ok(ok) => Ok(ok),
          Err(_) => {
            thread.say(&ctx, format!("The request to retrieve available users failed.\nThis is likely due to an incorrect response. Is this really a supported mediaserver?").as_str()).await?;
            thread.say(ctx, "Please try again by entering your jellyfin/emby address.").await?;
            continue
          }
        }
      },
      Err(err) => {
        thread.say(&ctx, format!("The request to retrieve available users failed. Try to add \"https://\"\nError: {}", err).as_str()).await?;
        thread.say(ctx, "Please try again by entering your jellyfin/emby address.").await?;
        continue
      }
    };
    thread.say(&ctx, "Received the token.\nNow enter the username, for which you would like to receive the notifications.").await?;
    loop {
      let username = &msg.author.await_reply(&ctx).await.unwrap().content;
      let mut user_id_raw: Option<String> = None;
      for user in users.as_ref().unwrap().clone().into_iter() {
        if user.Name.to_lowercase() == username.to_lowercase().trim() {
          user_id_raw = Some(user.Id)
        }
      };
      if user_id_raw.is_none() {
        thread.say(&ctx, "Username could not be found, please enter a different one.").await?;
      } else {
        let user_id = user_id_raw.clone().unwrap();
        if sqlx::query!(
          "SELECT UserID FROM FRONT WHERE UserID=? AND Channel_ID=?",
          user_id, channel_id,
        ).fetch_one(&database).await.is_ok() {
          msg.reply(&ctx, "This UserID has already been added.\nUse \"dump\" to recreate a connection.").await.unwrap();
          break
        };
        let timed_response = get_serialized_page(format!("{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series,Episode&Fields=MediaStreams", &domain, &user_id_raw.unwrap(), &api_token.content));
        let serialized = match timed_response {
          Ok(ok) => {
            ok
          },
          Err(_) => {
            continue
          }
        };
        if sqlx::query(format!("SELECT {} FROM LIBRARY", &user_id).as_str()).fetch_one(&database).await.is_ok() {
          sqlx::query(format!("ALTER TABLE LIBRARY RENAME COLUMN {:?} TO \"{}_{}\"", &user_id, &user_id, chrono::offset::Utc::now().timestamp()).as_str()).execute(&database).await.expect("couldn't rename database");
        };
        sqlx::query(
          format!("ALTER TABLE LIBRARY ADD {:?} VARCHAR(30)", &user_id).as_str()).execute(&database)
        .await.expect("insert error");
        let mut id_as_value: String = String::new();
        for item in serialized.clone().Items {
          if &item == serialized.Items.last().unwrap() {
            id_as_value.push_str(format!("(\"{}\")", item.Id).as_str());
          } else {
            id_as_value.push_str(format!("(\"{}\"),", item.Id).as_str());
          }
        };
        sqlx::query(format!("INSERT INTO LIBRARY ({:?}) VALUES {}", &user_id, &id_as_value).as_str()).execute(&database)
        .await.expect("insert error");
        let add = Instance {
          active_channel: 1,
          channel_id: channel_id,
          domain,
          token: api_token.content.clone(),
          user_id: user_id.to_string(),
        };
        let channel_id = add.channel_id as i64;
        sqlx::query!(
          "INSERT INTO FRONT (Active_Channel, Channel_ID, Domain, Token, UserID) VALUES (?1, ?2, ?3, ?4, ?5)",
          add.active_channel, channel_id, add.domain, add.token, add.user_id).execute(&database)
        .await.expect("insert error");
        thread.say(&ctx, "Username has been found and added to the configuration.").await?;
        break
      }
    }
    database.close().await;
    break
  };
  thread.delete(ctx).await?;
  msg.delete(ctx).await?;
  Ok(())
}


#[command]
async fn dump(ctx: &Context, msg: &Message) -> CommandResult {
  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to database");
  let channel_id = *&msg.channel_id.0 as i64;
  sqlx::query!(
    "DELETE FROM FRONT WHERE Channel_ID=?",
    channel_id).execute(&database)
  .await.expect("dump error");
  database.close().await;
  msg.reply(&ctx, "Removed all connections for this channel.").await?;
  Ok(())
}

#[command]
async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to database");
  let channel_id = *&msg.channel_id.0 as i64;
  sqlx::query!(
    "UPDATE FRONT SET Active_Channel = 0 WHERE Channel_ID=?",
    channel_id).execute(&database)
    .await.expect("pause error");
  database.close().await;
  msg.reply(&ctx, "Paused all connections for this channel.").await?;
  Ok(())
}


#[command]
async fn unpause(ctx: &Context, msg: &Message) -> CommandResult {
  let database = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
      sqlx::sqlite::SqliteConnectOptions::new()
        .filename("jellycord.sqlite")
        .create_if_missing(true),
    )
    .await
    .expect("Couldn't connect to database");
  let channel_id = *&msg.channel_id.0 as i64;
  sqlx::query!(
    "UPDATE FRONT SET Active_Channel = 1 WHERE Channel_ID=?",
    channel_id).execute(&database)
    .await.expect("unpause error");
  database.close().await;
  msg.reply(&ctx, "Unpaused all connections for this channel.").await?;
  Ok(())
}


#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
  msg.reply(ctx, "Pong!").await?;
  Ok(())
}


fn get_serialized_page(url: String) -> Result<MediaResponse, ()> {
  let web_request = Request::get(url).timeout(Duration::from_secs(120))
  .header("Content-Type", "application/json")
  .body(()).expect("Failed to create request. Maybe the link isn't correct.")
  .send();
  if web_request.is_err() {
    return Err(());
  }
  let webpage_as_string = match web_request.as_ref().unwrap().status() {
    StatusCode::OK => {
      match web_request.unwrap().text() {
        Ok(res) => res,
        Err(_e) => {
          return Err(())
        }
      }
    },
    _ => return Err(())
  };
  match serde_json::from_str::<MediaResponse>(&webpage_as_string) {
    Ok(serialized) => Ok(serialized),
    Err(_e) => {
      println!("{}", _e);
      return Err(())
    }
  }
}
