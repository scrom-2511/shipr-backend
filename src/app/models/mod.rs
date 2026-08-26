use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "project_status")]
pub enum ProjectStatus {
    #[sea_orm(string_value = "deploying")]
    Deploying,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "stopped")]
    Stopped,
    #[sea_orm(string_value = "ready")]
    Ready,
    #[sea_orm(string_value = "error")]
    ErrorStatus,
}

pub mod users {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub username: String,
        #[sea_orm(unique)]
        pub email: String,
        pub password: String,
        pub credit_balance: i64,
        pub dodo_customer_id: Option<String>,
        pub dodo_subscription_id: Option<String>,
        pub auto_topup_enabled: bool,
        pub created_at: Option<DateTime>,
        pub updated_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::projects::Entity")]
        Projects,
        #[sea_orm(has_many = "super::github_repos::Entity")]
        GithubRepos,
    }

    impl Related<super::projects::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Projects.def()
        }
    }

    impl Related<super::github_repos::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::GithubRepos.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod github_repos {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "github_repos")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub installation_ids: Option<Vec<i32>>,
        pub user_id: Option<i32>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id",
            on_update = "NoAction",
            on_delete = "Cascade"
        )]
        Users,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Users.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod projects {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "projects")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sea_orm(unique)]
        pub project_id: String,
        pub branch: String,
        pub project_type: Option<ProjectType>,
        pub full_name: String,
        pub root_dir: String,
        pub url: Option<String>,
        pub user_id: i32,
        pub commit_hash: Option<String>,
        pub envs: Option<serde_json::Value>,
        pub last_deployment_time: Option<DateTime>,
        pub status: ProjectStatus,
        pub active_seconds: i64,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id",
            on_update = "NoAction",
            on_delete = "NoAction"
        )]
        Users,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Users.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod project_traffic {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "project_traffic")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub project_id: i32,
        pub date: Date,
        pub request_count: i32,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::projects::Entity",
            from = "Column::ProjectId",
            to = "super::projects::Column::Id",
            on_update = "NoAction",
            on_delete = "NoAction"
        )]
        Projects,
    }

    impl Related<super::projects::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Projects.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod billing {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "billing")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: i32,
        pub payment_id: String,
        pub checkout_session_id: String,
        pub amount: i64,
        pub currency: String,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id",
            on_update = "NoAction",
            on_delete = "NoAction"
        )]
        Users,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Users.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub type User = users::Model;
pub type Project = projects::Model;
pub type GithubAppInstallation = github_repos::Model;
pub type ProjectTraffic = project_traffic::Model;
pub type Billing = billing::Model;
