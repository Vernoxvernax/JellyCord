#![allow(non_snake_case)]
use std::env;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;
use std::time::Duration;
use config::{Config, File};
use isahc::Request;
use isahc::prelude::*;
use isahc::http::StatusCode;
use regex::Regex;
use serde_derive::{Serialize, Deserialize};
use serenity::async_trait;
use serenity::all::{ActivityData, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage};
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;
use serenity::model::prelude::*;

mod commands;
mod database;
use database::*;

#[derive(Deserialize)]
struct ConfigFile {
  discord_token: String
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
  Items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum Type {
  Movie,
  Series,
  Season,
  Episode,
  Special
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Item {
  Name: String,
  Id: String,
  IndexNumber: Option<u32>,
  ParentIndexNumber: Option<u32>,
  Type: Type,
  SeriesName: Option<String>,
  SeriesId: Option<String>,
  SeasonName: Option<String>,
  SeasonId: Option<String>,
  MediaStreams: Option<Vec<MediaStream>>,
  CommunityRating: Option<f64>,
  RunTimeTicks: Option<u64>,
  PremiereDate: Option<String>,
  pub ProductionYear: Option<u32>,
  Status: Option<String>,
  EndDate: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MediaStream {
  Type: String,
  Language: Option<String>,
  Height: Option<u32>,
  IsInterlaced: bool
}

trait LibraryTools {
  fn contains(&self, s: String) -> bool;
}

impl LibraryTools for Vec<Item> {
  fn contains(&self, s: String) -> bool {
    for item in self.iter() {
      if item.Id == s.clone() {
        return true;
      }
    }
    false
  }
}

impl ToString for Type {
  fn to_string(&self) -> String {
    match self {
      Self::Movie => String::from("Movie"),
      Self::Episode => String::from("Episode"),
      Self::Season => String::from("Season"),
      Self::Series => String::from("Series"),
      Self::Special => String::from("Special")
    }
  }
}

impl ToString for Item {
  fn to_string(&self) -> String {
    let time = if let (Some(start), Some(end)) = (self.PremiereDate.clone(), self.EndDate.clone()) {
      format!("({}-{})", &start[0..4], &end[0..4])
    } else if self.Status == Some(String::from("Continuing")) {
      format!("({}-)", &self.PremiereDate.clone().unwrap_or(String::from("????"))[0..4])
    } else if let Some(premiere_date) = &self.PremiereDate {
      format!("({})", &premiere_date[0..4])
    } else if let Some(production_year) = &self.ProductionYear {
      format!("({})", production_year)
    } else {
      "(???)".to_string()
    };
    let mut name: String;
    match self.Type.to_string().as_str() {
      "Season" | "Episode" => name = self.SeriesName.clone().unwrap_or(String::from("???")),
      _ => name = self.Name.clone()
    }
    if name.contains('(') {
      let re = Regex::new(r" \(\d{4}\)").unwrap();
      name = re.replace_all(&name, "").to_string();
    }
    
    match self.Type.to_string().as_str() {
      "Movie" | "Series" => {
        format!("{} {}", name, time)
      },
      "Season" => {
        format!("{} {} - {}",
          name,
          time,
          self.Name.clone()
        )
      },
      "Episode" => {
        format!("{} {} - S{:02}E{:02} - {}",
          name,
          time,
          self.ParentIndexNumber.unwrap_or(0),
          self.IndexNumber.unwrap_or(0),
          self.Name
        )
      },
      _ => format!("{} {} (unknown media type)", self.Name, time)
    }
  }
}

struct Handler {
  is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
  async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
    if let Interaction::Command(command) = interaction {
      let content = match command.data.name.as_str() {
        "help" => commands::help::run(&command.data.options).await,
        "init" => commands::init::run(&command.data.options).await,
        "reset" => commands::reset::run(&command.data.options).await,
        "pause" => commands::pause::run(&command.data.options).await,
        "ping" => commands::ping::run(&command.data.options).await,
        _ => "Not implemented >~< - (Contact: @DepriSheep)".to_string()
      };

      let data = CreateInteractionResponseMessage::new().content(content);
      let builder = CreateInteractionResponse::Message(data);
      if let Err(why) = command.create_response(&ctx.http, builder).await {
        println!("Cannot respond to slash command: {}", why);
      }
    }
  }

  async fn ready(&self, ctx: Context, ready: Ready) {
    Command::create_global_command(&ctx.http, commands::help::register()).await.unwrap();
    Command::create_global_command(&ctx.http, commands::init::register()).await.unwrap();
    Command::create_global_command(&ctx.http, commands::pause::register()).await.unwrap();
    Command::create_global_command(&ctx.http, commands::reset::register()).await.unwrap();
    Command::create_global_command(&ctx.http, commands::ping::register()).await.unwrap();

    println!("{} is connected!", ready.user.name);
    ctx.set_activity(Some(ActivityData::watching("the internet.")));
  }

  async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
    println!("Cache built successfully!");
    if !self.is_loop_running.load(Ordering::Relaxed) {
      tokio::spawn(async move {
        loop {
          let front_db = get_front_database().await;
          for server in front_db {
            let timed_response_obj = get_serialized_page(
              format!("{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series,Episode,Season,Special&Fields=MediaStreams&collapseBoxSetItems=False",
              server.domain, server.user_id, server.token)
            );
            if let Ok(serialized_server) = timed_response_obj {
              let lib = get_library_by_user(server.clone().user_id).await;
              
              // Fill the library if it's empty
              // There is a problem with situations where the library is empty upon creating
              // and then gets a new entry, but it's absolutely necessary. See `commands/init.rs`
              if lib.is_empty() {
                let database = sqlx::sqlite::SqlitePoolOptions::new()
                  .max_connections(5)
                  .connect_with(
                  sqlx::sqlite::SqliteConnectOptions::new()
                  .filename("jellycord.sqlite")
                  .create_if_missing(true),
                ).await
                .expect("Couldn't connect to database");

                let mut id_as_value: String = String::new();
                for item in serialized_server.clone().Items {
                  if &item == serialized_server.Items.last().unwrap() {
                    id_as_value.push_str(format!("(\"{}\")", item.Id).as_str());
                  } else {
                    id_as_value.push_str(format!("(\"{}\"),", item.Id).as_str());
                  }
                };
  
                sqlx::query(format!("INSERT INTO LIBRARY ({:?}) VALUES {}", &server.user_id, &id_as_value).as_str()).execute(&database)
                .await.expect("insert error");
                database.close().await;
                continue;
              }

              let mut library_stringed: Vec<String> = vec![];
              for item in lib {
                library_stringed.push(item);
              }

              let mut raw_new_items: Vec<Item> = vec![];
              let mut new_items: Vec<Item> = vec![];
              let mut pre_season_items: Vec<Item> = vec![];
              let mut pre_episode_items: Vec<Item> = vec![];
              for item in serialized_server.Items {
                if ! library_stringed.contains(&item.Id) {
                  raw_new_items.append(&mut vec![item.clone()]);
                  if item.Type == Type::Movie || item.Type == Type::Series {
                    new_items.append(&mut vec![item.clone()]);
                  } else if item.Type == Type::Season {
                    pre_season_items.append(&mut vec![item.clone()]);
                  } else if item.Type == Type::Episode || item.Type == Type::Special {
                    pre_episode_items.append(&mut vec![item.clone()]);
                  }
                }
              }

              for season in pre_season_items.clone() {
                if ! new_items.contains(season.SeriesId.clone().unwrap()) {
                  new_items.append(&mut vec![season.clone()]);
                }
              }

              for episode in pre_episode_items.clone() {
                if episode.SeasonId.clone().is_none() {
                  break;
                }
                if ! new_items.contains(episode.SeasonId.clone().unwrap()) &&
                ! new_items.contains(episode.SeriesId.clone().unwrap()) {
                  new_items.append(&mut vec![episode.clone()]);
                }
              }

              for x in new_items.clone() {
                if let Some(streams) = x.MediaStreams.clone() {
                  if streams.is_empty() {
                    continue;
                  }
                }

                if x.Type == Type::Episode || x.Type == Type::Special || x.Type == Type::Movie {
                  let name = x.to_string();
                  let image = format!("{}/Items/{}/Images/Primary?api_key={}&Quality=100", server.domain, x.clone().SeasonId.unwrap_or(x.clone().Id), server.token);
                  let (resolution, a_languages, s_languages) = if x.MediaStreams.is_some() {
                    let mut height: String = String::new();
                    let mut a_languages: String = String::new();
                    let mut s_languages: String = String::new();
                    let mut scan_type: char = 'p';
                    for x in x.MediaStreams.unwrap() {
                      if x.Type == "Video" {
                        height = x.Height.unwrap().to_string();
                        if x.IsInterlaced {
                          scan_type = 'i';
                        }
                      } else if x.Type == "Audio" {
                        a_languages.push_str(&(x.Language.unwrap_or("?".to_string())+", "))
                      } else if x.Type == "Subtitle" {
                        s_languages.push_str(&(x.Language.unwrap_or("?".to_string())+", "))
                      }
                    }
                    if height.is_empty() {
                      height = "?".to_string()
                    }
                    if a_languages.is_empty() {
                      a_languages = "?".to_string()
                    } else if a_languages != *"?" {
                      a_languages = a_languages.strip_suffix(", ").unwrap().to_string();
                    }
                    if s_languages != *"?" && !s_languages.is_empty() {
                      s_languages = s_languages.strip_suffix(", ").unwrap().to_string();
                    } else {
                      s_languages = String::new()
                    }
                    height.push(scan_type);
                    (height, a_languages, s_languages)
                  } else {
                    ("?".to_string(), "?".to_string(), String::new())
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
                        format!("00:00:{time:02}")
                    };
                    formated
                  } else {
                    "?".to_string()
                  };

                  let mut fields = Vec::new();
                  fields.push((":star: — Rating".to_string(), if x.CommunityRating.is_some() { x.CommunityRating.unwrap().to_string() } else { "?".to_string() }, true));
                  fields.push((":film_frames: — Runtime".to_string(), runtime.to_string(), true));
                  fields.push((":frame_photo: — Resolution".to_string(), resolution.to_string(), true));
                  fields.push((":loud_sound: — Languages".to_string(), a_languages.to_string(), false));

                  if !s_languages.is_empty() {
                    fields.push((":notepad_spiral: — Languages".to_string(), s_languages.to_string(), false));
                  }

                  let mut embed = CreateEmbed::default();
                  for (name, value, inline) in &fields {
                    embed = embed.field(name.clone(), value.clone(), *inline);
                  }

                  let res = ChannelId::new(server.channel_id as u64)
                  .send_message(&ctx, CreateMessage::new()
                    .add_embed(
                      CreateEmbed::new()
                      .title(name)
                      .image(image)
                    )
                    .add_embed(
                      embed
                    )
                  ).await;

                  if let Err(why) = res {
                    eprintln!("Error sending message: {why:?}");
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
                } else if x.Type == Type::Season || x.Type == Type::Series {
                  let seasons = if x.Type == Type::Series {
                    let mut temp = vec![];
                    for season in pre_season_items.clone() {
                      if season.SeriesId.clone().unwrap() == x.Id {
                        temp.push(season);
                      }
                    }
                    temp
                  } else {
                    vec![x.clone()]
                  };

                  let mut runtimes: Vec<u64> = vec![];
                  let mut ratings: Vec<f64> = vec![];
                  let mut resolutions: Vec<String> = vec![];
                  let mut a_languages: Vec<String> = vec![];
                  let mut s_languages: Vec<String> = vec![];
                  for season in seasons {
                    for episode in pre_episode_items.clone() {
                      if episode.SeasonId.unwrap() == season.Id {
                        if let Some(runtime) = episode.RunTimeTicks {
                          runtimes.push(runtime);
                        }
                        if let Some(rating) = episode.CommunityRating {
                          if rating != 0.0 {
                            ratings.push(rating);
                          }
                        }
                        if let Some(mediastreams) = episode.MediaStreams {
                          let mut height = String::new();
                          let mut scan_type: char = 'p';
                          for x in mediastreams {
                            if x.Type == "Video" {
                              height = x.Height.unwrap().to_string();
                              if x.IsInterlaced {
                                scan_type = 'i';
                              }
                            } else if x.Type == "Audio" {
                              let lang = x.Language.unwrap_or("?".to_string());
                              if !a_languages.contains(&lang) {
                                a_languages.push(lang);
                              }
                            } else if x.Type == "Subtitle" {
                              let lang = x.Language.unwrap_or("?".to_string());
                              if !s_languages.contains(&lang) {
                                s_languages.push(lang);
                              }
                            }
                          }
                          if height.is_empty() {
                            height = "?".to_string()
                          }
                          height.push(scan_type);
                          if !resolutions.contains(&height) {
                            resolutions.push(height);
                          }
                        }
                      }
                    }
                  }
                  let image = format!("{}/Items/{}/Images/Primary?api_key={}&Quality=100", server.domain, x.clone().SeasonId.unwrap_or(x.clone().Id), server.token);
                  let name = x.to_string();

                  let avg_rating = if !ratings.is_empty() {
                    let mut total = 0.0;
                    for x in ratings.clone() {
                      total += x;
                    }
                    format!("{}", total / ratings.len() as f64)
                  } else {
                    String::from("?")
                  };

                  let total_runtime = if !runtimes.is_empty() {
                    let mut total = 0;
                    for x in runtimes {
                      total += x;
                    }
                    let time = (total as f64) / 10000000.0;
                    let runtime: String = if time > 60.0 {
                      if (time / 60.0) > 60.0 {
                          format!("{:02}:{:02}:{:02}", ((time / 60.0) / 60.0).trunc(), ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                      } else {
                          format!("00:{:02}:{:02}", (time / 60.0).trunc(), (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc())
                      }
                    } else {
                        format!("00:00:{time:02}")
                    };
                    runtime
                  } else {
                    String::from("?")
                  };

                  let resolution_list = if !resolutions.is_empty() {
                    let mut temp = String::new();
                    for (index, x) in resolutions.iter().enumerate() {
                      if index > 50 {
                        break;
                      }
                      temp.push_str(&(x.to_owned()+", "));
                    }
                    temp.strip_suffix(", ").unwrap().to_string()
                  } else {
                    String::from("?")
                  };

                  let a_languages_list = if !a_languages.is_empty() {
                    let mut temp = String::new();
                    for (index, x) in a_languages.iter().enumerate() {
                      if index > 50 {
                        break;
                      }
                      temp.push_str(&(x.to_owned()+", "));
                    }
                    temp.strip_suffix(", ").unwrap().to_string()
                  } else {
                    String::from("?")
                  };

                  let s_languages_list = if !s_languages.is_empty() {
                    let mut temp = String::new();
                    for (index, x) in s_languages.iter().enumerate() {
                      if index > 50 {
                        break;
                      }
                      temp.push_str(&(x.to_owned()+", "));
                    }
                    temp.strip_suffix(", ").unwrap().to_string()
                  } else {
                    String::from("?")
                  };

                  let mut fields = vec![
                    (":star: — Rating".to_string(), avg_rating, true),
                    (":film_frames: — Runtime".to_string(), total_runtime, true),
                    (":frame_photo: — Resolution".to_string(), resolution_list, true),
                    (":loud_sound: — Languages".to_string(), a_languages_list, false)
                  ];
                  
                  if !s_languages_list.is_empty() {
                    fields.push((":notepad_spiral: — Languages".to_string(), s_languages_list, false));
                  }

                  let mut embed = CreateEmbed::default();
                  for (name, value, inline) in &fields {
                    embed = embed.field(name.clone(), value.clone(), *inline);
                  }

                  let res = ChannelId::new(server.channel_id as u64)
                  .send_message(&ctx, CreateMessage::new()
                    .add_embed(
                      CreateEmbed::new()
                      .title(name)
                      .image(image)
                    )
                    .add_embed(
                      embed
                    )
                  ).await;

                  if let Err(why) = res {
                    eprintln!("Error sending message: {why:?}");
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
                    if x.Type == Type::Series {
                      for item in raw_new_items.clone() {
                        if item.SeriesId == Some(x.Id.clone()) {
                          sqlx::query(format!("INSERT INTO LIBRARY ({:?}) VALUES (\"{}\")", &server.user_id, &item.Id).as_str()).execute(&database)
                          .await.expect("insert error");
                        }
                      }
                    } else if x.Type == Type::Season {
                      for item in raw_new_items.clone() {
                        if item.SeasonId == Some(x.Id.clone()) {
                          sqlx::query(format!("INSERT INTO LIBRARY ({:?}) VALUES (\"{}\")", &server.user_id, &item.Id).as_str()).execute(&database)
                          .await.expect("insert error");
                        }
                      }
                    }
                  }
                }
              };
            } else {
              eprintln!("Failed to connect to the server. {}", server.domain);
              tokio::time::sleep(Duration::from_secs(5)).await; // Don't ddos the dns server.
              continue
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
  loop {
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let client = Client::builder(serialized.discord_token.clone(), intents)
      .event_handler(Handler {
        is_loop_running: AtomicBool::new(false),
      })
    .await;
    if client.is_err() {
      println!("Error creating discord client. Retrying in 60 seconds...");
      tokio::time::sleep(Duration::from_secs(60)).await;
      continue;
    }
    if let Err(why) = client.unwrap().start().await {
      println!("An error occurred while running the client: {why:?}");
    }
  }
}

fn get_serialized_page(url: String) -> Result<MediaResponse, ()> {
  let web_request = Request::get(url).timeout(Duration::from_secs(120))
  .header("Content-Type", "application/json")
  .body(()).expect("Failed to create request. Maybe the link isn't correct.")
  .send();

  let mut response = if let Err(res) = web_request {
    eprintln!("Error: {}", res.to_string().as_str());
    exit(1);
  } else {
    web_request.unwrap()
  };

  let webpage_as_string = match response.status() {
    StatusCode::OK => {
      response.text().unwrap()
    },
    _ => {
      return Err(());
    }
  };

  match serde_json::from_str::<MediaResponse>(&webpage_as_string) {
    Ok(serialized) => Ok(serialized),
    Err(e) => {
      eprintln!("Error: {}", e);
      Err(())
    }
  }
}
