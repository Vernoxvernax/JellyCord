#![allow(non_snake_case)]
use config::{Config, File};
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serenity::all::{
  ActivityData, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
  CreateMessage,
};
use serenity::async_trait;
use serenity::model::id::{ChannelId, GuildId};
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::env;
use std::path::Path;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

mod commands;
mod database;
use database::*;

#[derive(Deserialize)]
struct ConfigFile {
  discord_token: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Instance {
  pub active_channel: i64,
  pub channel_id: i64,
  pub domain: String,
  pub token: String,
  pub user_id: String,
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
  Special,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Item {
  Name: String,
  Id: String,
  IndexNumber: Option<u32>,
  ParentIndexNumber: Option<u32>,
  IndexNumberEnd: Option<u32>,
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
  EndDate: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MediaStream {
  Type: String,
  Language: Option<String>,
  Height: Option<u32>,
  IsInterlaced: bool,
}

trait LibraryTools {
  fn contains(&self, s: String) -> bool;
}

impl LibraryTools for Vec<Vec<Item>> {
  fn contains(&self, s: String) -> bool {
    for itemlist in self.iter() {
      for item in itemlist {
        if item.Id == s.clone() {
          return true;
        }
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
      Self::Special => String::from("Special"),
    }
  }
}

impl ToString for Item {
  fn to_string(&self) -> String {
    let time = if let (Some(start), Some(end)) = (self.PremiereDate.clone(), self.EndDate.clone()) {
      if start[0..4] == end[0..4] {
        format!("({})", &start[0..4])
      } else {
        format!("({}-{})", &start[0..4], &end[0..4])
      }
    } else if self.Status == Some(String::from("Continuing")) {
      format!(
        "({}-)",
        &self.PremiereDate.clone().unwrap_or(String::from("????"))[0..4]
      )
    } else if let Some(premiere_date) = &self.PremiereDate {
      format!("({})", &premiere_date[0..4])
    } else if let Some(production_year) = &self.ProductionYear {
      format!("({})", production_year)
    } else {
      "(???)".to_string()
    };
    let mut name: String;
    match self.Type {
      Type::Season | Type::Episode => name = self.SeriesName.clone().unwrap_or(String::from("???")),
      _ => name = self.Name.clone(),
    }
    if name.contains('(') {
      let re = Regex::new(r" \(\d{4}\)").unwrap();
      name = re.replace_all(&name, "").to_string();
    }

    match self.Type {
      Type::Movie | Type::Series => {
        format!("{} {}", name, time)
      },
      Type::Season => {
        format!("{} {} - {}", name, time, self.Name.clone())
      },
      Type::Episode => match self.IndexNumberEnd {
        Some(indexend) => {
          format!(
            "{} {} - S{:02}E{:02}-{:02} - {}",
            name,
            time,
            self.ParentIndexNumber.unwrap_or(0),
            self.IndexNumber.unwrap_or(0),
            indexend,
            self.Name
          )
        },
        None => {
          format!(
            "{} {} - S{:02}E{:02} - {}",
            name,
            time,
            self.ParentIndexNumber.unwrap_or(0),
            self.IndexNumber.unwrap_or(0),
            self.Name
          )
        },
      },
      _ => format!("{} {} (unknown media type)", self.Name, time),
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
        _ => "Not implemented >~< - (Contact: @DepriSheep)".to_string(),
      };

      let data = CreateInteractionResponseMessage::new().content(content);
      let builder = CreateInteractionResponse::Message(data);
      if let Err(why) = command.create_response(&ctx.http, builder).await {
        println!("Cannot respond to slash command: {}", why);
      }
    }
  }

  async fn ready(&self, ctx: Context, ready: Ready) {
    Command::create_global_command(&ctx.http, commands::help::register())
      .await
      .unwrap();
    Command::create_global_command(&ctx.http, commands::init::register())
      .await
      .unwrap();
    Command::create_global_command(&ctx.http, commands::pause::register())
      .await
      .unwrap();
    Command::create_global_command(&ctx.http, commands::reset::register())
      .await
      .unwrap();
    Command::create_global_command(&ctx.http, commands::ping::register())
      .await
      .unwrap();

    println!("{} is connected!", ready.user.name);
    ctx.set_activity(Some(ActivityData::watching("the internet.")));
  }

  async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
    println!("Cache built successfully!");
    if !self.is_loop_running.load(Ordering::Relaxed) {
      tokio::spawn(async move {
        'main: loop {
          let front_db = get_front_database().await;
          for server in front_db {
            let timed_response_obj = get_serialized_page(format!(
              "{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series,Episode,Season,Special&Fields=MediaStreams&collapseBoxSetItems=False",
              server.domain, server.user_id, server.token
            )).await;
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
                  )
                  .await
                  .expect("Couldn't connect to database");

                let mut id_as_value: String = String::new();
                for item in serialized_server.clone().Items {
                  id_as_value.push_str(format!("(\"{}\"),", item.Id).as_str());
                }
                id_as_value.pop();

                sqlx::query(
                  format!(
                    "INSERT INTO LIBRARY ({:?}) VALUES {}",
                    &server.user_id, &id_as_value
                  )
                  .as_str(),
                )
                .execute(&database)
                .await
                .expect("insert error");
                database.close().await;
                continue;
              }

              let mut library_stringed: Vec<String> = vec![];
              for item in lib {
                library_stringed.push(item);
              }

              let mut raw_new_items: Vec<Item> = vec![]; // contains all new items
              // new movies or series items; it will eventually get all new items from the for loops later
              // type is a nested list to group episodes of the same season together while keeping the order mostly the same
              let mut new_items: Vec<Vec<Item>> = vec![];
              let mut pre_season_items: Vec<Item> = vec![]; // all new season items
              let mut pre_episode_items: Vec<Item> = vec![]; // all new episode items
              for item in &serialized_server.Items {
                if !library_stringed.contains(&item.Id) {
                  raw_new_items.append(&mut vec![item.clone()]);
                  if item.Type == Type::Movie || item.Type == Type::Series {
                    new_items.push(vec![item.clone()]);
                  } else if item.Type == Type::Season {
                    pre_season_items.append(&mut vec![item.clone()]);
                  } else if item.Type == Type::Episode || item.Type == Type::Special {
                    if item.SeasonId.is_none() {
                      // something's wrong. give jellyfin more time to find metadata to propagate this value.
                      continue 'main;
                    }
                    pre_episode_items.append(&mut vec![item.clone()]);
                  }
                }
              }

              for season in pre_season_items.clone() {
                if !new_items.contains(season.SeriesId.clone().unwrap()) {
                  new_items.push(vec![season.clone()]);
                }
              }

              for episode in pre_episode_items.clone() {
                if !new_items.contains(episode.SeasonId.clone().unwrap())
                  && !new_items.contains(episode.SeriesId.clone().unwrap())
                {
                  let mut inserted = false;
                  for itemlist in new_items.iter_mut() {
                    if itemlist[0].SeasonId == episode.SeasonId {
                      itemlist.push(episode.clone());
                      inserted = true;
                    }
                  }
                  if !inserted {
                    new_items.push(vec![episode.clone()]);
                  }
                }
              }

              pre_episode_items.sort_by(|x, y| {
                x.ParentIndexNumber
                  .unwrap()
                  .cmp(&y.ParentIndexNumber.unwrap())
                  .then(x.IndexNumber.unwrap().cmp(&y.IndexNumber.unwrap()))
              });

              new_items.reverse();

              for itemlist in new_items.iter_mut() {
                if itemlist.len() == 1 {
                  let item = itemlist[0].clone();
                  if let Some(streams) = item.MediaStreams.clone() {
                    if streams.is_empty() {
                      continue;
                    }
                  }

                  if item.Type == Type::Episode
                    || item.Type == Type::Special
                    || item.Type == Type::Movie
                  {
                    let name = item.to_string();
                    let image = format!(
                      "{}/Items/{}/Images/Primary?Quality=100",
                      server.domain,
                      item.clone().SeasonId.unwrap_or(item.clone().Id)
                    );
                    let (resolution, a_languages, s_languages) = if item.MediaStreams.is_some() {
                      let mut height: String = String::new();
                      let mut a_languages: String = String::new();
                      let mut s_languages: String = String::new();
                      let mut scan_type: char = 'p';
                      for x in item.MediaStreams.unwrap() {
                        if x.Type == "Video" {
                          height = x.Height.unwrap().to_string();
                          if x.IsInterlaced {
                            scan_type = 'i';
                          }
                        } else if x.Type == "Audio" {
                          a_languages.push_str(&(x.Language.unwrap_or("?".to_string()) + ", "))
                        } else if x.Type == "Subtitle" {
                          s_languages.push_str(&(x.Language.unwrap_or("?".to_string()) + ", "))
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
                    let runtime: String = if item.RunTimeTicks.is_some() {
                      let time = (item.RunTimeTicks.unwrap() as f64) / 10000000.0;
                      let formated: String = if time > 60.0 {
                        if (time / 60.0) > 60.0 {
                          format!(
                            "{:02}:{:02}:{:02}",
                            ((time / 60.0) / 60.0).trunc(),
                            ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0)
                              .trunc(),
                            (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
                          )
                        } else {
                          format!(
                            "00:{:02}:{:02}",
                            (time / 60.0).trunc(),
                            (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
                          )
                        }
                      } else {
                        format!("00:00:{time:02}")
                      };
                      formated
                    } else {
                      "?".to_string()
                    };

                    let mut fields = Vec::new();
                    fields.push((
                      ":star: — Rating".to_string(),
                      if let Some(rating) = item.CommunityRating {
                        format!("{:.2}", rating)
                      } else {
                        "?".to_string()
                      },
                      true,
                    ));
                    fields.push((
                      ":film_frames: — Runtime".to_string(),
                      runtime.to_string(),
                      true,
                    ));
                    fields.push((
                      ":frame_photo: — Resolution".to_string(),
                      resolution.to_string(),
                      true,
                    ));
                    fields.push((
                      ":loud_sound: — Languages".to_string(),
                      a_languages.to_string(),
                      false,
                    ));

                    if !s_languages.is_empty() {
                      fields.push((
                        ":notepad_spiral: — Languages".to_string(),
                        s_languages.to_string(),
                        false,
                      ));
                    }

                    let mut embed = CreateEmbed::default();
                    for (name, value, inline) in &fields {
                      embed = embed.field(name.clone(), value.clone(), *inline);
                    }

                    let res = ChannelId::new(server.channel_id as u64)
                      .send_message(
                        &ctx,
                        CreateMessage::new()
                          .add_embed(CreateEmbed::new().title(name).image(image))
                          .add_embed(embed),
                      )
                      .await;

                    if let Err(why) = res {
                      eprintln!("Error sending message: {why:?}");
                    } else {
                      let database = sqlx::sqlite::SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect_with(
                          sqlx::sqlite::SqliteConnectOptions::new()
                            .filename("jellycord.sqlite")
                            .create_if_missing(true),
                        )
                        .await
                        .expect("Couldn't connect to database");
                      sqlx::query(
                        format!(
                          "INSERT INTO LIBRARY ({:?}) VALUES (\"{}\")",
                          &server.user_id, &item.Id
                        )
                        .as_str(),
                      )
                      .execute(&database)
                      .await
                      .expect("insert error");
                    }
                  } else if item.Type == Type::Season || item.Type == Type::Series {
                    let mut ids: Vec<String> = vec![item.Id.clone()];
                    let seasons = if item.Type == Type::Series {
                      let mut temp = vec![];
                      for season in pre_season_items.clone() {
                        if season.SeriesId.clone().unwrap() == item.Id {
                          ids.push(season.Id.clone());
                          temp.push(season);
                        }
                      }
                      temp
                    } else {
                      vec![item.clone()]
                    };

                    let mut desc = String::new();
                    let mut a_languages: Vec<String> = vec![];
                    let mut s_languages: Vec<String> = vec![];
                    let mut v_resolutions: Vec<String> = vec![];
                    let mut ratings: Vec<f64> = vec![];
                    let mut total_runtime: u64 = 0;

                    let mut current_start = -1;
                    for season in seasons {
                      for (i, episode) in pre_episode_items.clone().iter().enumerate() {
                        if episode.SeasonId.clone().unwrap() != season.Id {
                          continue;
                        }

                        ids.push(episode.Id.clone());

                        if let Some(mediastreams) = &episode.MediaStreams {
                          for x in mediastreams {
                            if x.Type == "Video" {
                              let resolution: String;
                              let scan_type: char;

                              if x.IsInterlaced {
                                scan_type = 'i';
                              } else {
                                scan_type = 'p';
                              }

                              if let Some(height) = x.Height {
                                resolution = height.to_string() + &scan_type.to_string();
                              } else {
                                resolution = String::from("?") + &scan_type.to_string();
                              }

                              if !v_resolutions.contains(&resolution) {
                                v_resolutions.push(resolution);
                              }
                            } else if x.Type == "Audio" {
                              let lang = x.Language.clone().unwrap_or("?".to_string());
                              if !a_languages.contains(&lang) {
                                a_languages.push(lang);
                              }
                            } else if x.Type == "Subtitle" {
                              let lang = x.Language.clone().unwrap_or("?".to_string());
                              if !s_languages.contains(&lang) {
                                s_languages.push(lang);
                              }
                            }
                          }
                        }

                        if let Some(runtime) = episode.RunTimeTicks {
                          total_runtime += runtime;
                        }

                        if let Some(rating) = episode.CommunityRating {
                          ratings.push(rating);
                        }

                        let index_start = episode.IndexNumber.unwrap() as i32;
                        let index_end = if let Some(end) = episode.IndexNumberEnd {
                          end as i32
                        } else {
                          index_start
                        };
                        let item_name_full = match episode.IndexNumberEnd {
                          Some(indexend) => {
                            format!(
                              "S{:02}E{:02}-{:02}",
                              episode.ParentIndexNumber.unwrap_or(0),
                              episode.IndexNumber.unwrap_or(0),
                              indexend
                            )
                          },
                          None => {
                            format!(
                              "S{:02}E{:02}",
                              episode.ParentIndexNumber.unwrap_or(0),
                              episode.IndexNumber.unwrap_or(0)
                            )
                          },
                        };
                        let item_name_end = match episode.IndexNumberEnd {
                          Some(indexend) => {
                            format!(
                              "S{:02}E{:02}",
                              episode.ParentIndexNumber.unwrap_or(0),
                              indexend
                            )
                          },
                          None => {
                            format!(
                              "S{:02}E{:02}",
                              episode.ParentIndexNumber.unwrap_or(0),
                              episode.IndexNumber.unwrap_or(0)
                            )
                          },
                        };
                        let item_name_start = format!(
                          "S{:02}E{:02}",
                          episode.ParentIndexNumber.unwrap_or(0),
                          episode.IndexNumber.unwrap_or(0)
                        );

                        if pre_episode_items.len() - 1 == i {
                          if current_start == -1 {
                            desc.push_str(&format!("{}", item_name_full));
                          } else {
                            desc.push_str(&format!("-{}", item_name_end));
                          }
                        } else if i == 0 || current_start == -1 {
                          if pre_episode_items[i + 1].IndexNumber.unwrap() as i32 != index_end + 1 {
                            desc.push_str(&format!("{}, ", item_name_full));
                            current_start = -1;
                            continue;
                          } else {
                            desc.push_str(&item_name_start);
                          }
                        } else if pre_episode_items[i + 1].IndexNumber.unwrap() as i32
                          != index_end + 1
                        {
                          desc.push_str(&format!("-{}, ", item_name_end));
                          current_start = -1;
                          continue;
                        }
                        current_start = index_start;
                      }
                    }

                    let image = format!(
                      "{}/Items/{}/Images/Primary?api_key={}&Quality=100",
                      server.domain,
                      item.clone().SeasonId.unwrap_or(item.clone().Id),
                      server.token
                    );
                    let name = item.to_string();

                    let time = (total_runtime as f64) / 10000000.0;
                    let formatted_runtime: String = if time > 60.0 {
                      if (time / 60.0) > 60.0 {
                        format!(
                          "{:02}:{:02}:{:02}",
                          ((time / 60.0) / 60.0).trunc(),
                          ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(),
                          (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
                        )
                      } else {
                        format!(
                          "00:{:02}:{:02}",
                          (time / 60.0).trunc(),
                          (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
                        )
                      }
                    } else {
                      format!("00:00:{time:02}")
                    };

                    let mut fields = Vec::new();
                    fields.push((
                      ":star: — Rating".to_string(),
                      format!("{:.2}", ratings.iter().sum::<f64>() / ratings.len() as f64),
                      true,
                    ));
                    fields.push((
                      ":film_frames: — Runtime".to_string(),
                      formatted_runtime.to_string(),
                      true,
                    ));
                    fields.push((
                      ":frame_photo: — Resolution".to_string(),
                      v_resolutions.join(", "),
                      true,
                    ));
                    fields.push((
                      ":loud_sound: — Languages".to_string(),
                      // 205 is the exact max amount of langs to show (they should all be 3 chars long)
                      a_languages
                        .iter()
                        .take(205)
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                        .join(", "),
                      false,
                    ));

                    if !s_languages.is_empty() {
                      fields.push((
                        ":notepad_spiral: — Languages".to_string(),
                        s_languages
                          .iter()
                          .take(205)
                          .map(|s| s.as_str())
                          .collect::<Vec<&str>>()
                          .join(", "),
                        false,
                      ));
                    }

                    let mut embed = CreateEmbed::default();
                    for (name, value, inline) in &fields {
                      embed = embed.field(name.clone(), value.clone(), *inline);
                    }

                    let res = ChannelId::new(server.channel_id as u64)
                      .send_message(
                        &ctx,
                        CreateMessage::new()
                          .add_embed(
                            CreateEmbed::new()
                              .title(name)
                              .image(image)
                              .description(desc),
                          )
                          .add_embed(embed),
                      )
                      .await;

                    if let Err(why) = res {
                      eprintln!("Error sending message: {why:?}");
                    } else {
                      for x in ids {
                        let database = sqlx::sqlite::SqlitePoolOptions::new()
                          .max_connections(5)
                          .connect_with(
                            sqlx::sqlite::SqliteConnectOptions::new()
                              .filename("jellycord.sqlite")
                              .create_if_missing(true),
                          )
                          .await
                          .expect("Couldn't connect to database");
                        sqlx::query(
                          format!(
                            "INSERT INTO LIBRARY ({:?}) VALUES (\"{}\")",
                            &server.user_id, &x
                          )
                          .as_str(),
                        )
                        .execute(&database)
                        .await
                        .expect("insert error");
                      }
                    }
                  }
                } else {
                  let series_id = itemlist[0].SeriesId.clone().unwrap();
                  let mut item: Item = itemlist[0].clone();
                  for x in &serialized_server.Items {
                    if x.Id == series_id {
                      item = x.clone();
                      break;
                    }
                  }
                  if item.Type != Type::Series {
                    eprintln!(
                      "Failed to find a Series object that belongs to \"{}\"",
                      item.Id
                    );
                    continue;
                  }

                  itemlist.sort_by_key(|i| i.IndexNumber.unwrap());
                  let mut desc = String::new();
                  let mut a_languages: Vec<String> = vec![];
                  let mut s_languages: Vec<String> = vec![];
                  let mut v_resolutions: Vec<String> = vec![];
                  let mut ratings: Vec<f64> = vec![];
                  let mut total_runtime: u64 = 0;

                  let mut current_start: i32 = -1;
                  for (i, episode) in itemlist.iter().enumerate() {
                    if episode.MediaStreams.is_some() {
                      for x in episode.MediaStreams.clone().unwrap() {
                        if x.Type == "Video" {
                          let resolution: String;
                          let scan_type: char;

                          if x.IsInterlaced {
                            scan_type = 'i';
                          } else {
                            scan_type = 'p';
                          }

                          if let Some(height) = x.Height {
                            resolution = height.to_string() + &scan_type.to_string();
                          } else {
                            resolution = String::from("?") + &scan_type.to_string();
                          }

                          if !v_resolutions.contains(&resolution) {
                            v_resolutions.push(resolution);
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
                    }

                    if let Some(rating) = episode.CommunityRating {
                      ratings.push(rating);
                    }

                    if let Some(runtime) = episode.RunTimeTicks {
                      total_runtime += runtime;
                    }

                    let index_start = episode.IndexNumber.unwrap() as i32;
                    let index_end = if let Some(end) = episode.IndexNumberEnd {
                      end as i32
                    } else {
                      index_start
                    };
                    let item_name_full = match episode.IndexNumberEnd {
                      Some(indexend) => {
                        format!(
                          "S{:02}E{:02}-{:02}",
                          episode.ParentIndexNumber.unwrap_or(0),
                          episode.IndexNumber.unwrap_or(0),
                          indexend
                        )
                      },
                      None => {
                        format!(
                          "S{:02}E{:02}",
                          episode.ParentIndexNumber.unwrap_or(0),
                          episode.IndexNumber.unwrap_or(0)
                        )
                      },
                    };
                    let item_name_end = match episode.IndexNumberEnd {
                      Some(indexend) => {
                        format!(
                          "S{:02}E{:02}",
                          episode.ParentIndexNumber.unwrap_or(0),
                          indexend
                        )
                      },
                      None => {
                        format!(
                          "S{:02}E{:02}",
                          episode.ParentIndexNumber.unwrap_or(0),
                          episode.IndexNumber.unwrap_or(0)
                        )
                      },
                    };
                    let item_name_start = format!(
                      "S{:02}E{:02}",
                      episode.ParentIndexNumber.unwrap_or(0),
                      episode.IndexNumber.unwrap_or(0)
                    );

                    if itemlist.len() - 1 == i {
                      if current_start == -1 {
                        desc.push_str(&format!("{}", item_name_full));
                      } else {
                        desc.push_str(&format!("-{}", item_name_end));
                      }
                    } else if i == 0 || current_start == -1 {
                      if itemlist[i + 1].IndexNumber.unwrap() as i32 != index_end + 1 {
                        desc.push_str(&format!("{}, ", item_name_full));
                        current_start = -1;
                        continue;
                      } else {
                        desc.push_str(&item_name_start);
                      }
                    } else if itemlist[i + 1].IndexNumber.unwrap() as i32 != index_end + 1 {
                      desc.push_str(&format!("-{}, ", item_name_end));
                      current_start = -1;
                      continue;
                    }
                    current_start = index_start;
                  }

                  let image = format!(
                    "{}/Items/{}/Images/Primary?Quality=100",
                    server.domain,
                    item.clone().SeasonId.unwrap_or(item.clone().Id)
                  );

                  let time = (total_runtime as f64) / 10000000.0;
                  let formatted_runtime: String = if time > 60.0 {
                    if (time / 60.0) > 60.0 {
                      format!(
                        "{:02}:{:02}:{:02}",
                        ((time / 60.0) / 60.0).trunc(),
                        ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(),
                        (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
                      )
                    } else {
                      format!(
                        "00:{:02}:{:02}",
                        (time / 60.0).trunc(),
                        (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
                      )
                    }
                  } else {
                    format!("00:00:{time:02}")
                  };

                  let mut fields = Vec::new();
                  fields.push((
                    ":star: — Rating".to_string(),
                    format!("{:.2}", ratings.iter().sum::<f64>() / ratings.len() as f64),
                    true,
                  ));
                  fields.push((
                    ":film_frames: — Runtime".to_string(),
                    formatted_runtime.to_string(),
                    true,
                  ));
                  fields.push((
                    ":frame_photo: — Resolution".to_string(),
                    v_resolutions.join(", "),
                    true,
                  ));
                  fields.push((
                    ":loud_sound: — Languages".to_string(),
                    // 205 is the exact max amount of langs to show (they should all be 3 chars long)
                    a_languages
                      .iter()
                      .take(205)
                      .map(|s| s.as_str())
                      .collect::<Vec<&str>>()
                      .join(", "),
                    false,
                  ));

                  if !s_languages.is_empty() {
                    fields.push((
                      ":notepad_spiral: — Languages".to_string(),
                      s_languages
                        .iter()
                        .take(205)
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>()
                        .join(", "),
                      false,
                    ));
                  }

                  let mut embed = CreateEmbed::default();
                  for (name, value, inline) in &fields {
                    embed = embed.field(name.clone(), value.clone(), *inline);
                  }

                  let res = ChannelId::new(server.channel_id as u64)
                    .send_message(
                      &ctx,
                      CreateMessage::new()
                        .add_embed(
                          CreateEmbed::new()
                            .title(item.to_string())
                            .description(desc)
                            .image(image),
                        )
                        .add_embed(embed),
                    )
                    .await;

                  if let Err(why) = res {
                    eprintln!("Error sending message: {why:?}");
                  } else {
                    for x in itemlist {
                      let database = sqlx::sqlite::SqlitePoolOptions::new()
                        .max_connections(5)
                        .connect_with(
                          sqlx::sqlite::SqliteConnectOptions::new()
                            .filename("jellycord.sqlite")
                            .create_if_missing(true),
                        )
                        .await
                        .expect("Couldn't connect to database");
                      sqlx::query(
                        format!(
                          "INSERT INTO LIBRARY ({:?}) VALUES (\"{}\")",
                          &server.user_id, &x.Id
                        )
                        .as_str(),
                      )
                      .execute(&database)
                      .await
                      .expect("insert error");
                    }
                  }
                }
              }
            } else {
              eprintln!("Failed to connect to the server. {}", server.domain);
              tokio::time::sleep(Duration::from_secs(5)).await; // Don't ddos the dns server.
              continue;
            }
          }
          tokio::time::sleep(Duration::from_secs(300)).await;
        }
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
  sqlx::migrate!("./migrations")
    .run(&database)
    .await
    .expect("Couldn't run database migrations");
  database.close().await;
  if env::var("SETUP") == Ok("1".to_string()) {
    exit(0x100);
  };
  let settings_file_raw = Config::builder()
    .add_source(File::from(Path::new(&"./jellycord.yaml".to_string())))
    .build()
    .unwrap();
  let serialized = settings_file_raw
    .try_deserialize::<ConfigFile>()
    .expect("Reading config file.");
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

async fn get_serialized_page(url: String) -> Result<MediaResponse, ()> {
  let client = reqwest::Client::new();
  let web_request = client
    .get(url)
    .timeout(Duration::from_secs(120))
    .header("Content-Type", "application/json")
    .send()
    .await;

  let response = if let Err(res) = web_request {
    eprintln!("Error: {}", res.to_string().as_str());
    return Err(());
  } else {
    web_request.unwrap()
  };

  let webpage_as_string = match response.text().await {
    Ok(text) => text,
    _ => {
      return Err(());
    },
  };

  match serde_json::from_str::<MediaResponse>(&webpage_as_string) {
    Ok(serialized) => Ok(serialized),
    Err(e) => {
      eprintln!("Error: {}", e);
      Err(())
    },
  }
}
