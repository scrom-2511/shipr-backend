# Shipr: Job Completion & Project Type Detection Context

This document summarizes the implementation details for the "Job Completion" signal and "Project Type Detection" features.

## Overview
The goal was to enable the build worker to signal the main application server when a deployment or redeployment job is finished. This signal carries critical metadata like the final commit hash, the detected project type, and the job type (Deploy vs Redeploy), which are then persisted in the database.

## Architecture & Flow

### 1. Worker Execution (`src/core/worker/executer/job_executer.rs`)
The `JobExecuter` is responsible for pulling the code, installing dependencies, and building the project.
- **Project Type Detection**: Right before completing a job, it calls `detect_project_type()` to identify the stack (e.g., Rust, Node, React).
- **Completion Signal**: It calls `host.job_completed()` with the project's `full_name`, `job_type`, `commit_hash`, and the detected `project_type`.

### 2. Host API Bridge (`src/core/worker/api/host.rs`)
The `Host` struct provides an interface for the worker to communicate with the main application.
- **Endpoint**: `POST /job-completed`
- **Payload**: `JobCompletedReq` (JSON)

### 3. Application Controller (`src/app/controllers/project/job_completed.rs`)
A dedicated controller handles the completion signal.
- **Logic**:
    - If `job_type` is `Deploy`: Updates `commit_hash`, `last_deployment_time`, and `project_type`.
    - Otherwise (e.g., `Redeploy`, `Run`): Updates only `last_deployment_time` and `project_type`.
- **Lookup**: Projects are identified by their `full_name` (e.g., `owner/repo`), which is unique.

### 4. Data Models (`src/core/app_types.rs`)
- **`JobCompletedReq`**: The request structure for the completion signal.
- **`JobType`**: Enum for `Deploy`, `Redeploy`, `Run`.
- **`ProjectType`**: Enum for `Rust`, `Node`, `React`, `Html`, `Unknown`.

## Database Schema Updates

### Projects Table
The following columns are updated during the completion signal:
- `commit_hash`: The SHA of the deployed commit.
- `last_deployment_time`: Timestamp of the latest successful build.
- `project_type`: The detected framework/runtime.

### Migrations
- **`0004_unique_full_name.sql`**: Added a `UNIQUE` constraint to the `full_name` column in the `projects` table to ensure reliable lookups by the worker signal.

## Communication Payload Example
```json
{
  "project_id": "owner/repo-name",
  "job_type": "Deploy",
  "commit_hash": "a1b2c3d4e5f6...",
  "project_type": "react"
}
```

## Future References
- To add new project types, update the `ProjectType` enum in `app_types.rs` and its `Display` implementation.
- If the worker needs to send more metadata (e.g., build duration), add fields to `JobCompletedReq`.
