use serenity::all::{CommandDataOption, CreateCommand, Permissions};

pub async fn run(_options: &[CommandDataOption]) -> String {
  // For users to check whether the bot is still working.
  "Pong!".to_string()
}

pub fn register() -> CreateCommand {
  CreateCommand::new("ping")
    .description("Wastes bandwidth")
    .default_member_permissions(Permissions::ADMINISTRATOR)
}
