use anyhow::{Result, bail};
use colored::Colorize;
use dialoguer::{Input, theme::SimpleTheme};
use indicatif::{ProgressBar, ProgressStyle};
use nekoai_config::loader::Config;
use nekoai_infra::logging::{WorkerGuard, init_tracing};
use nekoai_memory::{short_term::ShortTermMemory, store::MemoryStore};
use tokio::time::sleep;
use tracing::{error, info, warn};

pub struct StartCommand {
    pub config: Config,
    _guard: WorkerGuard,
    pub memory_store: MemoryStore,
}

impl StartCommand {
    pub async fn new() -> Result<Self> {
        let (config, guard, memory_store) = Self::start().await?;

        Ok(Self {
            config,
            _guard: guard,
            memory_store,
        })
    }

    pub async fn start() -> Result<(Config, WorkerGuard, MemoryStore)> {
        println!();
        println!("  ███╗   ██╗ ███████╗ ██╗  ██╗  ██████╗       █████╗  ██╗");
        println!("  ████╗  ██║ ██╔════╝ ██║ ██╔╝ ██╔═══██╗     ██╔══██╗ ██║");
        println!("  ██╔██╗ ██║ █████╗   █████╔╝  ██║   ██║     ███████║ ██║");
        println!("  ██║╚██╗██║ ██╔══╝   ██╔═██╗  ██║   ██║     ██╔══██║ ██║");
        println!("  ██║ ╚████║ ███████╗ ██║  ██╗ ╚██████╔╝     ██║  ██║ ██║");
        println!("  ╚═╝  ╚═══╝ ╚══════╝ ╚═╝  ╚═╝  ╚═════╝      ╚═╝  ╚═╝ ╚═╝");
        println!();
        println!("  Welcome to Neko AI!\n");

        sleep(std::time::Duration::from_secs(1)).await;

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("    {spinner} Initializing tracing...")?,
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(120));

        let guard = match init_tracing() {
            Ok(guard) => guard,
            Err(e) => {
                spinner.finish_and_clear();

                println!("    {} Failed to initialize tracing: {}", "✗".red(), e);

                bail!("failed to initialize tracing: {e}");
            }
        };

        spinner.finish_and_clear();

        info!("Tracing initialized successfully");
        println!("    {} Tracing initialized", "✓".green());

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("    {spinner} Loading configuration...")?,
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(120));

        info!("Checking for configuration file");

        if let Ok(false) = std::fs::exists(".config/config.json") {
            spinner.finish_and_clear();

            warn!("No configuration file found at .config/config.json");
            println!("    {} No configuration found\n", "✗".red());
            println!(
                "    It's likely that this is the first time the program is running, or the configuration file has been deleted."
            );

            loop {
                let response: String = Input::with_theme(&SimpleTheme)
                    .with_prompt("    Do you want to run the setup wizard to create a new configuration? [y/n]")
                    .interact_text()?;

                if response.to_lowercase() == "y" {
                    info!("User chose to run the setup wizard");
                    println!("\n    Starting setup wizard...");

                    break;
                } else if response.to_lowercase() == "n" {
                    info!("User chose to cancel the setup wizard");
                    println!("\n    Setup wizard cancelled. Shutting down...");

                    bail!("setup wizard cancelled by user");
                } else {
                    println!("\n    Invalid input. Please enter 'y' or 'n'.");
                }
            }
            // セットアップウィザードを起動
            info!("Running setup wizard to create new configuration");
        };

        let config = Config::load()
            .inspect_err(|_e| {
                spinner.finish_and_clear();
            })
            .inspect(|_| spinner.finish_and_clear())
            .map_err(|e| {
                error!("Failed to load configuration: {}", e);
                println!("    {} Failed to load configuration: {}", "✗".red(), e);
                e
            })
            .inspect(|_| {
                info!("Configuration loaded successfully");
                println!("    {} Configuration loaded", "✓".green());
            })?;

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("    {spinner} Initializing context memory...")?,
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(120));

        info!("Initializing context memory");

        let _short_term_memory = ShortTermMemory::new(10);
        let memory_store = MemoryStore::new(&config);

        memory_store.initialize().await.inspect_err(|e| {
            error!(error = %e, "failed to initialize vector memory collections");
        })?;

        // Start background cleanup job for mid-term memory retention
        memory_store.start_cleanup_job();

        spinner.finish_and_clear();

        info!("context memory initialized successfully");
        println!("    {} context memory initialized", "✓".green());

        Ok((config, guard, memory_store))
    }
}
