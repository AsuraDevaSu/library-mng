# Library Management

Rust full-stack library system: **Axum + SQLx + PostgreSQL** backend, **Leptos** (SSR + hydration) web UI,
and a REST API at `/api/v1` for the future **Flutter** mobile app. One binary serves everything.

## Layout

```
crates/
  domain/    entities, DTOs (validator), business rules — pure, compiles to WASM
  infra/     config, DB pool + migrations, repositories, auth (argon2/JWT), circulation service
  api/       REST routes /api/v1 (bearer JWT) — the contract for Flutter
  app/       Leptos components + #[server] functions (features: ssr | hydrate)
  frontend/  WASM entry point (hydrate)
  server/    main.rs: Axum router = Leptos routes + /api/v1
migrations/  SQL migrations (embedded into the binary, applied on startup)
style/       tailwind.css — Tailwind v4 + shadcn/ui theme tokens (Positivus colours)
```

### UI: shadcn/ui for Leptos

shadcn/ui is React-only, so `crates/app/src/ui.rs` ports it: the same token names
(`--primary`, `--accent`, `--muted`, `--border`, `--radius`…) and component recipes
(`Button` with variants/sizes, `Card`, `Badge`, `Alert`, `Table`, `INPUT`/`LABEL` classes).
The theme follows the Positivus Figma design: lime `#B9FF66`, ink `#191A23`, grey `#F3F3F3`,
Space Grotesk, dark outlines with an offset "brutal" shadow, 45px marketing cards.
cargo-leptos runs Tailwind automatically (`tailwind-input-file` in `Cargo.toml`).

Rule of thumb: business decisions go in `domain::rules` (unit-tested, no IO);
transactions and SQL go in `infra`; HTTP shapes go in `api` / `app`.

## Run

```bash
docker compose up -d db          # Postgres on localhost:5434
cp .env.example .env             # then set JWT_SECRET and ADMIN_PASSWORD
cargo leptos watch               # http://127.0.0.1:3000, live reload
```

Startup applies migrations and creates the admin from `ADMIN_EMAIL` / `ADMIN_PASSWORD`
if there are no users yet.

```bash
cargo test --workspace --exclude frontend
```

## API (v1)

| Method | Path | Auth | |
|---|---|---|---|
| GET  | `/api/v1/health` | – | DB check |
| POST | `/api/v1/auth/login` | – | `{email, password}` → JWT |
| GET  | `/api/v1/me` | staff | current user |
| GET  | `/api/v1/books?q=&limit=&offset=` | – | catalog search (title, author, ISBN) |
| POST | `/api/v1/books` | librarian | book + authors + copies |
| GET  | `/api/v1/books/{id}` | – | book with copies |
| GET/POST | `/api/v1/members` | librarian | search / register |
| POST | `/api/v1/circulation/checkout` | librarian | `{card_number, copy_barcode}` |
| POST | `/api/v1/circulation/return` | librarian | `{copy_barcode}` → loan + fine |
| POST | `/api/v1/loans/{id}/renew` | librarian | |

Errors are always `{"error": {"code", "message", "fields?"}}` (422 validation, 409 rule violations / conflicts).

## Web UI

| Path | Who | |
|---|---|---|
| `/books` | everyone | catalog search |
| `/login` | staff | HttpOnly cookie session (same JWT as the API) |
| `/admin` | admin | dashboard: stock, loans, overdue, unpaid fines, recent activity |
| `/admin/staff` | admin | add staff, change role, enable/disable, reset password |
| `/admin/policies` | admin | borrowing rules per member type (limits, loan days, fines) |
| `/admin/settings` | admin | library name/hours, bag-desk photo retention & face-match threshold |
| `/admin/audit` | admin | searchable audit log |

**Roles** are hierarchical: `admin` ⊇ `librarian` ⊇ `desk_staff` (reading room / bag desk).
Each request loads the user from the DB, so disabling an account or changing a role
takes effect immediately, not when the token expires. The last active admin can't be
demoted or disabled, and admins can't demote or disable themselves.

Every server function checks the role on the server (`auth::ssr::require`) — the
UI redirect is convenience, not security.

## Design notes

- **Double checkout** is prevented by an atomic `UPDATE copies … WHERE status = 'available'`
  plus a partial unique index (one open loan per copy). The member row is locked during checkout
  so the loan limit can't be raced.
- **Money** is `NUMERIC(10,2)` ↔ `rust_decimal::Decimal`, serialized as strings (`"1.25"`).
- **Dates**: due dates are `DATE` in the library timezone (`TZ` env var).
- **Audit log**: every staff write records a row in `audit_logs` in the same transaction.

## Next

- [x] Staff login (cookie session) + admin module
- [ ] Librarian pages: checkout/return desk (barcode input), members, add book
- [ ] Bag deposit desk (camera, face match, tags) + reading-room visits
- [ ] Login rate limiting; forced password change after admin reset
- [ ] Reservations / hold queue (return → `on_hold_shelf`, block renewals)
- [ ] Fine payments & waivers, overdue report
- [ ] Background job: overdue reminders by email (`lettre`, Mailpit in dev)
- [ ] Integration tests with `#[sqlx::test]`
- [ ] Flutter member app against `/api/v1`
