use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "price_alerts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub symbol: String,
    pub threshold: f64,
    /// "above" or "below"
    pub direction: String,
    pub label: Option<String>,
    pub triggered: bool,
    pub created_at: Option<DateTime>,
    pub triggered_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
