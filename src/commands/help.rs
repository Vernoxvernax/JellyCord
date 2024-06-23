use serenity::all::{
  Permissions, CreateCommand, CommandDataOption
};

pub async fn run(_options: &[CommandDataOption]) -> String {
  "```\
[JellyCord]

Commands:
  \"init\"  - Initialize current channel and setup jellyfin connection
  \"reset\" - Break jellyfin connection for the current channel
  \"pause\" - Don't check for any updates, regarding this channel | TOGGLE
  \"ping\"  - Check if the bot is still running
```".to_string()
}

pub fn register() -> CreateCommand {
  CreateCommand::new("help")
  .description("Small description of available commands")
  .default_member_permissions(Permissions::ADMINISTRATOR)
}
