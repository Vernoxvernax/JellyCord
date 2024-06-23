use serenity::all::{CommandDataOption, CreateCommand, Permissions};

pub async fn run(_options: &[CommandDataOption]) -> String {
  "Pong!".to_string()
}

pub fn register() -> CreateCommand {
  CreateCommand::new("ping")
  .description("Wastes bandwidth")
  .default_member_permissions(Permissions::ADMINISTRATOR)
}
