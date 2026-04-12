pub mod commands;

use agent::runtime::AgentRuntime;
use clap::Command;
use discord::client::DiscordClient;
use tracing::{error, info, warn};

fn cli() -> Command {
    Command::new("neko")
        .about("NekoAI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("start").about("Start NekoAI"))
}

#[tokio::main]
async fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("start", _sub_matches)) => {
            let start_command = match commands::start::StartCommand::new().await {
                Ok(start_command) => start_command,
                Err(err) => {
                    error!(error = %err, "failed to initialize start command");
                    eprintln!("Error: {:#}", err);
                    std::process::exit(1);
                }
            };

            info!("start command initialized");

            let runtime =
                match AgentRuntime::new(start_command.config.clone(), start_command.memory_store)
                    .await
                {
                    Ok(runtime) => runtime,
                    Err(err) => {
                        error!(error = %err, "failed to initialize agent runtime");
                        eprintln!("Error: {:#}", err);
                        std::process::exit(1);
                    }
                };

            info!("agent runtime initialized");

            let discord_client = match DiscordClient::new(
                start_command.config.discord.token,
                1233632516750184489,
                runtime,
            )
            .await
            {
                Ok(client) => client,
                Err(err) => {
                    error!(error = %err, "failed to create discord client");
                    eprintln!("Error: {:#}", err);
                    std::process::exit(1);
                }
            };

            if let Err(err) = discord_client.run().await {
                error!(error = %err, "application terminated with error");
                eprintln!("Error: {:#}", err);
                std::process::exit(1);
            }

            info!("application exited successfully");
            std::process::exit(0);
        }
        _ => {
            warn!("no command specified");
            println!("Please specify a command. Use --help for more information.");
            std::process::exit(1);
        }
    }
}
