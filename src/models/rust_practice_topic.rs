use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "rust_practice_topics")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub title: String,
    pub s1_title: String,
    pub s1_content: String,
    pub s1_code: Option<String>,
    pub s2_title: String,
    pub s2_description: String,
    pub s2_code: String,
    pub s3_title: String,
    pub s3_description: String,
    pub s3_code_with_blanks: String,
    pub s3_solution: String,
    pub s4_title: String,
    pub s4_task: String,
    pub s4_hint: Option<String>,
    pub s4_solution: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
