# Testing Resource Manager

Summarizing **what is tested** and **why each test method was chosen**.

---

## Methods

| Method | Used for | Why it fits                                                                                               |
| :--- | :--- |:----------------------------------------------------------------------------------------------------------|
| **Unit tests (pure logic)** | Domain utilities & parsers | Fast, deterministic, side‑effect free.                                                                    |
| **State-based tests** | Business actions | Ensures multi-struct state transitions (e.g., `RMState` + `AppConfig`) stay consistent.                   |
| **FS-simulated tests** | Infrastructure / detection | Validates file/path heuristics safely using temp directories (`TempDir`).                                 |
| **Parsing-focused tests** | Import / UI helpers | Import entry points mainly transform user input → data, we validate transformations without UI rendering. |

---

## Coverage

### Domain (Unit)

- **`game.rs`: version model**
  - Channel classification (release vs snapshot)
  - Stable ordering across formats (e.g., `1.20 < 1.20.1`; snapshots don’t outrank unrelated releases)

- **`project.rs`: project metadata & dependencies**
  - Effective dependency type resolution (`effective_*`) matches override rules
  - Derived names/filenames are deterministic and filesystem-safe

- **`project_list.rs`: list invariants**
  - Dependency → project promotion preserves links and semantics
  - Archiving/removal respects “still required” constraints; invariants remain valid

### Business (State-based)

- **`list_actions.rs`: list operations**
  - Instance settings inheritance / application stays consistent
  - Hierarchical operations (move/delete/update) don’t orphan children; ordering is updated

- **`list_group_actions.rs`: group tree operations**
  - Duplication produces new links/IDs (no aliasing of original nodes)
  - Parent/child relationships remain consistent and UI order stays in sync

### Infrastructure (FS-simulated)

- **`resource_detector.rs`: detection heuristics**
  - Version hints are extracted from real-world filenames
  - Normalization (spaces, `+`, URL-encoding) doesn’t change detection results

### Import / UI helpers (Parsing-focused)

- **`lists_manager.rs`: MMD import**
  - TOML/MMD deserialization yields valid domain objects
  - IDs/paths are sanitized (no collisions; sensible fallbacks)

- **`import_modal.rs` / `legacy_import_modal.rs`: name extraction**
  - Extensions are stripped correctly
  - Edge cases (hidden files, empty input, trailing separators) return safe defaults

- **`modrinth_collection_import_modal.rs`: URL parsing**
  - Query params/fragments are ignored
  - Valid collection IDs are extracted; invalid input is rejected cleanly

---

## Quality Properties

- **Deterministic:** no reliance on real user data or global state
- **Isolated:** each test owns its state/temporary directory
- **Regression-oriented:** covers the highest-risk flows (group/list operations + imports)
