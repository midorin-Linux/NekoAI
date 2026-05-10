use anyhow::{Result, bail};
use colored::Colorize;
use dialoguer::{Confirm, Input, Select, theme::SimpleTheme};
use nekoai_config::loader::{
    ChatPlatform, Config, ConversationModel, Discord, EmbeddingModel, Memory, Provider, SecretKey,
    SummarizerModel, ToolPermissions, VectorDb,
};

const EMBED_OPTIONS: &[(&str, u64)] = &[("Custom", 1536)];

fn print_header(step: usize, total: usize, title: &str) {
    println!();
    println!(
        "  {}",
        format!("═══ Step {}/{} : {} ", step, total, title)
            .cyan()
            .bold()
    );
    println!();
}

fn print_footer() {
    println!();
    println!("  {}", "[Enter: next]  [Esc/Ctrl+C: exit]".dimmed());
}

/// Run all 4 steps of the setup wizard and return a complete Config.
pub fn run_wizard() -> Result<Config> {
    // ── Step 1: Discord Token ──────────────────────────────────
    print_header(1, 4, "Discord Bot Token");
    println!("  Enter your Discord Bot Token:");
    println!(
        "  {}",
        "  Create a Bot on the Discord Developer Portal and paste the token here.".dimmed()
    );
    println!();
    let token: String = Input::with_theme(&SimpleTheme)
        .with_prompt("  Bot Token")
        .interact_text()?;
    print_footer();

    // ── Step 2: AI Provider (Custom only) ─────────────────────
    print_header(2, 4, "AI Provider");
    println!("  Enter the API endpoint details for your AI provider:");
    println!();
    let base_url: String = Input::with_theme(&SimpleTheme)
        .with_prompt("  Base URL")
        .interact_text()?;
    println!();
    let api_key: String = Input::with_theme(&SimpleTheme)
        .with_prompt("  API Key")
        .interact_text()?;
    print_footer();

    // ── Step 3: Model Selection ────────────────────────────────
    print_header(3, 4, "Model Selection");
    println!("  Enter the AI model names to use:");
    println!(
        "  {}",
        "  The conversation model handles user interactions.".dimmed()
    );
    println!(
        "  {}",
        "  The summarizer model (can be a cheaper/faster one) handles memory summarization."
            .dimmed()
    );
    println!();
    let model_name: String = Input::with_theme(&SimpleTheme)
        .with_prompt("  Conversation model name")
        .interact_text()?;
    println!();
    let summarizer_model_name: String = Input::with_theme(&SimpleTheme)
        .with_prompt("  Summarizer model name")
        .with_initial_text(model_name.clone())
        .interact_text()?;
    println!();
    println!("  Select an embedding model for vector search (long-term & mid-term memory):");
    println!();
    let embed_labels: Vec<&str> = EMBED_OPTIONS.iter().map(|(n, _)| *n).collect();
    let embed_idx = Select::with_theme(&SimpleTheme)
        .with_prompt("  Embedding model")
        .items(&embed_labels)
        .default(0)
        .interact()?;

    let (embed_model_name, embed_dimension) = if embed_idx == EMBED_OPTIONS.len() - 1 {
        // "Custom" option selected
        println!();
        let name: String = Input::with_theme(&SimpleTheme)
            .with_prompt("  Embedding model name")
            .interact_text()?;
        let dim: String = Input::with_theme(&SimpleTheme)
            .with_prompt("  Embedding dimension")
            .default("1536".to_string())
            .interact_text()?;
        let dim: u64 = dim
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid dimension"))?;
        (name, dim)
    } else {
        let (name, dim) = EMBED_OPTIONS[embed_idx];
        (name.to_string(), dim)
    };
    print_footer();

    // ── Step 4: Tool Permissions ──────────────────────────────
    print_header(4, 4, "Tool Permissions");
    println!("  Enable the tools you want the agent to use:");
    println!(
        "  {}",
        "  You can change these later by editing the config file.".dimmed()
    );
    println!();
    let web_search = Confirm::with_theme(&SimpleTheme)
        .with_prompt("  Enable Web Search?")
        .default(false)
        .interact()?;
    let code_exec = Confirm::with_theme(&SimpleTheme)
        .with_prompt("  Enable Code Execution?")
        .default(false)
        .interact()?;
    print_footer();

    // ── Summary & Confirm ─────────────────────────────────────
    println!();
    println!("  {}", "═══ Summary ═══".green().bold());
    println!();
    println!(
        "  Discord Token  : {}",
        "*".repeat(token.len().saturating_sub(4)) + &token[token.len().saturating_sub(4) ..]
    );
    println!("  Base URL        : {}", base_url);
    println!("  Conversation    : {}", model_name);
    println!("  Summarizer      : {}", summarizer_model_name);
    println!(
        "  Embedding Model : {} (dim: {})",
        embed_model_name, embed_dimension
    );
    println!(
        "  Web Search     : {}",
        if web_search {
            "✓ enabled".green()
        } else {
            "✗ disabled".red()
        }
    );
    println!(
        "  Code Execution : {}",
        if code_exec {
            "✓ enabled".green()
        } else {
            "✗ disabled".red()
        }
    );
    println!();

    let confirmed = Confirm::with_theme(&SimpleTheme)
        .with_prompt("  Save this configuration?")
        .default(true)
        .interact()?;

    if !confirmed {
        bail!("setup cancelled by user");
    }

    // ── Build Config ──────────────────────────────────────────
    let config = Config {
        chat_platform: ChatPlatform::Discord,
        discord: Discord {
            token: SecretKey::new(token),
            guild_id: 0,
        },
        provider: Provider {
            conversation_model: ConversationModel {
                provider_base_url: base_url.clone(),
                api_key: SecretKey::new(api_key.clone()),
                model_name: model_name.clone(),
                parameters: Default::default(),
            },
            summarizer_model: SummarizerModel {
                provider_base_url: base_url.clone(),
                api_key: SecretKey::new(api_key.clone()),
                model_name: summarizer_model_name,
                parameters: Default::default(),
            },
            embedding_model: EmbeddingModel {
                provider_base_url: base_url,
                api_key: SecretKey::new("".to_string()),
                model_name: embed_model_name,
                dimension: embed_dimension,
            },
        },
        memory: Memory {
            vector_db: VectorDb::default(),
            short_term_max_entries: 20,
            mid_term_top_k: 3,
            long_term_top_k: 5,
            mid_term_retention_days: 30,
        },
        tools: ToolPermissions {
            web_search,
            code_exec,
        },
    };

    Ok(config)
}
