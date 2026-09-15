# Codex Switch

**Use any AI model inside Codex, with one click.**

Codex is great, but it's locked to OpenAI's models. Codex Switch adds an
ON/OFF switch to your desktop. Flip it **ON** and Codex (the desktop app and
the command line) runs on any of the ~430 models on
[OpenRouter](https://openrouter.ai): DeepSeek, Claude, Gemini, GLM, Qwen, Llama
and the rest. Flip it **OFF** and you're back on OpenAI, set up exactly as
before.

- **One click to switch.** No config files to edit and no terminal needed.
- **Pick any model from a dropdown.** Search it by name. The models that
  developers use most for coding are listed first.
- **The next thread uses the new model.** Start a new thread and it's on the
  model you picked. You don't restart anything.
- **It works in the Codex app's own model picker too.** The OpenRouter models
  show up there next to OpenAI's.
- **It puts your setup back exactly.** When you turn it off, your original
  OpenAI settings come back. A backup is saved before every change, and
  **Undo** restores it.
- **Your API key stays on your computer.** It's stored encrypted on your PC and
  only goes to OpenRouter. While the switch is ON, it's also written into
  Codex's settings file so Codex can use it.
- **Nothing extra runs in the background.** No proxy or server. It changes
  Codex's settings and gets out of the way.

---

## Get started in 5 minutes (Windows)

You don't need to know how to code. You need:

- **Codex** installed (the desktop app, the command line, or both)
- An **OpenRouter account** with a little credit. Sign up at
  [openrouter.ai](https://openrouter.ai) and add a few dollars under *Credits*.

### 1. Download

Get **`codex-switch-windows-x64.zip`** from the
[latest release](https://github.com/Druidia-Bot/codex-switch/releases/latest).
Right-click the zip, choose **Extract All…**, and extract it anywhere, such as
your Downloads folder.

> Works on normal Windows PCs and on Windows on ARM (Surface, Snapdragon laptops).

### 2. Install

In the extracted folder, right-click **`install-start-menu.ps1`** and choose
**Run with PowerShell**. A blue window opens for a moment and closes on its own.

That's the whole install. It adds **Codex Switch** to your Start Menu.

<details>
<summary>The script didn't run, or the window closed straight away</summary>

Windows sometimes blocks scripts. Open the extracted folder, click the address
bar, type `powershell`, and press Enter. Then paste this and press Enter:

```powershell
powershell -ExecutionPolicy Bypass -File .\install-start-menu.ps1
```
</details>

### 3. Open it

Press the **Windows key**, type **Codex Switch**, and open it. A small card
appears. Drag it wherever you like, or click 📌 to keep it on top.

> **"Windows protected your PC"?** The app is new and isn't code-signed yet, so
> Windows SmartScreen doesn't recognise it. Click **More info → Run anyway**.
> You only have to do this once.

### 4. Add your OpenRouter key

1. Go to [openrouter.ai/keys](https://openrouter.ai/keys) and click
   **Create Key**. Copy the key; it starts with `sk-or-`.
2. In Codex Switch, click **⚙**, paste the key, and save.

Codex Switch checks the key with OpenRouter before saving it, so you'll know
straight away if it was mistyped.

### 5. Restart Codex, once

**Close the Codex app completely and open it again.** Codex only reads its
list of models when it starts, and this restart lets it pick up the new ones.
You won't need to restart it again.

### 6. Flip it on

1. Flip the switch from **OpenAI** to **OpenRouter**.
2. Pick a model from the dropdown. Type in it to search, e.g. `deepseek` or
   `claude`.
3. **Start a new thread** in Codex. It now runs on the model you picked.

To go back to OpenAI, turn the switch **OFF** and start a new thread.

---

## Everyday use

| You want to… | Do this |
| --- | --- |
| Use a different model | Pick it from the dropdown, then start a new thread |
| Go back to OpenAI | Switch **OFF**, then start a new thread |
| See newly added OpenRouter models | Click **↻** |
| Change your API key | Click **⚙** |
| Open it every day | Right-click **Codex Switch** in the Start Menu → **Pin to taskbar** |

Codex Switch remembers the last model you used, so switching ON takes you
straight back to it. Models you've used recently appear at the top of the
list.

## Good to know

- **Start a new thread after switching.** A conversation started on OpenAI
  can't continue on another provider, and the reverse is true too. Threads you
  already have keep working on the provider they started on.
- **Pick models that support tools.** Codex runs commands and edits files, so a
  model has to support "tool calling" to be useful. Models that don't are
  hidden from the Codex app's picker and listed last in the dropdown.
- **Watch your costs.** Codex sends the model a lot of context on every turn:
  its instructions, your files, and any plugins or MCP servers you have set
  up. That can be hundreds of thousands of tokens per turn, and OpenRouter
  bills you for every one. Cheap, fast models such as DeepSeek V4.1 Flash cost
  a few cents a turn. Check prices on the model's OpenRouter page, and watch
  your usage at [openrouter.ai/activity](https://openrouter.ai/activity).
- **Small-context models may fail.** For the same reason, models with a small
  context window (for example 128k tokens) may reject Codex's requests. Models
  with large windows work best.
- **Some Codex tools are OpenAI-only.** Web search and image generation only
  work with OpenAI models.
- **The OpenRouter models stay in the Codex app's picker when the switch is
  OFF.** They're labelled "· OpenRouter", but they won't work until you switch
  ON.

## Troubleshooting

**The OpenRouter models don't appear in the Codex app.**
Close the Codex app completely and reopen it (step 5). Codex Switch shows a
warning in its status line when the app needs this restart.

**Codex says the model returned an error or a 402.**
Your OpenRouter credit has probably run out. Top up at
[openrouter.ai/credits](https://openrouter.ai/credits).

**Something about my Codex setup looks wrong.**
Switch **OFF** first. If it's still wrong, open a terminal and run
`codex-switch undo`. That restores Codex's settings file from the backup taken
before the last change.

**Updating to a new version.**
Download the new zip and run `install-start-menu.ps1` again. It closes the
running app and replaces it for you.

## Uninstall

1. Open a terminal and run `codex-switch uninstall`. This switches OFF and
   removes everything Codex Switch added to your Codex settings.
2. Delete `codex-switch.exe` and `codex-switch-bar.exe` from
   `%USERPROFILE%\.local\bin`.
3. Delete `Codex Switch.lnk` from
   `%APPDATA%\Microsoft\Windows\Start Menu\Programs` to remove the Start Menu
   entry.
4. Optionally, delete the `%USERPROFILE%\.codex\codex-switch` folder. It holds
   your saved key and the cached model list.

---

# For developers

Everything below is for people who want the command line, want to build from
source, or want to know how it works.

## Command line

The installer puts `codex-switch` on your PATH. It does everything the
toolbar does:

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

Point it at a different file with `--config /path/to/config.toml`. The toolbar
re-reads the config every 1.5 s, so changes made from the CLI show up there.

## Build from source

Requires [Rust](https://rustup.rs) (stable).

```bash
git clone https://github.com/Druidia-Bot/codex-switch.git
cd codex-switch
cargo build --release
```

This produces two binaries in `target/release/`:

- **`codex-switch-bar`**: the floating toolbar
- **`codex-switch`**: the CLI

**Windows:** after building, run `.\scripts\install-start-menu.ps1` from the
repo root. It finds the binaries in `target\release`, copies them to
`~\.local\bin`, adds that folder to PATH, and creates the Start Menu entry.
`.\scripts\package.ps1` builds the release zip into `dist\`.

Windows build notes:

- The release is built with the x64 MSVC toolchain
  (`cargo +stable-x86_64-pc-windows-msvc build --release`), which also runs on
  Windows on ARM under emulation.
- If Git for Windows is on your PATH, its `link.exe` can shadow the MSVC
  linker. Strip it for the build:
  ```powershell
  $env:PATH = ($env:PATH -split ';' | Where-Object { $_ -notmatch 'Git\\usr\\bin' }) -join ';'
  ```

**macOS / Linux:** the same CLI and toolbar build and run there. There's no
DPAPI, so set the key in the `OPENROUTER_API_KEY` environment variable instead
of storing it.

## How it works

Verified against Codex CLI 0.153 and Codex Desktop 26.908:

1. **Codex re-reads `~/.codex/config.toml` every time a thread starts.**
   Flipping `model_provider` therefore applies to the *next* thread in the
   Codex app or CLI, without a restart.
2. **OpenRouter speaks Codex's wire protocol natively.** Codex only supports
   the Responses API (`wire_api = "chat"` is rejected), and OpenRouter's
   `/api/v1/responses` implements it, tool calls included. A DeepSeek turn that
   runs shell commands works with a plain provider block, so no local proxy is
   needed.
3. **The Codex app's model picker comes from a model catalog.** Codex only has
   metadata (context window, reasoning levels) for OpenAI models, so
   codex-switch writes a *combined* catalog: the native OpenAI models, copied
   verbatim from Codex's own cache, plus every OpenRouter model with Codex's
   real system prompt attached. It registers the catalog via
   `model_catalog_json`. The app-server caches that list for its lifetime, so
   the catalog stays registered permanently and the app needs one restart
   after the first install.
4. **The key goes in the config.** While ON, the OpenRouter key is written into
   the provider block (`experimental_bearer_token`), which the running app
   picks up immediately; an environment variable would only be seen after a
   restart. OFF removes the block again. The key is stored DPAPI-encrypted under
   `~/.codex/codex-switch/`. An existing DeepAstra key is imported
   automatically.

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
moment you turn it ON and put back exactly on OFF, and the provider block (with
the key) is removed. `service_tier = "priority"` is OpenAI-only and makes Codex
warn on every OpenRouter turn, so it is parked while ON. Only the
`model_catalog_json` line persists after OFF; `codex-switch uninstall` removes
that too. `toml_edit` keeps comments and the rest of the file untouched, and
every write is backed up to `config.toml.codex-switch.bak` first.

### Model ordering

The picker (toolbar, CLI and the Codex app's catalog) is ordered:

1. **Recent**: models you used through codex-switch, newest first
2. **Popular for coding**: OpenRouter's live *programming* ranking
   (`/models?category=programming`), with a curated fallback when offline
3. **All models**: alphabetical. Models without tool calling go last and are
   hidden from the Codex app's picker.

### Model metadata

- **Context window** comes from OpenRouter's catalog (DeepSeek V4.1 Flash:
  1,048,576 tokens, auto-compaction at 80 %). Without the catalog, Codex would
  clamp unknown models to its 272k fallback.
- **Reasoning effort**: models that advertise `reasoning` on OpenRouter get
  low/medium/high; others get a single level.
- **Provider mixing**: reasoning models such as DeepSeek return reasoning
  items in a format OpenAI rejects, and OpenAI's encrypted reasoning items
  can't be read by other vendors. That's why threads can't move between
  providers.

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

The shadow home exists because Codex stops refreshing its own model cache
while a custom catalog is registered. Before every catalog regeneration,
codex-switch asks Codex for a fresh list there (one `model/list` call, which
spends no tokens).
