use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("price_alerts"))
                    .add_column(ColumnDef::new(Alias::new("triggered_at")).date_time().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("price_alerts"))
                    .drop_column(Alias::new("triggered_at"))
                    .to_owned(),
            )
            .await
    }
}
