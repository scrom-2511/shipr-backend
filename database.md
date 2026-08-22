# SeaORM Database Layer Architecture & Context

This document provides complete architectural context and operational details of the database layer in the `shipr-backend` codebase following the migration from **SQLx** to **SeaORM** (v1.1).

---

## 1. Overview & Setup

- **ORM**: SeaORM `1.1` (with features `["sqlx-postgres", "runtime-tokio-rustls", "macros", "with-chrono"]`).
- **Database Engine**: PostgreSQL (Neon PostgreSQL pooler).
- **Connection Type Alias**: `crate::app::db::DbPool` maps directly to `sea_orm::DatabaseConnection`.
- **Error Mapping**: `crate::app_errors::AppError::DbErr(sea_orm::DbErr)` wraps SeaORM database errors.
- **Migration Strategy**: Native SeaORM migrations (`sea-orm-migration` crate) defined in `src/app/migrations/` using `MigratorTrait` (`m20220101_000001_create_initial_tables.rs`, `m20220102_000002_add_billing_tables.rs`). Executed on startup in `src/bin/app.rs` via `Migrator::up(&pool, None)` or via CLI binary `cargo run --bin migrate`.

---

## 2. Database Schema & Entities

The entity definitions reside in `src/app/models/mod.rs`.

### 2.1 `users` Table (`users::Model` / `User`)
- `id`: `i32` (Primary Key, Auto Increment)
- `username`: `String`
- `email`: `String` (Unique)
- `password`: `String` (Bcrypt Hashed or `github_oauth`)
- `credit_balance`: `f64` (Default: `50.0`)
- `plan_tier`: `String` (Default: `"Developer"`)
- `created_at`: `Option<DateTime>`
- `updated_at`: `Option<DateTime>`

**Relations**: Has many `projects`, `github_repos`, `billing_invoices`, `payment_methods`.

### 2.2 `projects` Table (`projects::Model` / `Project`)
- `id`: `i32` (Primary Key, Auto Increment)
- `project_id`: `String` (Unique, project slug)
- `install_cmds`: `Option<Vec<String>>`
- `run_cmds`: `Option<Vec<String>>`
- `build_cmds`: `Option<Vec<String>>`
- `branch`: `Option<String>`
- `project_type`: `Option<crate::core::app_types::ProjectType>` (SeaORM `DeriveActiveEnum`: `html`, `rust`, `react`, `node`, `unknown`)
- `full_name`: `String` (GitHub `owner/repo`)
- `dist_dir`: `Option<String>`
- `root_dir`: `String`
- `url`: `Option<String>`
- `user_id`: `i32` (Foreign Key -> `users.id`)
- `commit_hash`: `Option<String>`
- `envs`: `Option<Vec<String>>` (Encrypted JSON string array)
- `last_deployment_time`: `Option<DateTime>`
- `status`: `String` (`active`, `deploying`, `stopped`, `error`)
- `active_seconds`: `i64` (Default: `3600`)
- `created_at`: `Option<DateTime>`
- `updated_at`: `Option<DateTime>`

**Relations**: Belongs to `users`, Has many `project_traffic`.

### 2.3 `github_repos` Table (`github_repos::Model` / `GithubAppInstallation`)
- `id`: `i32` (Primary Key, Auto Increment)
- `installation_ids`: `Vec<i32>` (PostgreSQL array of GitHub App installation IDs)
- `user_id`: `Option<i32>` (Foreign Key -> `users.id`)
- `created_at`: `Option<DateTime>`
- `updated_at`: `Option<DateTime>`

**Relations**: Belongs to `users`.

### 2.4 `project_traffic` Table (`project_traffic::Model`)
- `id`: `i32` (Primary Key, Auto Increment)
- `project_id`: `i32` (Foreign Key -> `projects.id`)
- `date`: `Date` (`chrono::NaiveDate`)
- `request_count`: `i32`

**Relations**: Belongs to `projects`.

### 2.5 `billing_invoices` Table (`billing_invoices::Model`)
- `id`: `i32` (Primary Key, Auto Increment)
- `user_id`: `i32` (Foreign Key -> `users.id`)
- `invoice_number`: `String` (Unique)
- `amount`: `f64`
- `status`: `String` (`paid`, `unpaid`, etc.)
- `active_hours`: `f64`
- `rate_per_hour`: `f64`
- `period_start`: `Option<DateTime>`
- `period_end`: `Option<DateTime>`
- `created_at`: `Option<DateTime>`

**Relations**: Belongs to `users`.

### 2.6 `payment_methods` Table (`payment_methods::Model`)
- `id`: `i32` (Primary Key, Auto Increment)
- `user_id`: `i32` (Foreign Key -> `users.id`)
- `card_brand`: `String`
- `last4`: `String`
- `exp_month`: `i32`
- `exp_year`: `i32`
- `is_default`: `bool`
- `created_at`: `Option<DateTime>`

**Relations**: Belongs to `users`.

---

## 3. Operations & SeaORM Logic Across the Codebase

### 3.1 Authentication (`src/app/controllers/auth/`)
- `signup.rs`:
  - Inserts new user using `users::ActiveModel`.
  - Checks duplicate email via error matching.
- `signin.rs`:
  - Queries `users::Entity` filtering by `users::Column::Email`.
- `github_signup.rs`:
  - Queries user by email using `users::Entity::find()`.
  - Creates new user via `users::ActiveModel` if not existing.

### 3.2 Billing (`src/app/controllers/billing/`)
- `add_credits.rs`:
  - Fetches `user` via `users::Entity::find_by_id`.
  - Updates credit balance using `users::ActiveModel`.
  - Inserts invoice record using `billing_invoices::ActiveModel`.
- `get_billing_details.rs`:
  - Queries user details, projects usage (`projects::Entity`), default payment method (`payment_methods::Entity`), and invoice list (`billing_invoices::Entity`).
  - Auto-seeds default payment method or initial invoice if missing.

### 3.3 GitHub Integrations & Webhooks (`src/app/controllers/github/`, `src/app/webhooks/`)
- `update_userid_github_app_installations.rs`:
  - Executes parameterised PostgreSQL array check `UPDATE github_repos SET user_id = $1 WHERE $2 = ANY(installation_ids)` via `sea_orm::Statement`.
- `github_installation.rs`:
  - Inserts GitHub installation ID array via `github_repos::ActiveModel`.
- `github_push.rs`:
  - Checks project existence by filtering `projects::Entity` on `FullName` and `Branch`.

### 3.4 Projects (`src/app/controllers/project/`)
- `add_new_project.rs`:
  - Inserts project details using `projects::ActiveModel`.
- `check_name_availability.rs`:
  - Checks if `projects::Column::ProjectId` exists via `projects::Entity::find()`.
- `delete_project.rs`:
  - Finds project matching `Id` and `UserId`, then deletes via `ModelTrait::delete`.
- `deploy_project.rs`:
  - Validates installation ID via `github_repos::Entity`.
  - Creates new project entry using `projects::ActiveModel`.
- `edit_project_details.rs`:
  - Updates configuration (branch, root_dir, dist_dir, build/run/install cmds) via `projects::ActiveModel`.
- `get_all_deployed_projects.rs`:
  - Queries user's deployed projects ordered by creation date.
- `get_all_github_app_installed_repos.rs`:
  - Fetches user installation records via `github_repos::Entity`.
- `get_project_details.rs`:
  - Queries project detail and applies default fallback commands per `ProjectType`.
- `get_project_traffic.rs`:
  - Queries past 7 days traffic records via `project_traffic::Entity`.
- `job_completed.rs`:
  - Updates project deployment status, commit hash, branch, and timestamp via `projects::ActiveModel`.

### 3.5 Core Engine & CLI Workers (`src/core/controller/`)
- `job_dispatcher.rs`:
  - Looks up project environment variables (`envs`) via `projects::Entity`.
- `vm_request_proxy.rs`:
  - Resolves project ID/slug via `projects::Entity`.
  - Tracks traffic by incrementing `request_count` on `project_traffic::ActiveModel` or creating today's entry.
- `listen_idle_kill.rs`:
  - Updates project `active_seconds`, sets status to `'stopped'`, and updates timestamp via `projects::ActiveModel`.
- `listen_redeploy.rs`:
  - Queries matching projects for push event ref/branch via `projects::Entity`.

---

## 4. Key Traits to Import for SeaORM Queries

When writing new database queries or models, ensure these SeaORM traits are in scope:

```rust
use sea_orm::{
    EntityTrait,        // for Entity::find(), Entity::find_by_id()
    ColumnTrait,        // for Column::Name.eq(...)
    QueryFilter,        // for .filter(...)
    QueryOrder,         // for .order_by_asc(...), .order_by_desc(...)
    ActiveModelTrait,   // for active_model.insert(), active_model.update()
    ModelTrait,         // for model.delete()
    Set,                // for setting ActiveModel attributes (Set(value))
    ConnectionTrait,    // for executing raw statements
};
```

---

## 5. Migration System (`sea-orm-migration` & SeaQuery Builders)

The migration layer uses `sea-orm-migration` with type-safe SeaQuery fluent builders.

### 5.1 Architecture & Structure
- **Migrator Trait**: Defined in `src/app/migrations/mod.rs` via `pub struct Migrator` implementing `MigratorTrait`.
- **Migration Files**:
  - `src/app/migrations/m20220101_000001_create_initial_tables.rs`: Schema definitions for `users`, `github_repos`, `projects`, `project_type` Postgres enum, and `project_traffic`.
  - `src/app/migrations/m20220102_000002_add_billing_tables.rs`: Schema definitions for `billing_invoices`, `payment_methods`, and columns `credit_balance`, `plan_tier`, `active_seconds`.
- **SeaQuery Fluent Builders**:
  All migrations use type-safe identifiers and schema builders (`Table::create()`, `Table::alter()`, `pk_auto()`, `col()`, `ForeignKey::create()`, `#[derive(DeriveIden)]`).

### 5.2 Executing Migrations
- **Application Startup**: Executed automatically in `src/bin/app.rs` via `Migrator::up(&pool, None).await?`.
- **CLI Management**: Managed via CLI binary runner `src/bin/migrate.rs`:
  ```bash
  cargo run --bin migrate            # Applies all pending migrations (up)
  cargo run --bin migrate -- status   # Displays migration status
  cargo run --bin migrate -- fresh    # Drops all tables and re-applies migrations
  cargo run --bin migrate -- down     # Rolls back the latest migration batch
  ```
