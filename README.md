# Patchyx - Pijul Cloud Platform

> A cloud platform for hosting Pijul repositories, similar to GitHub but for Pijul VCS.

[![License: GPL-2.0](https://img.shields.io/badge/License-GPL%202.0-blue.svg)](LICENSE)

## Overview

Patchyx is a self-hosted platform for Pijul repositories, providing:

- **SSH Protocol Gateway** - Push and pull over SSH (`pijul push/pull`)
- **HTTP REST API** - Repository management and web UI backend
- **Multi-Repository Hosting** - Host unlimited Pijul repos

Built on top of the core Pijul libraries (`libpijul`, `pijul-remote`, etc.).

## Quick Start

```bash
# Build the server
cargo build -p patchyx-server

# Run with defaults (SSH:2222, HTTP:3000)
./target/debug/patchyx-server

# Or configure via environment
PATCHYX_SSH_PORT=22 PATCHYX_HTTP_PORT=80 ./target/debug/patchyx-server
```

## Configuration

| Environment Variable    | Default    | Description                  |
| ----------------------- | ---------- | ---------------------------- |
| `PATCHYX_SSH_HOST`      | 0.0.0.0    | SSH bind address             |
| `PATCHYX_SSH_PORT`      | 2222       | SSH port                     |
| `PATCHYX_HTTP_HOST`     | 127.0.0.1  | HTTP bind address            |
| `PATCHYX_HTTP_PORT`     | 3000       | HTTP port                    |
| `PATCHYX_REPOS_DIR`     | ./repos    | Repository storage directory |
| `PATCHYX_HOST_KEY_PATH` | ./host_key | SSH host key file            |
| `PATCHYX_LOG_LEVEL`     | info       | Logging level                |

## Project Structure

```
pijul/
├── libpijul/           # Core Pijul library
├── pijul-config/       # Configuration handling
├── pijul-identity/     # Identity management
├── pijul-remote/       # Remote protocol handling
├── pijul-repository/   # Repository utilities
├── pijul-macros/       # Procedural macros
└── patchyx-server/     # 🚀 Cloud server (this project)
    ├── src/
    │   ├── main.rs     # Entry point
    │   ├── config.rs   # Configuration
    │   ├── error.rs    # Error types
    │   ├── ssh/        # SSH protocol
    │   └── http/       # REST API
    └── Cargo.toml
```

## Roadmap

### ✅ Completed

- [x] Project cleanup (removed CLI, refactored dependencies)
- [x] Server scaffold with SSH and HTTP
- [x] Environment-based configuration
- [x] Custom error types
- [x] Structured logging
- [x] Pijul command parsing (clone/pull/push/ping)
- [x] Health check and repo listing endpoints
- [x] Graceful shutdown

### 🔄 In Progress

- [ ] Fix `libpijul` compilation (sanakirja dependency issue)
- [ ] Integrate `libpijul::Pristine` for repository operations

### 📋 TODO

- [ ] **Authentication**

  - [ ] SSH public key verification against user database
  - [ ] HTTP API token authentication
  - [ ] OAuth2 integration (GitHub, GitLab)

- [ ] **Repository Management**

  - [ ] Create/delete repositories
  - [ ] Access control (public/private)
  - [ ] Channel browsing
  - [ ] Change history viewing

- [ ] **Protocol Implementation**

  - [ ] Full `pijul clone` over SSH
  - [ ] Full `pijul push` with change application
  - [ ] Full `pijul pull` with change streaming

- [ ] **Web Interface**

  - [ ] Repository browser UI
  - [ ] File content viewer (reconstruct from patches)
  - [ ] Change diff viewer
  - [ ] User dashboard

- [ ] **Database Integration**

  - [ ] PostgreSQL for user/repo metadata
  - [ ] User registration and management
  - [ ] Repository permissions

- [ ] **DevOps**
  - [ ] Docker containerization
  - [ ] Kubernetes deployment manifests
  - [ ] CI/CD pipeline

## Known Issues

### `libpijul` Compilation Error

The `sanakirja` database library (version 2.0.0-beta) has dependency resolution issues. The crate on crates.io appears incompatible with `libpijul` beta.11.

**Workarounds being explored:**

1. Patch `sanakirja` from git source
2. Vendor a working version
3. Wait for upstream fix

## API Endpoints

| Method | Endpoint        | Description       |
| ------ | --------------- | ----------------- |
| GET    | `/`             | Server info       |
| GET    | `/health`       | Health check      |
| GET    | `/api/v1/repos` | List repositories |

## Contributing

Contributions welcome! This is a work in progress.

```bash
# Format code
cargo fmt

# Run checks
cargo check -p patchyx-server

# Run tests (when available)
cargo test -p patchyx-server
```

## License

GPL-2.0 (inherited from Pijul)

---

## About Pijul

Pijul is a distributed VCS based on a mathematical theory of patches. Unlike Git's snapshot model, Pijul models history as a DAG of commutative changes. This makes merging trivial and conflict resolution persistent.

Learn more: [pijul.org](https://pijul.org)
