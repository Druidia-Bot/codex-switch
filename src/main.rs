//! codex-switch — flip the Codex CLI / Codex app between OpenAI and OpenRouter
//! by rewriting ~/.codex/config.toml in place.
//!
//! The config is edited with `toml_edit`, so comments and formatting in the
//! rest of your file survive untouched. Every write is backed up first.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use codex_switch::*;
use dialoguer::FuzzySelect;
use std::fs;
use std::path::PathBuf;

// ANSI helpers — cheaper than pulling in a colour crate.
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

#[derive(Parser)]
#[command(
    name = "codex-switch",
    about = "Toggle Codex between OpenAI and OpenRouter (hot, no app restart)",
    version
)]
struct Cli {
    /// Path to the Codex config (default: ~/.codex/config.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show which provider and model Codex is currently pointed at
    Status,
    /// Switch ON: route Codex through OpenRouter (last model, or pick one)
    On {
        /// Skip the picker and use this OpenRouter model id directly
        #[arg(long)]
        model: Option<String>,
        /// Open the picker even if a model is remembered, pre-filtered
        #[arg(long)]
        search: Option<String>,
        /// Force the picker even if a model is remembered
        #[arg(long)]
        pick: bool,
    },
    /// Switch OFF: restore the original OpenAI config
    Off,
    /// Flip sides, keeping each side's last model
    Toggle,
    /// Change the OpenRouter model (switches ON if needed)
    Model {
        /// OpenRouter model id, e.g. deepseek/deepseek-v4.1-flash
        id: String,
    },
    /// List the OpenRouter catalog without changing anything
    Models {
        /// Case-insensitive substring filter
        #[arg(long)]
        search: Option<String>,
        /// Re-fetch from OpenRouter instead of using the 6h cache
        #[arg(long)]
        refresh: bool,
    },
    /// Store or verify the OpenRouter API key
    Key {
        /// Verify and store this key (DPAPI-protected on Windows)
        #[arg(long)]
        set: Option<String>,
    },
    /// Regenerate the combined model catalog (native + OpenRouter) and make
    /// sure it is registered in config.toml
    Catalog {
        /// Re-fetch the OpenRouter list instead of using the cache
        #[arg(long)]
        refresh: bool,
    },
    /// Restore config.toml from the most recent backup
    Undo,
    /// Switch OFF and remove the catalog registration too
    Uninstall,
}

fn label(m: &ModelEntry) -> String {
    match m.meta() {
        Some(meta) => format!("{}  {}({}){}", m.id, DIM, meta, RESET),
        None => m.id.clone(),
    }
}

fn pick_model(paths: &Paths, cache: &OrCache, search: Option<&str>) -> Result<String> {
    let state = State::load(&paths.state);
    let mut items: Vec<ModelEntry> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    for (section, entries) in sectioned(cache, &state) {
        let shown = filter_models(&entries, search);
        for m in shown {
            labels.push(format!("{}{}{}  {}", DIM, section, RESET, label(&m)));
            items.push(m);
        }
    }
    if items.is_empty() {
        bail!("no models matched that filter");
    }
    let start = state
        .last_openrouter_model
        .as_deref()
        .and_then(|p| items.iter().position(|m| m.id == p))
        .unwrap_or(0);
    let idx = FuzzySelect::new()
        .with_prompt("Model (type to filter)")
        .items(&labels)
        .default(start)
        .interact()
        .context("model selection cancelled")?;
    Ok(items[idx].id.clone())
}

// ---------------------------------------------------------------- commands

fn cmd_status(paths: &Paths) -> Result<()> {
    let doc = load_doc(&paths.config)?;
    let (provider, model) = current(&doc);
    let on = provider == PROVIDER_ID;

    let tag = if on {
        format!("{}{}OpenRouter{}", BOLD, CYAN, RESET)
    } else if provider == "openai" {
        format!("{}{}OpenAI{}", BOLD, GREEN, RESET)
    } else {
        format!("{}{}{}{}", BOLD, GREEN, provider, RESET)
    };
    println!("Provider  {}", tag);
    println!("Model     {}{}{}", BOLD, model, RESET);
    println!("Config    {}{}{}", DIM, paths.config.display(), RESET);
    match key_source_label(paths) {
        Some(src) => println!("Key       {}{}{}", DIM, src, RESET),
        None => println!("Key       {}none — run `codex-switch key --set sk-or-…`{}", YELLOW, RESET),
    }
    let registered = catalog_registered(&doc, paths);
    println!(
        "Catalog   {}{}{}",
        DIM,
        if registered { "registered (Codex app lists OpenRouter models)" } else { "not registered — run `codex-switch catalog`" },
        RESET
    );
    let recent = desktop_recent_models(paths);
    if !recent.is_empty() {
        println!("Codex app {}last used {}{}", DIM, recent.join(", "), RESET);
    }
    if desktop_needs_restart(paths) {
        println!(
            "\n{}note{}  the Codex app was started before the catalog was registered — restart it once so its model picker lists the OpenRouter models. Provider flips already apply to its next thread.",
            YELLOW, RESET
        );
    }
    Ok(())
}

fn cmd_on(paths: &Paths, model: Option<String>, search: Option<String>, pick: bool) -> Result<()> {
    ensure_installed(paths)?;
    let key = key_source(paths).ok_or_else(|| {
        anyhow::anyhow!("no OpenRouter key — run `codex-switch key --set sk-or-…` or set {OPENROUTER_ENV_KEY}")
    })?;
    let state = State::load(&paths.state);
    let model = match model {
        Some(m) => m,
        None if !pick && search.is_none() && state.last_openrouter_model.is_some() => {
            state.last_openrouter_model.clone().unwrap()
        }
        None => {
            println!("{}Fetching OpenRouter catalog…{}", DIM, RESET);
            let cache = load_openrouter(paths, false)?;
            println!("{}{} models available{}\n", DIM, cache.models.len(), RESET);
            pick_model(paths, &cache, search.as_deref())?
        }
    };
    switch_on(paths, &model, &key)?;
    println!("{}→{} OpenRouter  {}{}{}", CYAN, RESET, BOLD, model, RESET);
    if desktop_needs_restart(paths) {
        println!(
            "{}note{}  restart the Codex app once so its picker lists the OpenRouter models (new threads already use OpenRouter).",
            YELLOW, RESET
        );
    }
    Ok(())
}

fn cmd_off(paths: &Paths) -> Result<()> {
    let model = switch_off(paths)?;
    println!("{}→{} OpenAI  {}{}{}", GREEN, RESET, BOLD, model, RESET);
    Ok(())
}

fn cmd_toggle(paths: &Paths) -> Result<()> {
    let doc = load_doc(&paths.config)?;
    if is_on(&doc) {
        cmd_off(paths)
    } else {
        cmd_on(paths, None, None, false)
    }
}

fn cmd_models(paths: &Paths, search: Option<String>, refresh: bool) -> Result<()> {
    let cache = load_openrouter(paths, refresh)?;
    let state = State::load(&paths.state);
    let mut shown = 0;
    for (section, entries) in sectioned(&cache, &state) {
        let filtered = filter_models(&entries, search.as_deref());
        if filtered.is_empty() {
            continue;
        }
        println!("{}{}── {} ──{}", BOLD, DIM, section, RESET);
        for m in &filtered {
            println!("{}", label(m));
            shown += 1;
        }
    }
    println!("\n{}{} of {} models{}", DIM, shown, cache.models.len(), RESET);
    Ok(())
}

fn cmd_key(paths: &Paths, set: Option<String>) -> Result<()> {
    match set {
        Some(k) => {
            let info = save_key(paths, &k)?;
            println!("{}stored{}  {}", GREEN, RESET, info);
        }
        None => match resolve_key(paths) {
            Some(k) => {
                let info = verify_key(&k)?;
                println!("{}ok{}  {} ({})", GREEN, RESET, info, key_source_label(paths).unwrap_or("?"));
            }
            None => bail!("no key stored — run `codex-switch key --set sk-or-…`"),
        },
    }
    Ok(())
}

fn cmd_catalog(paths: &Paths, refresh: bool) -> Result<()> {
    let (cache, n) = rebuild_catalog(paths, refresh)?;
    let changed = ensure_installed(paths)?;
    println!(
        "{}catalog{}  {} entries ({} OpenRouter) → {}",
        GREEN,
        RESET,
        n,
        cache.models.len(),
        paths.catalog.display()
    );
    if changed {
        println!("{}registered{} in {} — restart the Codex app once to see the models in its picker", GREEN, RESET, paths.config.display());
    }
    Ok(())
}

fn cmd_undo(paths: &Paths) -> Result<()> {
    if !paths.backup.exists() {
        bail!("no backup found at {}", paths.backup.display());
    }
    fs::copy(&paths.backup, &paths.config)?;
    println!("{}restored{} {}", GREEN, RESET, paths.config.display());
    cmd_status(paths)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = Paths::resolve(cli.config)?;

    match cli.command {
        None | Some(Command::Status) => cmd_status(&paths),
        Some(Command::On { model, search, pick }) => cmd_on(&paths, model, search, pick),
        Some(Command::Off) => cmd_off(&paths),
        Some(Command::Toggle) => cmd_toggle(&paths),
        Some(Command::Model { id }) => cmd_on(&paths, Some(id), None, false),
        Some(Command::Models { search, refresh }) => cmd_models(&paths, search, refresh),
        Some(Command::Key { set }) => cmd_key(&paths, set),
        Some(Command::Catalog { refresh }) => cmd_catalog(&paths, refresh),
        Some(Command::Undo) => cmd_undo(&paths),
        Some(Command::Uninstall) => {
            uninstall(&paths)?;
            println!("{}removed{} OpenRouter provider and catalog registration", GREEN, RESET);
            Ok(())
        }
    }
}
