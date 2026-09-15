# codex-switch

An ON/OFF switch that routes Codex (the CLI **and** the Codex desktop app)
through **OpenRouter** — any of its ~430 models, DeepSeek V4.1 Flash included —
and restores your original OpenAI config when you flip it back. No proxy, and
the flip itself needs no app restart.

Ships as two binaries:

- **`codex-switch-bar`** — a small floating toolbar with the switch, a model dropdown and key settings
- **`codex-switch`** — the CLI (status / on / off / toggle / model / models / key / catalog / undo)

## Download and run (Windows)

1. Grab `codex-switch-windows-x64.zip` from the
   [latest release](https://github.com/Druidia-Bot/codex-switch/releases/latest)
   and extract it anywhere. (The x64 build also runs on Windows on ARM.)
2. Right-click `install-start-menu.ps1` → **Run with PowerShell**. It copies the
   two exes to `%USERPROFILE%\.local\bin`, puts that on your PATH, and adds a
   **Codex Switch** entry to the Start Menu. If PowerShell refuses to run scripts:
   `powershell -ExecutionPolicy Bypass -File .\install-start-menu.ps1`
3. Press the Windows key, type **Codex Switch**, open it.
4. Click **⚙** and paste your OpenRouter API key (from
   [openrouter.ai/keys](https://openrouter.ai/keys)). It is verified and stored
   encrypted. If you already run DeepAstra, its key is imported automatically.
5. **Restart the Codex app once.** The first launch of the bar registers a model
   catalog with Codex; the app only reads that list when it starts. After this
   one restart, flips are instant.
6. Flip the switch **ON**, pick a model from the dropdown, and start a new
   thread in Codex (app or CLI). Flip **OFF** to go back to OpenAI exactly as
   you had it.

The exes are unsigned, so Windows SmartScreen may show "Windows protected your
PC" the first time: click **More info → Run anyway**, or build from source.

macOS/Linux: build from source (see below); the same CLI and bar work, with the
key read from `OPENROUTER_API_KEY`.

## How it works

Everything is verified against Codex CLI 0.153 and Codex Desktop 26.908:

1. **Codex re-reads `~/.codex/config.toml` every time a thread starts.**
   Flipping `model_provider` therefore applies to the *next* thread you start in
   the Codex app or CLI — nothing needs restarting.
2. **OpenRouter speaks Codex's wire protocol natively.** Codex only supports the
   Responses API now (`wire_api = "chat"` is rejected), and OpenRouter's
   `/api/v1/responses` implements it, including tool calls, so a DeepSeek turn that
   runs shell commands works with a plain provider block. No local proxy.
3. **The Codex app's model picker comes from a model catalog.** Codex only knows
   metadata (context window, reasoning levels) for OpenAI models, so codex-switch
   writes a *combined* catalog — the native OpenAI models copied verbatim from
   Codex's own cache plus every OpenRouter model, with Codex's real system prompt
   attached — and registers it via `model_catalog_json`. The app-server caches
   that list for its lifetime, so the catalog stays registered permanently and the
   Codex app needs **one restart after the first install** to list the OpenRouter
   models in its own picker. After that, flips are hot.
4. **The key rides in config.** While ON, the OpenRouter key is written into the
   provider block (`experimental_bearer_token`), which the running app picks up
   immediately; an environment variable would only be seen after a restart. OFF
   removes the block again. The key is stored DPAPI-encrypted under
   `~/.codex/codex-switch/`, and an existing DeepAstra key is imported automatically.

### What ON writes

```toml
model = "deepseek/deepseek-v4.1-flash"
model_provider = "openrouter"
model_catalog_json = "C:/Users/you/.codex/codex-switch/model-catalog.json"

[model_providers.openrouter]
name = "OpenRouter"
base_url = "https://openrouter.ai/api/v1"
wire_api = "responses"
supports_websockets = false
experimental_bearer_token = "sk-or-…"   # or env_key = "OPENROUTER_API_KEY" if that is set
```

Your original `model` / `model_provider` / `service_tier` are snapshotted the
moment you turn it ON and put back exactly on OFF; the provider block (and key)
is removed. (`service_tier = "priority"` is OpenAI-only and makes Codex warn on
every OpenRouter turn, so it is parked while ON.) Only
the `model_catalog_json` line persists (it is what makes the app list the
models); `codex-switch uninstall` removes that too. `toml_edit` keeps every
comment and the rest of the file untouched, and each write is backed up to
`config.toml.codex-switch.bak` first (`codex-switch undo` restores it).

### Model ordering

The picker (bar, CLI and the catalog the Codex app sees) is ordered:

1. **Recent** — models you used through codex-switch, newest first
2. **Popular for coding** — OpenRouter's live *programming* ranking
   (`/models?category=programming`), with a curated fallback when offline
3. **All models** — alphabetical; models without tool calling go last and are
   hidden from the Codex app's picker (Codex needs tools to be useful)

Switching ON always uses the last model you had selected.

## The floating toolbar

```
codex-switch-bar
```

A small frameless card, draggable by its title bar. Caption buttons use the
system Segoe Fluent Icons font: refresh catalog, settings, pin, minimize, close.

- **Toggle OFF** → OpenAI, your config restored.
- **Toggle ON** → OpenRouter with your last model. A dropdown lists the whole
  catalog with search; a pick is applied to `config.toml` immediately.
- **⚙** → enter your OpenRouter API key. It is verified against OpenRouter
  before being stored.
- **↻** → re-fetch the OpenRouter catalog and regenerate the combined catalog.
- The status line shows what Codex is pointed at now and warns when the Codex
  app was started before the catalog was registered (restart it once).

The bar re-reads the config every 1.5 s, so changes made by the CLI show up.

## CLI

```bash
codex-switch                       # status (default)
codex-switch on                    # OpenRouter with the last model (picker on first use)
codex-switch on --pick             # force the fuzzy picker
codex-switch on --search deepseek  # picker, pre-filtered
codex-switch on --model deepseek/deepseek-v4.1-flash
codex-switch model anthropic/claude-sonnet-5   # change model (turns ON if needed)
codex-switch off                   # back to your original OpenAI config
codex-switch toggle                # flip sides, no picker
codex-switch models --search glm   # browse without changing anything
codex-switch key --set sk-or-…     # verify + store the key
codex-switch key                   # verify the stored key
codex-switch catalog --refresh     # regenerate the combined catalog now
codex-switch undo                  # restore config.toml from backup
codex-switch uninstall             # OFF + remove the catalog registration
```

Point it at a different file with `--config /path/to/config.toml`.

## DeepSeek and other non-OpenAI models — known limits

- **Don't move an existing thread between providers.** DeepSeek (and other
  reasoning models) return reasoning items in a format OpenAI rejects, and
  OpenAI's encrypted reasoning items can't be read by other vendors. Start a
  new thread after flipping; threads keep working on the provider they began on.
- **Context window** comes from OpenRouter's catalog (DeepSeek V4.1 Flash:
  1,048,576 tokens, auto-compaction at 80 %). Without the catalog Codex would
  clamp unknown models to its 272k fallback.
- **Reasoning effort**: models that advertise `reasoning` on OpenRouter get
  low/medium/high; others get a single level. OpenRouter ignores Codex's
  reasoning parameters for models that don't support them.
- Codex's web search and image generation tools are OpenAI-only and are
  disabled for OpenRouter models.
- While OFF, the Codex app still lists the OpenRouter models (they are tagged
  "· OpenRouter"); picking one while OFF fails, because the request goes to
  OpenAI. Flip ON first.

## Notes from the live verification (2026-09-15)

- `codex exec` through the real config with `model_provider = "openrouter"` and
  DeepSeek V4.1 Flash completed a shell tool call; Codex recorded the session
  with `model_provider: openrouter`. OFF left config.toml identical apart from
  the catalog line (restored keys may move to the end of the root table).
- A turn in this Codex setup carries roughly 450–560k input tokens (many MCP
  servers, plugins and skills). That is Codex's normal prompt, not something
  the switch adds, but it matters on OpenRouter: it costs about $0.10 per
  uncached DeepSeek turn, and models with a 128k window will fail outright.
- The Codex desktop app itself was not driven by automation here; the
  config-re-read and picker-caching behaviour was verified against
  `codex app-server`, which is the same process the app embeds.

## Build (Windows / this machine)

This box is Windows on ARM, and the installed MSVC Build Tools only include the
x64 linker, so build with the x86_64 toolchain (the exe runs fine under
Windows' x64 emulation). Git's `link.exe` shadows the MSVC linker, so strip
`Git\usr\bin` from PATH for the build:

```powershell
$env:PATH = ($env:PATH -split ';' | Where-Object { $_ -notmatch 'Git\\usr\\bin' }) -join ';'
cargo +stable-x86_64-pc-windows-msvc build --release
Copy-Item target\release\codex-switch.exe, target\release\codex-switch-bar.exe $HOME\.local\bin\
```

Then add the Start Menu entry (searchable as "Codex Switch"; right-click it to
pin to Start or the taskbar):

```powershell
.\scripts\install-start-menu.ps1
```

On Linux/macOS it's just `cargo build --release`. Key storage falls back to a
plain file there (set `OPENROUTER_API_KEY` in the environment instead).

## Files

| Path | Purpose |
| --- | --- |
| `~/.codex/config.toml` | edited in place |
| `~/.codex/config.toml.codex-switch.bak` | backup before every write |
| `~/.codex/codex-switch-state.json` | last model per side, recents, original snapshot |
| `~/.codex/codex-switch/model-catalog.json` | combined catalog registered with Codex |
| `~/.codex/codex-switch/openrouter-models.json` | cached OpenRouter list (6 h) |
| `~/.codex/codex-switch/openrouter.key` | DPAPI-protected key |
| `~/.codex/codex-switch/native-home/` | shadow Codex home used to refresh the native model list |

The shadow home exists because Codex stops refreshing its own model cache while
a custom catalog is registered; codex-switch asks Codex for a fresh list there
(one `model/list` call, no tokens spent) before every catalog regeneration.
