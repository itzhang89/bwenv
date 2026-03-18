# bwenv - Bitwarden to Environment Variables Tool

A CLI tool to read credentials from Bitwarden vault and convert them to environment variables.

## Installation

```bash
# Clone the project
git clone <repo-url>
cd bwenv

# Build
cargo build --release

# Install to PATH
cp target/release/bwenv /usr/local/bin/
```

## Requirements

- [Bitwarden CLI](https://github.com/bitwarden/clients/releases) (`bw`) must be installed
- First time use: run `bw login` to login

## Quick Start

```bash
# 1. Add a project
bwenv project add dev "mysql,redis" developer

# 2. Use the project
bwenv use dev
```

## How It Works

### Bitwarden Folder Structure

The tool uses Bitwarden's **Folder** feature to organize credentials. The folder name acts as a prefix filter.

```
Bitwarden Vault
├── developer/          (Folder)
│   ├── mysql          (Login item)
│   ├── redis          (Login item)
│   └── github         (Login item)
├── thoughtworks/      (Folder)
│   ├── aliyun         (Login item)
│   └── aws            (Login item)
└── database/          (Folder)
    └── postgres       (Login item)
```

### Configuration Format

In `~/.bwenv`, define projects with:

```yaml
# ~/.bwenv
- name: "dev"
  prefix: "developer"    # Matches Bitwarden folder name
  services:
    - mysql
    - redis
    - github

- name: "prod"
  prefix: "thoughtworks"
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
bwenv project add dev "mysql,redis" developer  # Add project
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

| Variable | Description |
|----------|-------------|
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
