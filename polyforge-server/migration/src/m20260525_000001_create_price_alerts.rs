use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("price_alerts"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("symbol")).string().not_null())
                    .col(ColumnDef::new(Alias::new("threshold")).double().not_null())
                    // "above" or "below"
                    .col(ColumnDef::new(Alias::new("direction")).string().not_null())
                    // optional user-supplied note
                    .col(ColumnDef::new(Alias::new("label")).string().null())
                    .col(
                        ColumnDef::new(Alias::new("triggered"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Alias::new("created_at")).date_time().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("price_alerts")).to_owned())
            .await
    }
}
