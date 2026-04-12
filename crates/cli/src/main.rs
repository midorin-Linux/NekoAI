pub mod commands;

use agent::runtime::AgentRuntime;
use clap::Command;
use discord::client::DiscordClient;

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
                    eprintln!("Error: {:#}", err);
                    std::process::exit(1);
                }
            };

            if let Err(err) =
                match AgentRuntime::new(start_command.config.clone(), start_command.memory_store)
                    .await
                {
                    Ok(runtime) => {
                        DiscordClient::new(
                            start_command.config.discord.token,
                            1233632516750184489,
                            runtime,
                        )
                        .await
                        .unwrap()
                        .run()
                        .await
                    }
                    Err(err) => {
                        eprintln!("Error: {:#}", err);
                        std::process::exit(1);
                    }
                }
            {};
            std::process::exit(0);
        }
        _ => {
            println!("Please specify a command. Use --help for more information.");
            std::process::exit(1);
        }
    }
}
