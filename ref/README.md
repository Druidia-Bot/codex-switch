# codex-switch

Flip the Codex CLI between OpenAI and Featherless by rewriting `~/.codex/config.toml`.

Uses `toml_edit`, so comments and formatting elsewhere in your config survive
untouched. Every write is backed up to `config.toml.codex-switch.bak` first.

## Build

```bash
cargo build --release
cp target/release/codex-switch ~/.local/bin/
```

## Use

```bash
codex-switch                    # status (default)
codex-switch featherless        # fetch catalog, fuzzy-pick a model
codex-switch featherless --search qwen3.8    # pre-filter the picker
codex-switch featherless --model huihui-ai/Huihui-Qwen3.8-27B-abliterated
codex-switch openai             # back to OpenAI
codex-switch toggle             # flip sides, no picker
codex-switch models --search abliterated     # browse without changing anything
codex-switch undo               # restore from backup
```

`toggle` remembers the last model used on each side, so flipping back and forth
never loses your selection or opens the picker.

Point it at a different file with `--config /path/to/config.toml`.

## Setup

```bash
export FEATHERLESS_API_KEY="your-key"
```

Put that in your shell profile. `status` warns if you're on Featherless without
it set.

## What it writes

Switching to Featherless adds this block if absent, then sets the two root keys:

```toml
model = "huihui-ai/Huihui-Qwen3.8-27B-abliterated"
model_provider = "featherless"

[model_providers.featherless]
name = "Featherless"
base_url = "https://api.featherless.ai/v1"
env_key = "FEATHERLESS_API_KEY"
wire_api = "chat"
```

`wire_api = "chat"` is the important one. Codex defaults to OpenAI's Responses
API, which third-party OpenAI-compatible endpoints don't implement.

## Adding more providers

The Featherless block is written by `ensure_featherless_provider()` in
`src/main.rs`. To support another OpenAI-compatible endpoint, copy that function,
change the three constants at the top of the file, and add a subcommand. The rest
of the machinery is provider-agnostic.
