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

### 3️⃣ Project Structure & Architecture

The repository is organized into modules that separate shared UI components from feature-specific logic. We follow a pattern that decouples the visual interface from business rules and external integrations.

#### 📁 Folder Structure (Overview)
```text
src/
├── main.rs               # Application entry point
│
├── common/               # Shared UI Framework
│   │                     # Global components like modals and notifications
│   └── prefabs/          # Reusable UI templates (Window wrappers, ViewControllers)
│
└── resource_downloader/  # Main Feature Module
    ├── app/              # UI Layer (The "View")
    │   ├── panels/       # Persistent UI sections (Sidebar, Main Panel)
    │   ├── modals/       # Interactive overlays (Search, Settings, Import)
    │   └── components/   # Small, reusable feature-specific widgets
    │
    ├── business/         # Application Logic (The "Brain")
    │   ├── rd_state.rs   # State management and event definitions
    │   ├── services/     # Async task pools and API orchestrators
    │   └── cache/        # Logic for data persistence and retrieval
    │
    ├── domain/           # Core Entities (The "Model")
    │   └── project.rs    # Definitions for Projects, Games, and Lists
    │
    └── infra/            # Infrastructure & IO (The "Hands")
        ├── adapters/     # External API clients (e.g., Modrinth)
        ├── rd_runtime.rs # Async runtime and task execution
        └── lists_manager.rs # Filesystem and config persistence
```

### 🔄 Execution Flow
Our architecture follows a unidirectional flow to keep the state predictable:

1.  **UI (app):** User triggers an action (e.g., clicks "Download").
2.  **Business:** The event is processed; state is updated or an "Effect" is scheduled.
3.  **Infra/Adapters:** External calls are made (API requests, File IO).
4.  **Domain:** Data is validated and structured according to business rules.
5.  **UI Updates:** The state change ripples back to the UI for re-rendering.

This separation ensures that:
- 🎨 **UI code** handles only layout and styling.
- ⚙️ **Business logic** remains independent of the specific UI framework.
- 🔌 **Infrastructure** isolates side effects like web requests and disk access.
- 🛠 **Common** provides a consistent look and feel across different app modules.

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