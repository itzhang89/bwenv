# bwenv - Bitwarden to Environment Variables Tool

Read credentials from your [Bitwarden](https://bitwarden.com/) vault via the [`bw`](https://github.com/bitwarden/clients) CLI and print **`export ...`** lines (or `.env` / JSON) for your shell and tools.

**Repository:** [github.com/itzhang89/bwenv](https://github.com/itzhang89/bwenv)

## Requirements

- [Bitwarden CLI](https://github.com/bitwarden/clients/releases) (`bw`) installed and logged in (`bw login`).

## Installation

### One-liner (macOS / Linux)

Installs the **latest release** binary into `$HOME/.local/bin` (create the directory if needed). Requires `curl`, `python3`, `tar`, and `shasum`.

```bash
curl -fsSL https://raw.githubusercontent.com/itzhang89/bwenv/main/install.sh | bash
```

Install system-wide:

```bash
curl -fsSL https://raw.githubusercontent.com/itzhang89/bwenv/main/install.sh | sudo env BWENV_INSTALL_DIR=/usr/local/bin bash
```

Forks / mirrors: set `BWENV_GITHUB_REPO=owner/repo` if you use another GitHub repo for releases.

**Windows:** there is no install script; open the [latest release](https://github.com/itzhang89/bwenv/releases/latest), download `bwenv-*-windows-x64.zip`, unzip, and put `bwenv.exe` on your PATH.

### Latest release (manual download)

Open the **latest release** page (always points at the newest version):

**[https://github.com/itzhang89/bwenv/releases/latest](https://github.com/itzhang89/bwenv/releases/latest)**

Pick the archive for your OS (each build also has a `.sha256` file you can check with `shasum -a 256 -c`):

| Suffix in filename | Platform |
|--------------------|----------|
| `linux-x64` | Linux x86_64 (glibc) |
| `darwin-arm64` | macOS Apple Silicon |
| `darwin-x64` | macOS Intel |
| `windows-x64` | Windows x86_64 (`.zip`) |

Example after download (Linux x64 — replace the file name with the one you downloaded):

```bash
tar -xzf bwenv-0.1.0-linux-x64.tar.gz
chmod +x bwenv
mv bwenv ~/.local/bin/   # or /usr/local/bin with sudo
```

### Build from source

```bash
git clone https://github.com/itzhang89/bwenv.git
cd bwenv
cargo build --release
cp target/release/bwenv /usr/local/bin/   # or ~/.local/bin
```

## Quick Start

```bash
# 1. Add a project
bwenv project add dev developer "mysql,redis"

# 2. Use the project
bwenv use dev
```

## How It Works

### Bitwarden Folder Structure

The tool uses Bitwarden's **Folder** feature to organize credentials. The folder name acts as a prefix filter.

```
Bitwarden Vault
├── developer/         (Folder)
│   ├── mysql          (Login item)
│   ├── redis          (Login item)
│   └── github         (Login item)
├── project1/          (Folder)
│   ├── aliyun         (Login item)
│   └── aws            (Login item)
└── database/          (Folder)
    └── postgres       (Login item)
```

### Configuration Format

In `~/.bwenv.d/bwenv`, define projects as below.

#### About Bitwarden session (`BW_SESSION`)

Bitwarden CLI returns a **session key** after unlock (`bw unlock --raw`). This session is what authorizes subsequent `bw` commands (e.g. `bw list items`) without re-entering your master password.

`bwenv` handles session like this:

- **Cache location**: `~/.bwenv.d/session` (plain text), permission best-effort set to `0600` on Unix.
- **Priority**: if `BW_SESSION` environment variable is set (non-empty), `bwenv` uses it first and also persists it to `~/.bwenv.d/session`.
- **Runtime strategy**: `bwenv` will optimistically run `bw` commands assuming the vault is already unlocked. If a command fails with an auth/locked/session-expired style error, it runs the unlock flow once to refresh the session and retries the command once.
- **Auto refresh**: if the cached session is **expired/invalid**, `bwenv` clears the cache and falls back to normal `bw status`/unlock flow; if you provide `BW_MASTER_PASSWORD`, it will re-run `bw unlock --raw` and write a fresh session back to `~/.bwenv.d/session`.
- **How `bw` consumes it**: `bw` supports either `--session <key>` (what `bwenv` uses) or exporting `BW_SESSION=<key>` in the environment.

```yaml
# ~/.bwenv
- name: "dev"
  prefix: "developer"    # Matches Bitwarden folder name
  services:
    - mysql
    - redis
    - github

- name: "prod"
  prefix: "project1"
  services:
    - aliyun
    - aws
```

### Output Examples

For a Bitwarden item like:

- Folder: `developer`
- Item name: `mysql`
- Username: `admin`
- Password: `secret123`

The tool generates environment variables:

```bash
# Shell format
export MYSQL_USER="admin"
export MYSQL_PASSWORD="secret123"

# .env format
MYSQL_USER=admin
MYSQL_PASSWORD=secret123

# JSON format
{
  "MYSQL_USER": "admin",
  "MYSQL_PASSWORD": "secret123"
}
```

## Usage

### Shell Integration

You can directly load credentials into your current shell session:

```bash
# Using eval (recommended)
eval "$(bwenv)"

# Or using process substitution
source <(bwenv)

# Load specific project
eval "$(bwenv use dev)"

# Load with filters
eval "$(bwenv -p developer -s mysql)"
```

In an **interactive** terminal, plain `bwenv` (no args) prints help. Commands like `eval "$(bwenv)"` still run the generator, because the subprocess stdout is a pipe, not a TTY—avoiding zsh `eval` errors on text such as `[OPTIONS]`.

#### Permanent Shell Setup

**Zsh** (add to `~/.zshrc`):

```zsh
# Load bwenv credentials on shell startup
eval "$(bwenv use dev)"

# Or with auto-detection from .bwenv file
eval "$(bwenv)"
```

**Bash** (add to `~/.bashrc` or `~/.bash_profile`):

```bash
# Load bwenv credentials on shell startup
eval "$(bwenv use dev)"

# Or with auto-detection from .bwenv file
eval "$(bwenv)"
```

> **Note**: This will prompt for your Bitwarden master password if not already unlocked. Consider using `bw unlock --persist` first for faster startup.

### Commands

```bash
# Generate environment variables (default)
bwenv                          # Use current project
bwenv -o .env                  # Export to file
bwenv -s github                # Filter by service
bwenv -p developer             # Filter by prefix
bwenv -f json                  # Output format

# Use project
bwenv use dev                  # Switch project (no output)
bwenv use dev -o .env          # Switch and export to file

# Project management
bwenv project                  # List projects
bwenv project add dev developer "mysql,redis"  # Add project (omit last arg for all services)
bwenv project remove dev       # Remove project
bwenv project load ~/.bwenv    # Load from file

# Other commands
bwenv list                     # List Bitwarden items
bwenv list --folders           # List all Bitwarden folders
bwenv current                  # Show current project
bwenv config show              # Show configuration
```

### Claude Code Integration

Export environment variables directly to Claude Code project settings:

```bash
# Add env vars to .claude/settings.local.json
bwenv -p developer -s mysql -o claude

# Remove env vars from Claude Code
bwenv -o claude:remove
```

This creates `.claude/settings.local.json`:

```json
{
  "_bwenv": {
    "dev": ["MYSQL_USER", "MYSQL_PASSWORD"]
  },
  "env": {
    "MYSQL_USER": "admin",
    "MYSQL_PASSWORD": "secret123"
  }
}
```

> **Security Note**: Add `.claude/settings.local.json` to `.gitignore` to prevent sensitive data from being committed.

### Project Directory .bwenv File

Create a `.bwenv` file in your project directory for auto-detection:

```yaml
# project/.bwenv
name: "myproject"
prefix: "developer"
services:
  - mysql
  - redis
```

When running `bwenv` in that directory or its subdirectories, the project will be auto-detected.

## Configuration

### Master Password Priority

1. Environment variable `BW_MASTER_PASSWORD`
2. Configuration file
3. Runtime input

### Environment Variables


| Variable             | Description               |
| -------------------- | ------------------------- |
| `BW_MASTER_PASSWORD` | Bitwarden master password |


## Examples

### Development Environment

```bash
# Export to .env file
bwenv use dev -o .env
source .env

# Or use eval
eval $(bwenv -p developer -s mysql)
```

### Docker Compose

```bash
bwenv use prod -o .env
```

### CI/CD

```bash
export BW_MASTER_PASSWORD="$BW_MASTER_PASSWORD"
bwenv use prod -f json > secrets.json
```

## Best Practices

### Security

- **Never commit secrets**: Add `.env`, `.claude/settings.local.json`, and any local config files to `.gitignore`
  ```gitignore
  # .gitignore
  .env
  .env.*
  .claude/settings.local.json
  .bwenv
  ```
- **Use session timeout**: Bitwarden CLI locks after inactivity. Use `bw lock` in longer workflows:
  ```bash
  bw unlock --persist # Remember session for this terminal
  ```
- **Prefer environment variable for master password**: More secure than storing in config file:
  ```bash
  export BW_MASTER_PASSWORD="your-master-password"
  bwenv use dev
  ```

### Project Organization

- **Use descriptive folder names**: Match Bitwarden folders to your project/environment names
  ```
  Bitwarden Folders: dev, staging, prod, personal
  ```
- **Use consistent naming**: Keep service names lowercase with underscores:
  ```yaml
  services:
    - mysql_db      # Good
    - mysql         # Also good
    - MySQL         # Avoid
  ```
- **Leverage per-project `.bwenv` files**: Store project-specific config in each project directory for auto-detection

### Workflow

- **Quick lookup**: Use `bwenv list` to verify Bitwarden items before exporting
- **Incremental export**: Filter by service when you only need specific credentials:
  ```bash
  bwenv -s mysql          # Only MySQL credentials
  bwenv -s mysql,redis    # Multiple services
  ```
- **Validate before use**: Preview output before writing to files

### Claude Code Integration

- **Keep credentials synced**: After updating Bitwarden, refresh Claude Code settings:
  ```bash
  bwenv -p developer -s mysql -o claude
  ```
- **Track which vars are managed**: The `_bwenv` field in settings shows which variables come from bwenv

### Maintenance

- **Regular cleanup**: Remove unused items from Bitwarden folders
- **Audit access**: Periodically check which projects have `.bwenv` files in your directories
- **Test in dev first**: Always test credential export in development before staging/production

## For maintainers

Publishing a release is automated by [`.github/workflows/release.yml`](.github/workflows/release.yml): push a SemVer tag `v*.*.*` whose version (without `v`) matches `version` in `Cargo.toml`. Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

