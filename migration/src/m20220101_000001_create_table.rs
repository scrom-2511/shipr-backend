use sea_orm::{
    ConnectionTrait, DatabaseBackend, DbBackend, DeriveActiveEnum, EnumIter, Schema, Statement,
    sea_query::extension::postgres::Type,
};
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
                    .col(string(Users::Username).not_null())
                    .col(string(Users::Email).unique_key().not_null())
                    .col(string(Users::Password).not_null())
                    .col(big_integer(Users::CreditBalance).not_null().default(5000))
                    .col(string(Users::DodoCustomerId).unique_key().null())
                    .col(string(Users::DodoSubscriptionId).unique_key().null())
                    .col(boolean(Users::AutoTopupEnabled).not_null().default(false))
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
                    .col(integer(GithubRepos::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(GithubRepos::Table, GithubRepos::UserId)
                            .to(Users::Table, Users::Id),
                    )
                    .col(timestamp(GithubRepos::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(GithubRepos::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        let schema = Schema::new(DbBackend::Postgres);
        if let Some(stmt) = schema.create_enum_from_active_enum::<ProjectType>() {
            manager.create_type(stmt).await?;
        }

        if let Some(stmt) = schema.create_enum_from_active_enum::<ProjectStatus>() {
            manager.create_type(stmt).await?;
        }

        // 4. Create `projects` table
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(pk_auto(Projects::Id))
                    .col(string(Projects::ProjectId).unique_key().not_null())
                    .col(string(Projects::Branch).not_null())
                    .col(
                        ColumnDef::new(Projects::ProjectType)
                            .custom("project_type")
                            .not_null(),
                    )
                    .col(string(Projects::FullName).not_null())
                    .col(string(Projects::RootDir).not_null())
                    .col(string(Projects::Url).unique_key())
                    .col(integer(Projects::UserId).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Projects::Table, Projects::UserId)
                            .to(Users::Table, Users::Id),
                    )
                    .col(string(Projects::CommitHash).not_null())
                    .col(ColumnDef::new(Projects::Envs).json_binary())
                    .col(timestamp(Projects::LastDeploymentTime).default(Expr::current_timestamp()))
                    .col(
                        ColumnDef::new(Projects::Status)
                            .custom("project_status")
                            .not_null(),
                    )
                    .col(integer(Projects::ActiveSeconds).default(0))
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
                    .col(integer(ProjectTraffic::ProjectId).not_null())
                    .col(date(ProjectTraffic::Date).not_null())
                    .col(integer(ProjectTraffic::RequestCount).not_null().default(0))
                    .col(timestamp(ProjectTraffic::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(ProjectTraffic::UpdatedAt).default(Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .from(ProjectTraffic::Table, ProjectTraffic::ProjectId)
                            .to(Projects::Table, Projects::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // 6. Create `billing` table
        manager
            .create_table(
                Table::create()
                    .table(Billing::Table)
                    .if_not_exists()
                    .col(pk_auto(Billing::Id))
                    .col(integer(Billing::UserId).not_null())
                    .col(string(Billing::PaymentId).unique_key().not_null())
                    .col(double(Billing::Amount).not_null())
                    .col(string(Billing::Currency).not_null())
                    .col(timestamp(Billing::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(Billing::UpdatedAt).default(Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Billing::Table, Billing::UserId)
                            .to(Users::Table, Users::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
    CreditBalance,
    DodoCustomerId,
    DodoSubscriptionId,
    AutoTopupEnabled,
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
    ActiveSeconds,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "project_type")]
pub enum ProjectType {
    #[sea_orm(string_value = "html")]
    Html,
    #[sea_orm(string_value = "rust")]
    Rust,
    #[sea_orm(string_value = "react")]
    React,
    #[sea_orm(string_value = "node")]
    Node,
    #[sea_orm(string_value = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "project_status")]
pub enum ProjectStatus {
    #[sea_orm(string_value = "deploying")]
    Deploying,

    #[sea_orm(string_value = "running")]
    Running,

    #[sea_orm(string_value = "stopped")]
    Stopped,

    #[sea_orm(string_value = "error")]
    ErrorStatus,
}

#[derive(DeriveIden)]
enum ProjectTraffic {
    Table,
    Id,
    ProjectId,
    Date,
    RequestCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Billing {
    Table,
    Id,
    UserId,
    PaymentId,
    Amount,
    Currency,
    CreatedAt,
    UpdatedAt,
}
