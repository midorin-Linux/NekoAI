use crate::bot::commands::{admin::*, general::*};
use crate::agent::agent::Agent;
use std::sync::Arc;

pub struct Data {
    #[allow(unused)]
    pub agent: Arc<Agent>,
}
pub type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, anyhow::Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

pub async fn command_framework(guild_id: u64, agent: Arc<Agent>) -> poise::framework::Framework<Data, anyhow::Error> {
    let mut admin_commands = vec![
        prompt::prompt(),
        exec::exec(),
    ];

    let mut general_commands = vec![
        ping::ping(),
    ];

    let mut commands = vec![];
    commands.append(&mut admin_commands);
    commands.append(&mut general_commands);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("w!".into()),
                ..Default::default()
            },
            on_error: |error| Box::pin(on_error(error)),
            pre_command: |ctx| {
                Box::pin(async move {
                    tracing::info!("Execute command {:#?}...", ctx.command().qualified_name);
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    tracing::info!("Command {:#?} executed.", ctx.command().qualified_name);
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id.into()).await.unwrap();
                Ok(Data {
                    agent,
                })
            })
        })
        .build();

    framework
}
