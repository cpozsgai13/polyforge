pub use sea_orm_migration::prelude::*;

mod m20260523_161541_create_users_table;
mod m20260525_000001_create_price_alerts;
mod m20260525_000002_add_triggered_at;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260523_161541_create_users_table::Migration),
            Box::new(m20260525_000001_create_price_alerts::Migration),
            Box::new(m20260525_000002_add_triggered_at::Migration),
        ]
    }
}
