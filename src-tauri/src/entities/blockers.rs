//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sea_orm::entity::prelude::*;

use crate::{Blocker, ScheduleLoadError};
use core::num;
use std::convert::TryFrom;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "blockers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    pub schedule_id: i32,
    pub weekday: i32,
    pub duration: i32,
    #[sea_orm(column_type = "Text")]
    pub start_time: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::schedules::Entity",
        from = "Column::ScheduleId",
        to = "super::schedules::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Schedules,
}

impl Related<super::schedules::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Schedules.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn to_blocker(&self) -> Result<Blocker, ScheduleLoadError> {
        Blocker::new(self.id, self.weekday, &self.start_time, self.duration)
    }
}
