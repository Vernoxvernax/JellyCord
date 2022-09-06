use config::{Config, File};
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;
use http::StatusCode;
use isahc::Request;
use isahc::prelude::*;
use serde_derive::{Serialize, Deserialize};
use serenity::async_trait;
use std::time::Duration;
use serenity::json::Value;
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
    Active_channel: u8,
    Channel_Id: u64,
    Endpoint: String,
    Token: String,
    UserId: String,
    LastId: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MinimalList {
    Name: String,
    Id: String
}

#[derive(Serialize, Deserialize, Clone)]
struct NextupLibrary {
    Name: String,
    Id: String,
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
        ctx.set_activity(Activity::watching("All your media")).await;
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
                    let db = sqlx::query!("SELECT * FROM JellyCord WHERE Active_channel = 1")
                    .fetch_all(&database).await;
                    for server in db.unwrap() {
                        let serialized = get_page(format!("{}/Users/{}/Items/Latest?api_key={}", server.Endpoint, server.UserId, server.Token));
                        let mut serialized = serialized.unwrap();
                        let maybe = serialized.nth(0).unwrap();
                        if server.LastId != maybe.Id {
                            let image = format!("{}/Items/{}/Images/Primary?api_key={}&Quality=100", server.Endpoint, maybe.Id, server.Token);
                            let res = ChannelId(server.Channel_Id.unwrap() as u64)
                                .send_message(&ctx, |m| {
                                    m.embed(|e| {
                                        e.title(maybe.Name)
                                        .image(image)
                                })
                            })
                            .await;
                            if let Err(why) = res {
                                eprintln!("Error sending message: {:?}", why);
                            };
                        };
                        sqlx::query!(
                            "UPDATE JellyCord SET LastId = ? WHERE Endpoint=?",
                            maybe.Id, server.Endpoint).execute(&database)
                            .await.expect("dump error");
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
        let endpoint: Result<String, ()> = if user_reply.content == "quit" {
            thread.delete(ctx).await?;
            break
        } else {
            let end = if user_reply.content.ends_with("/") {
                &user_reply.content[0..&user_reply.content.len() - 1]
            } else {
                &user_reply.content
            };
            if sqlx::query!(
                "SELECT Endpoint FROM JellyCord WHERE Endpoint=?",
                end
            ).fetch_one(&database).await.is_ok() {
                user_reply.reply(&ctx, "This jellyfin server has already been added.\nUse \"dump\" to recreate a connection.").await.unwrap();
                continue
            } else {
                Ok(end.to_string())
            }
        };
        let endpoint = endpoint.unwrap();
        user_reply.reply(&ctx, "Please create an api token and enter it here.").await?;
        let api_token = &msg.author.await_reply(&ctx).await.unwrap();
        api_token.delete(&ctx).await?;
        thread.say(&ctx, "Received the token.\nNow enter the username which you would like to receive the notifications from.").await?;
        let requ = reqwest::get(format!("{}/Users?api_key={}", &endpoint, &api_token.content)).await?;
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
                let requ = reqwest::get(format!("{}/Users/{}/Items/Latest?api_key={}", &endpoint, &user_id.as_ref().unwrap(), &api_token.content)).await?;
                let serialized: Vec<MinimalList> = serde_json::from_str(&requ.text().await.unwrap()).unwrap();
                let add = Instance {
                    Active_channel: 1,
                    Channel_Id: msg.channel_id.0,
                    Endpoint: endpoint,
                    Token: api_token.content.clone(),
                    UserId: user_id.unwrap(),
                    LastId: serialized.first().unwrap().Id.clone()
                };
                let channel_id = add.Channel_Id as i64; 
                sqlx::query!(
                    "INSERT INTO JellyCord (Active_channel, Channel_Id, Endpoint, Token, UserId, LastId) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    add.Active_channel, channel_id, add.Endpoint, add.Token, add.UserId, add.LastId).execute(&database)
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
        "DELETE FROM JellyCord WHERE Channel_Id=?",
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
        "UPDATE JellyCord SET Active_channel = 0 WHERE Channel_Id=?",
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
        "UPDATE JellyCord SET Active_channel = 1 WHERE Channel_Id=?",
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



fn get_page(url: String) -> Result<std::vec::IntoIter<NextupLibrary>, isahc::Error> {
    let mut response = Request::get(url).timeout(Duration::from_secs(10))
    .header("Content-Type", "application/json")
    .body(())?.send()?;
    let result = match response.status() {
        StatusCode::OK => {
            let fdsfd: Vec<NextupLibrary> = serde_json::from_str(&response.text().unwrap()).unwrap();
            fdsfd.into_iter()
        }
        _ => panic!("{} your server is missing some api endpoints, i think", response.status())
    };
    Ok(
        result
    )
}
