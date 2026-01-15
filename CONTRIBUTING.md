# 🧩 Contributing Guide

This project is developed collaboratively as a **university group project**.

---

## 1️⃣ Prerequisites
- Install Git: https://git-scm.com/downloads
- Install Rust: https://rustup.rs/
- Have collaborator access to the repository.

---

## 2️⃣ Set up your environment
```bash
git clone https://github.com/allwepro/minecraft-mod-downloader.git
cd minecraft-mod-downloader
```

Create a new branch:
```bash
git checkout -b feature/<short-description>
```

Examples:
- feature/gui-setup
- fix/download-crash

---

## 3️⃣ Project Structure & Architecture

The repository is organized into **logical layers** to keep responsibilities clearly separated and the codebase easy to maintain.

### 📁 Folder Structure (Overview)
```
src/
├── main.rs               # Application entry point
│
├── adapters/             # External service adapters
│   └── modrinth.rs       # Modrinth API adapter
│
├── app/                  # Application layer
│   ├── app_state.rs      # Global application state
│   ├── runtime.rs        # Event loop & task orchestration
│   └── effect.rs         # Side-effect definitions
│
├── domain/               # Core domain logic
│   ├── mod_service.rs    # Resource handling logic
│   └── mod_source.rs     # Abstract adapter interface
│
├── infra/                # Infrastructure & side effects
│   ├── api_service.rs    # HTTP / API handling
│   ├── config_manager.rs # Configuration & persistence
│   └── project_cache.rs  # Local caching
│
└── ui/                   # GUI layer
    ├── dialogs.rs        # Common dialogs
    ├── view_state.rs     # UI state definitions
    ├── panels/           # Main UI panels
    │   └── main_panel.rs 
    └── windows/          # Application windows
        └── search_window.rs
```

> This is a simplified overview.

---

### 🔄 Execution Flow
```
UI → app (state & effects) → domain (business logic)
   → adapters / infra (API, FS, cache)
   → domain → app → UI updates
```

This architecture ensures:
- 🖼 UI code focuses purely on presentation
- 🧠 Domain logic remains pure and easy to test
- 🔌 Infrastructure handles all external side effects
- 🌐 Adapters isolate third‑party services like Modrinth

---

## 4️⃣ Make Changes

- Open the project in your editor.
- Run frequently:
  ```bash
  cargo build
  cargo run
  cargo test
  ```
- Format check:
  ```bash
  cargo fmt --check
  ```

---

## 5️⃣ Commit and Push
```bash
git add .
git commit -m "Short summary of changes"
git push origin feature/<branch-name>
```

Then open a **Pull Request** to `dev`.

---

## 6️⃣ Code Quality (CI Requirements)

All PRs are validated by GitHub Actions:

- 🧹 Format check (`cargo fmt --check`)
- 🧠 Linting (`cargo clippy`)
- 🧱 Build check
- ⚙️ Test runner
- 🔒 Security audit (`cargo audit`)

All checks must pass before merging.

---

## 7️⃣ Branching Conventions

| Branch Type   | Purpose                       |
|---------------|-------------------------------|
| `main`        | Clean, production-ready       |
| `dev`         | Integration branch            |
| `feature/*`   | New feature or module         |
| `fix/*`       | Bug fix                       |
| `chore/*`     | Maintenance work              |
| `docs/*`      | Documentation changes         |

---

## 8️⃣ Pull Requests
- Tag teammates for review
- After approval → **Rebase & merge**

---

## 9️⃣ Additional Notes
- Code failing formatting or linting will be rejected by CI
- `main` is protected — no direct pushes allowed