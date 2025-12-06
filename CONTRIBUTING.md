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

The repository is organized into **layers**. This keeps GUI, core logic, and external IO cleanly separated.

### 📁 Folder Structure
```
src/
├── app/                      # GUI layer
│   ├── mod.rs
│   ├── window.rs
│   └── components/
│
├── core/                     # business logic (pure, no IO)
│   ├── mod.rs
│   ├── compatibility.rs
│   ├── manifest.rs
│   └── downloader.rs
│
├── infra/                    # external side‑effects (API, FS, HTTP)
│   ├── mod.rs
│   ├── modrinth_api.rs
│   ├── fs.rs
│   └── http.rs
│
├── common/                   # shared data models and types
│   ├── mod.rs
│   ├── mod_info.rs
│   └── version.rs
│
├── utils/                    # utilities
│   └── utils.rs
│
└── main.rs
```

### 🔄 Execution Flow
```
GUI → core (service functions) → infra (API/FS) → core → GUI updates
```

This architecture ensures:
- GUI does not perform IO
- core contains pure logic
- infra handles all external side‑effects

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
- One feature per PR
- After approval → **Rebase & merge**

---

## 9️⃣ Additional Notes
- Code failing formatting or linting will be rejected by CI
- `main` is protected — no direct pushes allowed