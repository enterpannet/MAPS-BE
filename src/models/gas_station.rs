use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gas_stations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub osm_id: i64,
    #[sea_orm(column_name = "osm_type")]
    pub osm_type: String,
    pub lat: f64,
    pub lng: f64,
    pub name: Option<String>,
    pub brand: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
