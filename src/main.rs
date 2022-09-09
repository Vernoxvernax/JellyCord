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
pub struct ConfigFile {
    pub discord_token: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Instance {
    Active_Channel: u8,
    Channel_ID: u64,
    Domain: String,
    Token: String,
    UserID: String,
    TRC: i64
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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Library {
    Name: String,
    Id: String
}


#[derive(serde::Serialize, sqlx::FromRow, Deserialize, Debug, Clone)]
struct LibraryItem {
    b4e66a8b5c46f4b0f22a055f24aa: String
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
                        let serialized = get_page(format!("{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series", server.Domain, server.UserID, server.Token)).unwrap();
                        if serialized.TotalRecordCount != server.TRC.unwrap() {
                            println!("New items found!");
                            let db_fetch: Vec<Result<String, sqlx::Error>> = sqlx::query(format!("SELECT {} FROM LIBRARY", &server.UserID.as_str()[2..server.UserID.len() - 2]).as_str())
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
                                    sqlx::query(format!("INSERT INTO LIBRARY ({}) VALUES(\"{}\")", &server.UserID[2..server.UserID.len() - 2], &x.Id).as_str()).execute(&database)
                                    .await.expect("insert error");
                                };
                                let new_trc = serialized.TotalRecordCount + new_items.len() as i64;
                                sqlx::query!("UPDATE FRONT SET TRC = ? WHERE UserID=?", new_trc, server.UserID).execute(&database).await.expect("Couldn't update database.");
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
        .configure(|c| c.prefix("~")) 
        .group(&GENERAL_GROUP);
        let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
        let mut client = Client::builder(serialized.discord_token, intents)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
            // database
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
    let thread = msg.channel_id.create_public_thread(ctx, msg.id, |t| t.name("JellyCord - Initialize")).await?;
    thread.say(ctx, "Please enter your jellyfin address.\nYou can stop this process by typing \"quit\"").await?;
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
            if sqlx::query!(
                "SELECT Domain FROM FRONT WHERE Domain=?",
                end
            ).fetch_one(&database).await.is_ok() {
                msg.reply(&ctx, "This jellyfin server has already been added.\nUse \"dump\" to recreate a connection.").await.unwrap();
                break
            } else {
                Ok(end.to_string())
            }
        };
        let domain = domain.unwrap();
        user_reply.reply(&ctx, "Please create an api token and enter it here.").await?;
        let api_token = &msg.author.await_reply(&ctx).await.unwrap();
        api_token.delete(&ctx).await?;
        thread.say(&ctx, "Received the token.\nNow enter the username which you would like to receive the notifications from.").await?;
        let requ = reqwest::get(format!("{}/Users?api_key={}", &domain, &api_token.content)).await?;
        let serialized: Vec<MinimalList> = serde_json::from_str(&requ.text().await.unwrap()).unwrap();
        loop {
            let username = &msg.author.await_reply(&ctx).await.unwrap().content;
            let mut user_id: Option<String> = None;
            for user in serialized.clone().into_iter() {
                if user.Name.to_lowercase() == username.to_lowercase().trim() {
                    user_id = Some(user.Id)
                }
            };
            if user_id.is_none() {
                thread.say(&ctx, "Username could not be found, please enter a different one.").await?;
            } else {
                thread.say(&ctx, "Username has been found and added to the configuration.").await?;
                let requ = reqwest::get(format!("{}/Users/{}/Items?api_key={}&Recursive=true&IncludeItemTypes=Movie,Series", &domain, &user_id.as_ref().unwrap(), &api_token.content)).await?;
                let serialized: MediaResponse = serde_json::from_str(&requ.text().await.unwrap()).unwrap();
                if sqlx::query(format!("SELECT {} FROM LIBRARY", &user_id.as_ref().unwrap()[2..user_id.as_ref().unwrap().len() - 2]).as_str()).fetch_one(&database).await.is_ok() {
                    dbg!(format!("ALTER TABLE LIBRARY RENAME COLUMN {} TO {}_{}", &user_id.as_ref().unwrap()[2..user_id.as_ref().unwrap().len() - 2], &user_id.as_ref().unwrap()[2..user_id.as_ref().unwrap().len() - 2], chrono::offset::Utc::now().timestamp()));
                    sqlx::query(format!("ALTER TABLE LIBRARY RENAME COLUMN {} TO {}_{}", &user_id.as_ref().unwrap()[2..user_id.as_ref().unwrap().len() - 2], &user_id.as_ref().unwrap()[2..user_id.as_ref().unwrap().len() - 2], chrono::offset::Utc::now().timestamp()).as_str()).execute(&database).await.expect("couldn't rename database");
                };
                sqlx::query(
                    format!("ALTER TABLE LIBRARY ADD {} VARCHAR(30)", &user_id.as_ref().unwrap()[2..user_id.as_ref().unwrap().len() - 2]).as_str()).execute(&database)
                    .await.expect("insert error");
                let mut id_as_value: String = String::new();
                for item in serialized.Items {
                    id_as_value.push_str(format!("(\"{}\"),", item.Id).as_str());
                };
                id_as_value.pop();
                sqlx::query(
                    format!("INSERT INTO LIBRARY ({}) VALUES {}", &user_id.as_ref().unwrap()[2..user_id.as_ref().unwrap().len() - 2], &id_as_value).as_str()).execute(&database)
                    .await.expect("insert error");
                let add = Instance {
                    Active_Channel: 1,
                    Channel_ID: msg.channel_id.0,
                    Domain: domain,
                    Token: api_token.content.clone(),
                    UserID: user_id.unwrap(),
                    TRC: serialized.TotalRecordCount.clone()
                };
                let channel_id = add.Channel_ID as i64; 
                sqlx::query!(
                    "INSERT INTO FRONT (Active_Channel, Channel_ID, Domain, Token, UserID, TRC) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    add.Active_Channel, channel_id, add.Domain, add.Token, add.UserID, add.TRC).execute(&database)
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



fn get_page(url: String) -> Result<MediaResponse, isahc::Error> {
    let mut response = Request::get(url).timeout(Duration::from_secs(10))
    .header("Content-Type", "application/json")
    .body(())?.send()?;
    let result = match response.status() {
        StatusCode::OK => {
            let fdsfd: MediaResponse = serde_json::from_str(&response.text().unwrap()).unwrap();
            fdsfd
        }
        _ => panic!("{} your server is missing some api Domains, i think", response.status())
    };
    Ok(
        result
    )
}
