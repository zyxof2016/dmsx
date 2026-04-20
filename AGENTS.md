# DMSX Agent Notes

## Repo Shape

- This repo is two projects, not one toolchain: the Rust workspace lives at the root (`Cargo.toml`, `crates/*`), and the frontend is a separate Vite app in `web/` with `npm`/`package-lock.json`.
- The current runtime path is `dmsx-api` + `dmsx-agent`. `dmsx-device-gw` exists, but `docs/ARCHITECTURE.md` marks it as an evolving data-plane skeleton, not the current default Agent communication path.

## Source Of Truth

- Prefer code/config/scripts over prose when they disagree. Example: `web/src/api/client.ts` is the current frontend auth/tenant behavior.
- Do not edit or trust `web/src/**/*.js`. `.gitignore` excludes them, and they are transpiled leftovers that can be stale relative to `.ts`/`.tsx`.

## Verified Commands

- Match CI with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `cargo build --workspace`, `cargo test --workspace`. CI also sets `RUSTFLAGS="-D warnings"`.
- `protobuf-compiler` is required for workspace builds because `crates/dmsx-device-gw/build.rs` compiles `proto/**/*.proto`.
- Fast local regression: `./scripts/internal-beta-verify.sh` runs `cargo test -p dmsx-api --lib` and `cargo test -p dmsx-agent --lib`.
- Preferred higher-level verification: `./scripts/internal-beta-dod.sh`. It runs the lib tests first, then the HTTP smoke flow. Use `DMSX_DOD_SKIP_SMOKE=1` only when API/smoke prerequisites are intentionally unavailable.
- For JWT + multi-tenant changes, run `./scripts/public-beta-multi-tenant-smoke.sh` against a running `dmsx-api` with `DMSX_API_AUTH_MODE=jwt` and a matching `DMSX_API_JWT_SECRET`.
- For real Agent behavior changes, run `DMSX_E2E_API="http://127.0.0.1:8080" ./scripts/agent-dev-e2e.sh`.
- Focused RLS check: `DMSX_TEST_DATABASE_URL="postgres://dmsx:dmsx@127.0.0.1:5432/dmsx" cargo test -p dmsx-api --test rls_tenant_session`.

## Env And Migrations

- `REPRODUCE_MINIMAL=1 ./scripts/reproduce-dev-env.sh` is the quickest way to stand up the minimum local dependency set (Postgres on host `127.0.0.1:5432`).
- `REPRODUCE_TAKE_PORT_5432=1` is destructive for local containers: `scripts/reproduce-dev-env.sh` will stop every container publishing host port `5432` before starting this repo's Postgres.
- `dmsx-api` runs embedded `sqlx::migrate!("../../migrations")` at startup. After changing any `migrations/*.sql`, rebuild `dmsx-api` before starting it, or the new migration SQL will not be embedded/applied.

## Frontend Gotchas

- Use `npm`, not `pnpm`, in `web/`. The repo has `web/package-lock.json`; docs mentioning `pnpm` are stale.
- Verified frontend commands are `cd web && npm install && npm run dev` and `cd web && npm run build`.
- `npm run build` already includes typechecking via `tsc --noEmit && vite build`.
- Vite proxies `/v1` and `/health` to `http://127.0.0.1:8080` (`web/vite.config.ts`).
- The frontend persists the active tenant in `localStorage["dmsx.tenant_id"]`; tenant-scoped UI/API changes usually flow through `web/src/appProviders.tsx` and `web/src/api/hooks.ts` rather than a hard-coded constant.
- The frontend sends `Authorization` only if `localStorage.getItem("dmsx.jwt")` is set; it auto-prefixes `Bearer ` when needed, and the header/tenant values are managed from the top-bar session UI.
- `web/src/main.tsx` must keep `RouterProvider` inside `AppProviders`; several pages and `App.tsx` rely on those contexts at runtime.
- Do not assume frontend lint is actually wired up just because `npm run lint` exists: `web/package.json` defines it, but the repo does not check in an ESLint config or ESLint dependency.

## Docs Rule

- `.cursor/rules/docs-update-required.mdc` is mandatory: if you change visible behavior or external contracts, update docs in the same change.
- Route/API/auth/config/probe changes usually require some combination of `README.md`, `docs/CHECKLIST.md`, `docs/API.md`, `docs/FRONTEND.md`, `docs/SECURITY.md`, `openapi/dmsx-control-plane.yaml`, and `deploy/*`.
