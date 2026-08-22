use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add stripe_customer_id to `users` table
        if !manager.has_column("users", "stripe_customer_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Users::Table)
                        .add_column(ColumnDef::new(Users::StripeCustomerId).string().null())
                        .to_owned(),
                )
                .await?;
        }

        // 2. Add stripe fields & payment tracking to `billing_invoices` table
        if !manager.has_column("billing_invoices", "stripe_checkout_session_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(BillingInvoices::Table)
                        .add_column(ColumnDef::new(BillingInvoices::StripeCheckoutSessionId).string().null())
                        .add_column(ColumnDef::new(BillingInvoices::StripePaymentIntentId).string().null())
                        .add_column(
                            ColumnDef::new(BillingInvoices::PaymentStatus)
                                .string()
                                .not_null()
                                .default("pending"),
                        )
                        .add_column(
                            ColumnDef::new(BillingInvoices::AmountPaid)
                                .double()
                                .not_null()
                                .default(0.0),
                        )
                        .add_column(
                            ColumnDef::new(BillingInvoices::Currency)
                                .string_len(10)
                                .not_null()
                                .default("usd"),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_column("billing_invoices", "stripe_checkout_session_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(BillingInvoices::Table)
                        .drop_column(BillingInvoices::StripeCheckoutSessionId)
                        .drop_column(BillingInvoices::StripePaymentIntentId)
                        .drop_column(BillingInvoices::PaymentStatus)
                        .drop_column(BillingInvoices::AmountPaid)
                        .drop_column(BillingInvoices::Currency)
                        .to_owned(),
                )
                .await?;
        }

        if manager.has_column("users", "stripe_customer_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Users::Table)
                        .drop_column(Users::StripeCustomerId)
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
    StripeCustomerId,
}

#[derive(DeriveIden)]
enum BillingInvoices {
    Table,
    StripeCheckoutSessionId,
    StripePaymentIntentId,
    PaymentStatus,
    AmountPaid,
    Currency,
}
