use sea_orm::DatabaseBackend;
use sea_orm_migration::sea_query::extension::postgres::Type;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create `users` table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(pk_auto(Users::Id))
                    .col(string(Users::Username))
                    .col(string(Users::Email).unique_key())
                    .col(string(Users::Password))
                    .col(timestamp(Users::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(Users::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 2. Create `github_repos` table
        manager
            .create_table(
                Table::create()
                    .table(GithubRepos::Table)
                    .if_not_exists()
                    .col(pk_auto(GithubRepos::Id))
                    .col(ColumnDef::new(GithubRepos::InstallationIds).array(ColumnType::Integer))
                    .col(integer_null(GithubRepos::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-github_repos-user_id")
                            .from(GithubRepos::Table, GithubRepos::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(timestamp(GithubRepos::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(GithubRepos::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 3. Create `project_type` Enum (Postgres)
        if manager.get_database_backend() == DatabaseBackend::Postgres {
            let _ = manager
                .create_type(
                    Type::create()
                        .as_enum(Alias::new("project_type"))
                        .values(["html", "rust", "react", "node", "unknown"])
                        .to_owned(),
                )
                .await;
        }

        // 4. Create `projects` table
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(pk_auto(Projects::Id))
                    .col(string(Projects::ProjectId).unique_key())
                    .col(ColumnDef::new(Projects::InstallCmds).array(ColumnType::Text))
                    .col(ColumnDef::new(Projects::RunCmds).array(ColumnType::Text))
                    .col(ColumnDef::new(Projects::BuildCmds).array(ColumnType::Text))
                    .col(string_null(Projects::Branch))
                    .col(ColumnDef::new(Projects::ProjectType).custom(Alias::new("project_type")))
                    .col(string(Projects::FullName))
                    .col(string_null(Projects::DistDir))
                    .col(string(Projects::RootDir))
                    .col(string_null(Projects::Url).unique_key())
                    .col(integer(Projects::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-projects-user_id")
                            .from(Projects::Table, Projects::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(string_null(Projects::CommitHash))
                    .col(ColumnDef::new(Projects::Envs).array(ColumnType::Text))
                    .col(timestamp(Projects::LastDeploymentTime).default(Expr::current_timestamp()))
                    .col(string(Projects::Status))
                    .col(timestamp(Projects::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(Projects::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 5. Create `project_traffic` table
        manager
            .create_table(
                Table::create()
                    .table(ProjectTraffic::Table)
                    .if_not_exists()
                    .col(pk_auto(ProjectTraffic::Id))
                    .col(integer(ProjectTraffic::ProjectId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-project_traffic-project_id")
                            .from(ProjectTraffic::Table, ProjectTraffic::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(date(ProjectTraffic::Date).default(Expr::current_date()))
                    .col(integer(ProjectTraffic::RequestCount).default(1))
                    .index(
                        Index::create()
                            .unique()
                            .name("idx-project_traffic-project_id-date")
                            .col(ProjectTraffic::ProjectId)
                            .col(ProjectTraffic::Date),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProjectTraffic::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await?;
        if manager.get_database_backend() == DatabaseBackend::Postgres {
            let _ = manager
                .drop_type(Type::drop().name(Alias::new("project_type")).to_owned())
                .await;
        }
        manager
            .drop_table(Table::drop().table(GithubRepos::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    Email,
    Password,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GithubRepos {
    Table,
    Id,
    InstallationIds,
    UserId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    ProjectId,
    InstallCmds,
    RunCmds,
    BuildCmds,
    Branch,
    ProjectType,
    FullName,
    DistDir,
    RootDir,
    Url,
    UserId,
    CommitHash,
    Envs,
    LastDeploymentTime,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ProjectTraffic {
    Table,
    Id,
    ProjectId,
    Date,
    RequestCount,
}
