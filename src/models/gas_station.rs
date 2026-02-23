use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gas_stations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub osm_id: Option<i64>,
    #[sea_orm(column_name = "osm_type")]
    pub osm_type: Option<String>,
    pub lat: f64,
    pub lng: f64,
    pub name: Option<String>,
    pub brand: Option<String>,
    #[sea_orm(column_name = "source")]
    pub source: String,
    #[sea_orm(column_name = "external_id")]
    pub external_id: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
