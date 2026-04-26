# bwenv

[![Latest release](https://img.shields.io/github/v/release/itzhang89/bwenv?sort=semver)](https://github.com/itzhang89/bwenv/releases/latest)
[![Release workflow](https://github.com/itzhang89/bwenv/actions/workflows/release.yml/badge.svg)](https://github.com/itzhang89/bwenv/actions/workflows/release.yml)

**bwenv** reads [Bitwarden](https://bitwarden.com/) vault data through the official **[Bitwarden CLI](https://github.com/bitwarden/clients)** (`bw`) and prints environment variables for your shell: **`export` lines** (default), **`.env`**, or **JSON**.

## Features

- Map Bitwarden **Login** items and custom fields to **`SERVICE_USER` / `SERVICE_PASSWORD`-style** names.
- **Projects** with folder prefix, optional service filters, and **`bwenv use …`** to switch the active project.
- **Session handling**: respects `BW_SESSION`, caches to `~/.bwenv.d/session`, refreshes on auth errors when a master password is available.
- Optional output to **Claude Code** (`-o claude`) and to files (`-o .env`).

## Table of contents

- [Requirements](#requirements)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Usage](#usage)
- [Configuration](#configuration)
- [How it works](#how-it-works)
- [Examples](#examples)
- [Security & best practices](#security--best-practices)
- [For maintainers](#for-maintainers)

## Requirements

- [`bw`](https://github.com/bitwarden/clients/releases) installed, with `bw login` completed (and vault unlocked as needed before running `bwenv`).

## Installation

### Script (macOS & Linux)

Installs the [latest release](https://github.com/itzhang89/bwenv/releases/latest) binary to **`$HOME/.local/bin`**. Needs `curl`, `python3`, `tar`, and `shasum` (or compatible).

```bash
curl -fsSL https://raw.githubusercontent.com/itzhang89/bwenv/HEAD/install.sh | bash
```

System-wide (e.g. `/usr/local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/itzhang89/bwenv/HEAD/install.sh | sudo env BWENV_INSTALL_DIR=/usr/local/bin bash
```

| Env | Purpose |
|-----|---------|
| `BWENV_INSTALL_DIR` | Target directory for `bwenv` (default: `$HOME/.local/bin`) |
| `BWENV_GITHUB_REPO` | `owner/name` of the repo that hosts releases (default: `itzhang89/bwenv`) |

> [!TIP]
> `HEAD` follows the **default branch** of the repository. If you need a specific branch, replace `HEAD` in the URL with the branch name.

### Manual download (all platforms including Windows)

1. Open **[Releases — Latest](https://github.com/itzhang89/bwenv/releases/latest)**.
2. Download the asset for your platform (each `tar.gz` / `zip` has a matching **`.sha256`** file for verification with `shasum -a 256 -c` or `sha256sum -c`).

| Suffix in filename | Platform |
|--------------------|----------|
| `linux-x64` | Linux x86_64 (glibc) |
| `darwin-arm64` | macOS Apple Silicon |
| `darwin-x64` | macOS Intel |
| `windows-x64` | Windows x86_64 (`.zip` → `bwenv.exe`) |

**Linux / macOS** after download:

```bash
tar -xzf bwenv-0.1.0-linux-x64.tar.gz   # use the exact filename you downloaded
chmod +x bwenv
mv bwenv ~/.local/bin/   # or: sudo mv bwenv /usr/local/bin/
```

**Windows:** unzip the release `.zip` and add the folder containing `bwenv.exe` to your **PATH**, or place `bwenv.exe` in a directory already on PATH.

### Build from source

```bash
git clone https://github.com/itzhang89/bwenv.git
cd bwenv
cargo build --release
# binary: target/release/bwenv
```

## Quick start

```bash
# 1. Add a project (name, Bitwarden folder prefix, optional comma-separated services)
bwenv project add dev developer "mysql,redis"

# 2. Select that project, then load env into the current shell
bwenv use dev
eval "$(bwenv)"
```

Run `bwenv` or `bwenv --help` in an interactive terminal for help and common examples. When stdout is a pipe (e.g. `eval "$(bwenv)"`), the default command is to **generate exports**, not to print the help text—this avoids `zsh` issues with `eval` and patterns like `[OPTIONS]`.

## Usage

### Shell

```bash
eval "$(bwenv)"                         # use current project
eval "$(bwenv use dev)"                 # switch project, then in same line load env
eval "$(bwenv -p developer -s mysql)"   # filter by folder prefix and service
source <(bwenv)                          # alternative to eval
```

For permanent login shells, you can add one of the above to **`~/.zshrc`** or **`~/.bashrc`**. You may be prompted for the master password unless the vault is already unlocked or credentials are provided via [configuration](#configuration).

### Common commands

```bash
bwenv -o .env                 # write to file
bwenv -f json                 # JSON output
bwenv list                    # list matching vault items
bwenv list --folders          # list Bitwarden folder names
bwenv current                 # show current project
bwenv project                 # list projects; see bwenv project --help
bwenv project add <name> <prefix> [services]   # add a project
bwenv config show             # show config
```

Claude Code merge/removal (writes `.claude/settings.local.json` in the current repo):

```bash
bwenv -p developer -s mysql -o claude
bwenv -o claude:remove
```

### Project `/.bwenv` (auto-detect)

In a project directory, a `.bwenv` file can describe the project for detection when you run `bwenv` there:

```yaml
# project/.bwenv
name: "myproject"
prefix: "developer"
services:
  - mysql
  - redis
```

## Configuration

**Master password** resolution order:

1. Environment variable `BW_MASTER_PASSWORD`
2. Value stored in `~/.bwenv.d/bwenv` (if set; file created with **mode `0600`** on save)
3. Interactive prompt (optional prompt to save into the config file)

| Variable | Description |
|----------|-------------|
| `BW_MASTER_PASSWORD` | Master password for `bw unlock` when needed |
| `BW_SESSION` | Bitwarden CLI session string; if set, it is also persisted under `~/.bwenv.d/session` |

**User config file:** `~/.bwenv.d/bwenv` (YAML) — projects, `current_project`, and optional `bitwarden.master_password`.

**Session file:** `~/.bwenv.d/session` (optional cache for `bw --session`).

> [!NOTE]
> `bwenv` uses Bitwarden’s session model: you can also export `BW_SESSION` yourself; see `bw unlock --raw` in the [Bitwarden CLI docs](https://bitwarden.com/help/cli/).

## How it works

### Vault layout (folders & items)

Use Bitwarden **Folders** to group items. A project’s **prefix** matches a **folder name**; **services** narrow which item names to include.

```text
Bitwarden vault
├── developer/          (folder = prefix "developer")
│   ├── mysql           (Login item)
│   └── redis
└── project1/
    ├── aliyun
    └── aws
```

### Config example (`~/.bwenv.d/bwenv`)

```yaml
- name: "dev"
  prefix: "developer"
  services:
    - mysql
    - redis

- name: "prod"
  prefix: "project1"
  services:
    - aliyun
    - aws
```

### Export naming (short)

- Only **Login**-type data: username, password, first URL, TOTP, and **custom fields** by name.
- **Service** segment comes from the item title (if it looks like a path, the **last segment** is used), then **UPPER_SNAKE** for variable names, e.g. `MYSQL_USER`, `MYSQL_PASSWORD`, or `SERVICE_URL` for URL-like fields.

For the full description, run **`bwenv --help`**.

### Example output (same item: folder `developer`, name `mysql`)

**Shell (default, `-f shell`):** `export MYSQL_USER=…` and `export MYSQL_PASSWORD=…`  
**`-f env`:** `KEY=value`  
**`-f json`:** JSON object of keys and values  

## Examples

**Development — export a `.env` and load it**

```bash
bwenv use dev -o .env
set -a && source .env && set +a
```

**Docker / Compose — generate a `.env` for compose**

```bash
bwenv use prod -o .env
```

**CI (non-interactive) — inject the master password from your secret store**

```bash
export BW_MASTER_PASSWORD="…"   # from env / CI secret, never hard-code
bwenv use prod -f json > secrets.json
```

In **GitHub Actions**, map a [repository secret](https://docs.github.com/en/actions/security-guides/using-secrets-in-github-actions) to `BW_MASTER_PASSWORD` in the job `env` block, then run `bwenv` in a step.

> [!WARNING]
> In CI, prefer short-lived `BW_SESSION` or secret injection over long-lived passwords in environment variables, and **never** log secret values.

## Security & best practices

- **Do not commit** secrets: add `.env`, `.env.*`, `.claude/settings.local.json`, and local `.bwenv` to [`.gitignore`](https://docs.github.com/en/get-started/git-basics/ignoring-files) where appropriate.
- **Bitwarden session**: the CLI may lock the vault; `bw unlock` / `bw unlock --raw` and `bw lock` behave as in stock `bw` usage.
- **Prompt once**: the tool can persist the master password in `~/.bwenv.d/bwenv` only if you opt in; prefer **`BW_MASTER_PASSWORD` in CI** or a password manager–backed environment when possible.
- Re-export after changing vault or project settings, e.g. `bwenv -o claude` again for Claude Code.

## For maintainers

Releases are built by [`.github/workflows/release.yml`](.github/workflows/release.yml) when a SemVer tag **`v*.*.*`** is pushed and **`Cargo.toml`’s `version`** matches the tag (without the `v`).

```bash
# After bumping version in Cargo.toml and committing:
git tag v0.1.0
git push origin v0.1.0
```
