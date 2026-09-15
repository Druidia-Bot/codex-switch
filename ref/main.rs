//! codex-switch — flip the Codex CLI between OpenAI and Featherless (or any
//! OpenAI-compatible provider) by rewriting ~/.codex/config.toml in place.
//!
//! The config is edited with `toml_edit`, so comments and formatting in the
//! rest of your file survive untouched. Every write is backed up first.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use dialoguer::FuzzySelect;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut, Item, Table};

const FEATHERLESS_BASE: &str = "https://api.featherless.ai/v1";
const FEATHERLESS_ENV_KEY: &str = "FEATHERLESS_API_KEY";
const PROVIDER_ID: &str = "featherless";
const DEFAULT_OPENAI_MODEL: &str = "gpt-5-codex";

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
    about = "Toggle Codex between OpenAI and Featherless",
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
    /// Switch to OpenAI
    Openai {
        /// Model id (defaults to the last one used, or gpt-5-codex)
        #[arg(long)]
        model: Option<String>,
    },
    /// Switch to Featherless, picking a model from the live catalog
    Featherless {
        /// Skip the picker and use this model id directly
        #[arg(long)]
        model: Option<String>,
        /// Pre-filter the picker list
        #[arg(long)]
        search: Option<String>,
    },
    /// Flip to the other provider, keeping each side's last model
    Toggle,
    /// Print the Featherless catalog without changing anything
    Models {
        /// Case-insensitive substring filter
        #[arg(long)]
        search: Option<String>,
    },
    /// Restore config.toml from the most recent backup
    Undo,
}

// ---------------------------------------------------------------- paths

struct Paths {
    config: PathBuf,
    backup: PathBuf,
    state: PathBuf,
}

impl Paths {
    fn resolve(override_path: Option<PathBuf>) -> Result<Self> {
        let config = match override_path {
            Some(p) => p,
            None => dirs::home_dir()
                .ok_or_else(|| anyhow!("could not determine home directory"))?
                .join(".codex")
                .join("config.toml"),
        };
        let dir = config
            .parent()
            .ok_or_else(|| anyhow!("config path has no parent directory"))?
            .to_path_buf();
        Ok(Paths {
            backup: dir.join("config.toml.codex-switch.bak"),
            state: dir.join("codex-switch-state.json"),
            config,
        })
    }
}

// ---------------------------------------------------------------- state

/// Remembers the last model used on each side so `toggle` is lossless.
#[derive(Default, Serialize, Deserialize)]
struct State {
    last_openai_model: Option<String>,
    last_featherless_model: Option<String>,
}

impl State {
    fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json).with_context(|| format!("writing state to {}", path.display()))
    }
}

// ---------------------------------------------------------------- config

fn load_doc(path: &Path) -> Result<DocumentMut> {
    if !path.exists() {
        // A brand-new config is fine — start from empty.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        return Ok(DocumentMut::new());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    text.parse::<DocumentMut>()
        .with_context(|| format!("{} is not valid TOML", path.display()))
}

fn save_doc(paths: &Paths, doc: &DocumentMut) -> Result<()> {
    if paths.config.exists() {
        fs::copy(&paths.config, &paths.backup).with_context(|| {
            format!("backing up {} before writing", paths.config.display())
        })?;
    }
    fs::write(&paths.config, doc.to_string())
        .with_context(|| format!("writing {}", paths.config.display()))
}

fn current(doc: &DocumentMut) -> (String, String) {
    let provider = doc
        .get("model_provider")
        .and_then(|i| i.as_str())
        .unwrap_or("openai")
        .to_string();
    let model = doc
        .get("model")
        .and_then(|i| i.as_str())
        .unwrap_or("(unset)")
        .to_string();
    (provider, model)
}

/// Write the [model_providers.featherless] block, creating parents as needed.
fn ensure_featherless_provider(doc: &mut DocumentMut) {
    if doc.get("model_providers").and_then(|i| i.as_table()).is_none() {
        let mut parent = Table::new();
        parent.set_implicit(true); // don't emit a bare [model_providers] header
        doc["model_providers"] = Item::Table(parent);
    }
    let providers = doc["model_providers"].as_table_mut().unwrap();

    if providers.get(PROVIDER_ID).and_then(|i| i.as_table()).is_none() {
        providers.insert(PROVIDER_ID, Item::Table(Table::new()));
    }
    let fl = providers[PROVIDER_ID].as_table_mut().unwrap();

    fl["name"] = value("Featherless");
    fl["base_url"] = value(FEATHERLESS_BASE);
    fl["env_key"] = value(FEATHERLESS_ENV_KEY);
    // Codex defaults to the Responses API; third-party OpenAI-compatible
    // endpoints only implement chat completions.
    fl["wire_api"] = value("chat");
}

fn set_provider(doc: &mut DocumentMut, provider: &str, model: &str) {
    doc["model_provider"] = value(provider);
    doc["model"] = value(model);
}

// ---------------------------------------------------------------- catalog

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize, Clone)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    context_length: Option<u64>,
    #[serde(default)]
    model_class: Option<String>,
    #[serde(default)]
    owned_by: Option<String>,
}

impl ModelEntry {
    fn label(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ctx) = self.context_length {
            parts.push(format!("{}k ctx", ctx / 1000));
        }
        if let Some(class) = &self.model_class {
            parts.push(class.clone());
        } else if let Some(owner) = &self.owned_by {
            parts.push(owner.clone());
        }
        if parts.is_empty() {
            self.id.clone()
        } else {
            format!("{}  {}({}){}", self.id, DIM, parts.join(", "), RESET)
        }
    }
}

fn fetch_models() -> Result<Vec<ModelEntry>> {
    let url = format!("{}/models", FEATHERLESS_BASE);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut req = client.get(&url);
    // The catalog is public, but send the key if we have one so any
    // account-scoped models show up too.
    if let Ok(key) = std::env::var(FEATHERLESS_ENV_KEY) {
        if !key.trim().is_empty() {
            req = req.bearer_auth(key);
        }
    }

    let resp = req.send().with_context(|| format!("GET {}", url))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        bail!("Featherless returned {}: {}", status, body.chars().take(300).collect::<String>());
    }

    let list: ModelList = resp.json().context("parsing the model list as JSON")?;
    let mut models = list.data;
    models.sort_by(|a, b| a.id.to_lowercase().cmp(&b.id.to_lowercase()));
    Ok(models)
}

fn filter(models: &[ModelEntry], needle: Option<&str>) -> Vec<ModelEntry> {
    match needle {
        None => models.to_vec(),
        Some(n) => {
            let n = n.to_lowercase();
            models
                .iter()
                .filter(|m| m.id.to_lowercase().contains(&n))
                .cloned()
                .collect()
        }
    }
}

fn pick_model(models: &[ModelEntry], preselect: Option<&str>) -> Result<String> {
    if models.is_empty() {
        bail!("no models matched that filter");
    }
    let labels: Vec<String> = models.iter().map(|m| m.label()).collect();
    let start = preselect
        .and_then(|p| models.iter().position(|m| m.id == p))
        .unwrap_or(0);

    let idx = FuzzySelect::new()
        .with_prompt("Model (type to filter)")
        .items(&labels)
        .default(start)
        .interact()
        .context("model selection cancelled")?;

    Ok(models[idx].id.clone())
}

// ---------------------------------------------------------------- commands

fn cmd_status(paths: &Paths) -> Result<()> {
    let doc = load_doc(&paths.config)?;
    let (provider, model) = current(&doc);

    let tag = if provider == PROVIDER_ID {
        format!("{}{}Featherless{}", BOLD, CYAN, RESET)
    } else {
        format!("{}{}OpenAI{}", BOLD, GREEN, RESET)
    };

    println!("Provider  {}", tag);
    println!("Model     {}{}{}", BOLD, model, RESET);
    println!("Config    {}{}{}", DIM, paths.config.display(), RESET);

    if provider == PROVIDER_ID && std::env::var(FEATHERLESS_ENV_KEY).is_err() {
        println!(
            "\n{}warning{}  {} is not set in this shell — Codex will fail to authenticate.",
            YELLOW, RESET, FEATHERLESS_ENV_KEY
        );
    }
    Ok(())
}

fn switch_openai(paths: &Paths, model: Option<String>) -> Result<()> {
    let mut state = State::load(&paths.state);
    let model = model
        .or_else(|| state.last_openai_model.clone())
        .unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string());

    let mut doc = load_doc(&paths.config)?;
    set_provider(&mut doc, "openai", &model);
    save_doc(paths, &doc)?;

    state.last_openai_model = Some(model.clone());
    state.save(&paths.state)?;

    println!("{}→{} OpenAI  {}{}{}", GREEN, RESET, BOLD, model, RESET);
    Ok(())
}

fn switch_featherless(
    paths: &Paths,
    model: Option<String>,
    search: Option<String>,
) -> Result<()> {
    let mut state = State::load(&paths.state);

    let model = match model {
        Some(m) => m,
        None => {
            println!("{}Fetching Featherless catalog…{}", DIM, RESET);
            let all = fetch_models()?;
            let shown = filter(&all, search.as_deref());
            println!("{}{} models available{}\n", DIM, shown.len(), RESET);
            pick_model(&shown, state.last_featherless_model.as_deref())?
        }
    };

    let mut doc = load_doc(&paths.config)?;
    ensure_featherless_provider(&mut doc);
    set_provider(&mut doc, PROVIDER_ID, &model);
    save_doc(paths, &doc)?;

    state.last_featherless_model = Some(model.clone());
    state.save(&paths.state)?;

    println!("{}→{} Featherless  {}{}{}", CYAN, RESET, BOLD, model, RESET);
    if std::env::var(FEATHERLESS_ENV_KEY).is_err() {
        println!(
            "{}warning{}  export {}=… before running codex.",
            YELLOW, RESET, FEATHERLESS_ENV_KEY
        );
    }
    Ok(())
}

fn cmd_toggle(paths: &Paths) -> Result<()> {
    let doc = load_doc(&paths.config)?;
    let (provider, _) = current(&doc);
    if provider == PROVIDER_ID {
        switch_openai(paths, None)
    } else {
        let state = State::load(&paths.state);
        // Reuse the remembered model so toggling never opens the picker.
        switch_featherless(paths, state.last_featherless_model.clone(), None)
    }
}

fn cmd_models(search: Option<String>) -> Result<()> {
    let all = fetch_models()?;
    let shown = filter(&all, search.as_deref());
    for m in &shown {
        println!("{}", m.label());
    }
    println!("\n{}{} of {} models{}", DIM, shown.len(), all.len(), RESET);
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
        Some(Command::Openai { model }) => switch_openai(&paths, model),
        Some(Command::Featherless { model, search }) => {
            switch_featherless(&paths, model, search)
        }
        Some(Command::Toggle) => cmd_toggle(&paths),
        Some(Command::Models { search }) => cmd_models(search),
        Some(Command::Undo) => cmd_undo(&paths),
    }
}
