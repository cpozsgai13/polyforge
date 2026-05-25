use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("users"))
                    .if_not_exists()
                    .col(ColumnDef::new(Alias::new("id")).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Alias::new("name")).string().not_null())
                    .col(ColumnDef::new(Alias::new("first")).string().not_null())
                    .col(ColumnDef::new(Alias::new("last")).string().not_null())
                    .col(ColumnDef::new(Alias::new("email")).string().not_null())
                    .col(ColumnDef::new(Alias::new("active")).boolean().not_null().default(false))
                    .col(ColumnDef::new(Alias::new("deleted")).boolean().not_null().default(false))
                    .col(ColumnDef::new(Alias::new("use_2fa")).boolean().not_null().default(false))
                    .col(ColumnDef::new(Alias::new("creation_time")).date_time().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("users")).to_owned())
            .await
    }
}
