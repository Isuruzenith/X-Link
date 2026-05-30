# Copilot instructions for TunX

## Build, test, lint
- Frontend build: `npm run build`
- Frontend lint: `npm run lint`
- Tauri app build (bundles frontend + Rust): `npm run tauri build`
- Rust tests (run from `src-tauri`): `cargo test`
- Single Rust test: `cargo test test_adapt_raw_uris` (or any test name)

## High-level architecture
- React/Vite frontend lives in `src/`; `App.tsx` is the main UI and calls the backend via `@tauri-apps/api` `invoke` and event `listen`.
- Rust backend lives in `src-tauri/src/`. `lib.rs` wires Tauri plugins, manages `ProxyState`, and registers commands in `src-tauri/src/commands/`.
- The backend starts a bundled `sing-box` sidecar (`binaries/sing-box`) from `commands/proxy.rs`, validates configs via `config/validator.rs`, and writes per-profile configs to the app data dir under `configs/{profile_id}.json` (see `config/mod.rs`).
- Settings and profiles are persisted via `tauri-plugin-store` as `settings.json` and `profiles.json` in the app data dir; Rust reads those files directly to configure ports, proxy mode, DNS/SNI, and tray behavior. `tauri.conf.json` defines the dev/build wiring and bundles `binaries/wintun.dll` on Windows.

## Key conventions
- Tauri command names are snake_case (`#[tauri::command]`) and must match the strings used by `invoke(...)` in `App.tsx`.
- Serialized structs from Rust use camelCase (e.g., `Profile`, `TrafficStats`); keep frontend interfaces in `src/utils/store.ts` aligned.
- When changing settings keys (e.g., `proxyMode`, `closeToTray`, ports), update both `src/utils/store.ts` and the Rust code that reads `settings.json` (several commands and `lib.rs`).
- Sidecar logging flows through `ProxyState`’s ring buffer and the `sing-box-log` / `sing-box-terminated` events; frontend listens to these and also calls `get_buffered_logs` on startup.
