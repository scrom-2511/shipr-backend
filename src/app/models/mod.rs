use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "project_status")]
pub enum ProjectStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "deploying")]
    Deploying,
    #[sea_orm(string_value = "error")]
    Error,
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
        pub credit_balance: f64,
        pub plan_tier: String,
        pub stripe_customer_id: Option<String>,
        pub created_at: Option<DateTime>,
        pub updated_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::projects::Entity")]
        Projects,
        #[sea_orm(has_many = "super::github_repos::Entity")]
        GithubRepos,
        #[sea_orm(has_many = "super::billing_invoices::Entity")]
        BillingInvoices,
        #[sea_orm(has_many = "super::payment_methods::Entity")]
        PaymentMethods,
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

    impl Related<super::billing_invoices::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::BillingInvoices.def()
        }
    }

    impl Related<super::payment_methods::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::PaymentMethods.def()
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

        pub install_cmds: Option<Vec<String>>,
        pub run_cmds: Option<Vec<String>>,
        pub build_cmds: Option<Vec<String>>,

        pub branch: Option<String>,
        pub project_type: Option<crate::core::app_types::ProjectType>,
        pub full_name: String,

        pub dist_dir: Option<String>,
        pub root_dir: String,
        pub url: Option<String>,

        pub user_id: i32,

        pub commit_hash: Option<String>,
        pub envs: Option<Vec<String>>,

        pub last_deployment_time: Option<DateTime>,
        pub status: String,
        pub active_seconds: i64,

        pub created_at: Option<DateTime>,
        pub updated_at: Option<DateTime>,
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
        #[sea_orm(has_many = "super::project_traffic::Entity")]
        ProjectTraffic,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Users.def()
        }
    }

    impl Related<super::project_traffic::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ProjectTraffic.def()
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
        pub installation_ids: Vec<i32>,
        pub user_id: Option<i32>,
        pub created_at: Option<DateTime>,
        pub updated_at: Option<DateTime>,
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
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::projects::Entity",
            from = "Column::ProjectId",
            to = "super::projects::Column::Id",
            on_update = "NoAction",
            on_delete = "Cascade"
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

pub mod billing_invoices {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "billing_invoices")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: i32,
        #[sea_orm(unique)]
        pub invoice_number: String,
        pub amount: f64,
        pub status: String,
        pub active_hours: f64,
        pub rate_per_hour: f64,
        pub stripe_checkout_session_id: Option<String>,
        pub stripe_payment_intent_id: Option<String>,
        pub payment_status: String,
        pub amount_paid: f64,
        pub currency: String,
        pub period_start: Option<DateTime>,
        pub period_end: Option<DateTime>,
        pub created_at: Option<DateTime>,
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

pub mod payment_methods {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "payment_methods")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: i32,
        pub card_brand: String,
        pub last4: String,
        pub exp_month: i32,
        pub exp_year: i32,
        pub is_default: bool,
        pub created_at: Option<DateTime>,
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

pub type User = users::Model;
pub type Project = projects::Model;
pub type GithubAppInstallation = github_repos::Model;
