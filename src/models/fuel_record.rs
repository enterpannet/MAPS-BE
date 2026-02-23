use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fuel_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub room_id: Uuid,
    pub trip_id: Uuid,
    pub user_id: Uuid,
    pub input_mode: String,
    pub distance_km: Option<f64>,
    pub fuel_liters: Option<f64>,
    pub km_per_liter: Option<f64>,
    pub price_per_liter: Option<f64>,
    pub total_cost: f64,
    pub receipt_image: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::room::Entity",
        from = "Column::RoomId",
        to = "super::room::Column::Id"
    )]
    Room,
    #[sea_orm(
        belongs_to = "super::trip::Entity",
        from = "Column::TripId",
        to = "super::trip::Column::Id"
    )]
    Trip,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}
