mod chat;
pub mod commands;

use clap::Command;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use nekoai_agent::runtime::{AgentRuntime, RuntimeInitProgress};
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

            let agent_init_bar = ProgressBar::new(RuntimeInitProgress::TOTAL_STEPS);
            let agent_init_style = match ProgressStyle::with_template(
                "    [{bar:32.cyan/blue}] {pos:>2}/{len:2} {msg}",
            ) {
                Ok(style) => style.progress_chars("=>-"),
                Err(_) => ProgressStyle::default_bar(),
            };
            agent_init_bar.set_style(agent_init_style);
            agent_init_bar.set_message("initializing agent runtime");

            let runtime = match AgentRuntime::new_with_progress(
                start_command.config.clone(),
                start_command.memory_store,
                |progress| {
                    agent_init_bar.set_position(progress.completed_steps);
                    agent_init_bar.set_length(progress.total_steps);
                    agent_init_bar.set_message(progress.message);
                },
            )
            .await
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    agent_init_bar.finish_and_clear();
                    error!(error = %err, "failed to initialize agent runtime");
                    println!(
                        "    {} Failed to initialize agent runtime: {}",
                        "✗".red(),
                        err
                    );
                    std::process::exit(1);
                }
            };

            agent_init_bar.finish_and_clear();
            println!("    {} Agent runtime initialized", "✓".green());

            info!("agent runtime initialized");

            let chat_client =
                match chat::ChatClient::initialize(&start_command.config, runtime).await {
                    Ok(client) => client,
                    Err(err) => {
                        error!(error = %err, "failed to initialize chat client");
                        eprintln!("Error: {:#}", err);
                        std::process::exit(1);
                    }
                };

            info!(
                platform = chat_client.platform_name(),
                "chat client initialized"
            );

            if let Err(err) = chat_client.run().await {
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
