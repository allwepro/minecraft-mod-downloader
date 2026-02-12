# Launcher Compatibility Matrix (Java + Minecraft)

This document defines the launcher's **known-good Java/Minecraft combinations** and the update policy.

## Current Matrix Policy

The launcher maps **Minecraft version -> recommended Java major** using release versions only.

- `1.0` to `1.16.x` -> Java `8`
- `1.17.x` to `1.19.x` -> Java `17`
- `1.20.0` to `1.20.4` -> Java `17`
- `1.20.5` and newer `1.x` releases -> Java `21`
- `2.x+` (future major) -> Java `21` (default until updated)

Non-release versions (snapshots / pre-releases like `23w12a`, `1.21-pre1`) are treated as **unknown**.

## Launcher Behavior

- If selected Java is outside the matrix, launch is blocked by default.
- User can explicitly enable **Allow experimental launch** to bypass.
- Fabric launch still requires Fabric support/readiness checks.

## Source of Truth in Code

- Matrix logic: `src/launcher/ui/launcher_panel.rs` in `recommended_java_major_for_mc_version`.
- Preflight gating: `src/launcher/ui/launcher_panel.rs` in `launch_disabled_reasons` and `evaluate_launch_preflight`.

## Update Policy

When Mojang or Java requirements change:

1. Update matrix logic in `recommended_java_major_for_mc_version`.
2. Update matrix tests in `launcher_panel.rs` (`recommended_java_major_follows_matrix_rules`).
3. Add/adjust preflight tests if behavior changes (blocking/allowing paths).
4. Validate with:
   - `cargo test 'launcher::'`
5. If behavior changes for users, mention it in release notes/PR summary.

## Why This Policy

- Keeps launcher behavior predictable for normal users.
- Still allows advanced users to try unsupported combinations.
- Makes matrix updates explicit and test-backed.
