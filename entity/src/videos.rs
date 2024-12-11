//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.2

use super::sea_orm_active_enums::Status;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "videos")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    #[sea_orm(unique)]
    pub yt_id: String,
    pub created_at: DateTime,
    pub status: Status,
    pub updated_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::actions::Entity")]
    Actions,
}

impl Related<super::actions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Actions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}