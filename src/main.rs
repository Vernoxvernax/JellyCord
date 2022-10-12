#![allow(non_snake_case)]
use config::{Config, File};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;
use http::StatusCode;
use std::env;
use isahc::Request;
use chrono;
use isahc::prelude::*;
use serde_derive::{Serialize, Deserialize};
use serenity::async_trait;
use std::time::Duration;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::prelude::*;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};


#[derive(Deserialize)]
struct ConfigFile {
    discord_token: String,
    command_prefix: Option<char>
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Instance {
    active_channel: u8,
    channel_id: u64,
    domain: String,
    token: String,
    user_id: String,
    trc: i64
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MinimalList {
    Name: String,
    Id: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MediaResponse {
    Items: Vec<Library>,
    TotalRecordCount: i64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Library {
    Name: String,
    Id: String
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
        // loop {};
        if !self.is_loop_running.load(Ordering::Relaxed) {
            tokio::spawn(async move {
                loop {
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
                    .fetch_all(&database).await;
                    for server in db.unwrap() {
                        let timed_response_obj = get_page(format!("{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series", server.Domain, server.UserID, server.Token));
                        if timed_response_obj.is_err() {
                            eprintln!("Failed to connect to the server.");
                            continue
                        } else {
                            if timed_response_obj.is_err() {
                                let error_message = ChannelId(server.Channel_ID.unwrap() as u64)
                                        .send_message(&ctx, |m| {
                                            m.content(format!("Your mediaserver could not be reached. If you believe that the error has been fixed, you can reactivate this channel by typing \"~unpause\"."))
                                }).await;
                                match error_message {
                                    Ok(_) => {
                                        sqlx::query!(
                                            "UPDATE FRONT SET Active_Channel = 0 WHERE UserID=?",
                                            server.UserID).execute(&database)
                                            .await.expect("pause error");
                                    },
                                    Err(_) => {
                                        sqlx::query!(
                                            "DELETE FROM FRONT WHERE UserID=?",
                                            server.UserID).execute(&database)
                                        .await.expect("dump error");
                                    }
                                };
                                continue
                            };
                            let serialized = timed_response_obj.unwrap();
                            if serialized.TotalRecordCount != server.TRC.unwrap() {
                                println!("New items found!");
                                let db_fetch: Vec<Result<String, sqlx::Error>> = sqlx::query(format!("SELECT {:?} FROM LIBRARY", &server.UserID).as_str())
                                .map(|row: SqliteRow| row.try_get(0))
                                .fetch_all(&database).await.expect("Failed while searching database.");
                                let mut db_string = String::new();
                                for items in db_fetch {
                                    db_string.push_str(&(items.unwrap() + " "));
                                };
                                let mut new_items: Vec<Library> = [].to_vec();
                                for item in serialized.Items {
                                    if ! db_string.contains(&item.Id) {
                                        new_items.append(&mut [item].to_vec());
                                    }
                                };
                                if new_items.is_empty() {
                                    continue
                                } else {
                                    for x in new_items.clone() {
                                        let image = format!("{}/Items/{}/Images/Primary?api_key={}&Quality=100", server.Domain, x.Id, server.Token);
                                        let res = ChannelId(server.Channel_ID.unwrap() as u64)
                                            .send_message(&ctx, |m| {
                                                m.embed(|e| {
                                                    e.title(x.Name)
                                                    .image(image)
                                            })
                                        }).await;
                                        if let Err(why) = res {
                                            eprintln!("Error sending message: {:?}", why);
                                        };
                                        sqlx::query(format!("INSERT INTO LIBRARY ({:?}) VALUES (\"{}\")", &server.UserID, &x.Id).as_str()).execute(&database)
                                        .await.expect("insert error");
                                    };
                                    sqlx::query!("UPDATE FRONT SET TRC = ? WHERE UserID=?", serialized.TotalRecordCount, server.UserID).execute(&database).await.expect("Couldn't update database.");
                                }
                            }
                        }
                    };
                    database.close().await;
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
        let users: Result<Vec<MinimalList>, String> = match users_response {
            Ok(mut ok) => {
                let serde_attempt = serde_json::from_str::<Vec<MinimalList>>(&ok.text().unwrap());
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
        thread.say(&ctx, "Received the token.\nNow enter the username which you would like to receive the notifications for.").await?;
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
                thread.say(&ctx, "Username has been found and added to the configuration.").await?;
                let timed_response = get_page(format!("{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series", &domain, &user_id_raw.unwrap(), &api_token.content));
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
                    channel_id: channel_id as u64,
                    domain,
                    token: api_token.content.clone(),
                    user_id: user_id.to_string(),
                    trc: serialized.TotalRecordCount.clone()
                };
                let channel_id = add.channel_id as i64;
                sqlx::query!(
                    "INSERT INTO FRONT (Active_Channel, Channel_ID, Domain, Token, UserID, TRC) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    add.active_channel, channel_id, add.domain, add.token, add.user_id, add.trc).execute(&database)
                    .await.expect("insert error");
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


fn get_page(url: String) -> Result<MediaResponse, error::Error> {
    let mut response = Request::get(url).timeout(Duration::from_secs(10))
    .header("Content-Type", "application/json")
    .body(()).expect("Failed to create request.").send().expect("Sending request");
    let result = match response.status() {
        StatusCode::OK => {
            let fdsfd: MediaResponse = serde_json::from_str(&response.text().unwrap()).unwrap();
            Ok(fdsfd)
        },
        _ => {
            Err(response.status().to_string())
        }
    };
    Ok(
    result.unwrap()
    )
}
