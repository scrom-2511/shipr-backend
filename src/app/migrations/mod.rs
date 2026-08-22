use sea_orm_migration::prelude::*;

mod m20220101_000001_create_initial_tables;
mod m20220102_000002_add_billing_tables;
mod m20220103_000003_add_stripe_billing_fields;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_initial_tables::Migration),
            Box::new(m20220102_000002_add_billing_tables::Migration),
            Box::new(m20220103_000003_add_stripe_billing_fields::Migration),
        ]
    }
}
