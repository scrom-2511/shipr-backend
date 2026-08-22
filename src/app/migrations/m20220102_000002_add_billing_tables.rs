use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add billing fields to `users` table
        if !manager.has_column("users", "credit_balance").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Users::Table)
                        .add_column(
                            ColumnDef::new(Users::CreditBalance)
                                .double()
                                .not_null()
                                .default(50.00),
                        )
                        .add_column(
                            ColumnDef::new(Users::PlanTier)
                                .string_len(50)
                                .not_null()
                                .default("Developer"),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 2. Add active_seconds to `projects` table
        if !manager.has_column("projects", "active_seconds").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Projects::Table)
                        .add_column(
                            ColumnDef::new(Projects::ActiveSeconds)
                                .big_integer()
                                .not_null()
                                .default(3600),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 3. Create `billing_invoices` table
        manager
            .create_table(
                Table::create()
                    .table(BillingInvoices::Table)
                    .if_not_exists()
                    .col(pk_auto(BillingInvoices::Id))
                    .col(integer(BillingInvoices::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-billing_invoices-user_id")
                            .from(BillingInvoices::Table, BillingInvoices::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(string(BillingInvoices::InvoiceNumber).unique_key())
                    .col(double(BillingInvoices::Amount))
                    .col(string(BillingInvoices::Status).default("paid"))
                    .col(double(BillingInvoices::ActiveHours).default(0.0))
                    .col(double(BillingInvoices::RatePerHour).default(0.02))
                    .col(timestamp(BillingInvoices::PeriodStart).default(Expr::current_timestamp()))
                    .col(timestamp(BillingInvoices::PeriodEnd).default(Expr::current_timestamp()))
                    .col(timestamp(BillingInvoices::CreatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 4. Create `payment_methods` table
        manager
            .create_table(
                Table::create()
                    .table(PaymentMethods::Table)
                    .if_not_exists()
                    .col(pk_auto(PaymentMethods::Id))
                    .col(integer(PaymentMethods::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-payment_methods-user_id")
                            .from(PaymentMethods::Table, PaymentMethods::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(string(PaymentMethods::CardBrand).default("Visa"))
                    .col(string(PaymentMethods::Last4).default("4242"))
                    .col(integer(PaymentMethods::ExpMonth).default(12))
                    .col(integer(PaymentMethods::ExpYear).default(2028))
                    .col(boolean(PaymentMethods::IsDefault).default(true))
                    .col(timestamp(PaymentMethods::CreatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PaymentMethods::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(BillingInvoices::Table).to_owned()).await?;

        if manager.has_column("projects", "active_seconds").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Projects::Table)
                        .drop_column(Projects::ActiveSeconds)
                        .to_owned(),
                )
                .await?;
        }

        if manager.has_column("users", "credit_balance").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Users::Table)
                        .drop_column(Users::CreditBalance)
                        .drop_column(Users::PlanTier)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    CreditBalance,
    PlanTier,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    ActiveSeconds,
}

#[derive(DeriveIden)]
enum BillingInvoices {
    Table,
    Id,
    UserId,
    InvoiceNumber,
    Amount,
    Status,
    ActiveHours,
    RatePerHour,
    PeriodStart,
    PeriodEnd,
    CreatedAt,
}

#[derive(DeriveIden)]
enum PaymentMethods {
    Table,
    Id,
    UserId,
    CardBrand,
    Last4,
    ExpMonth,
    ExpYear,
    IsDefault,
    CreatedAt,
}
