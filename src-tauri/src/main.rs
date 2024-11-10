// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::iter::zip;

use sea_orm::*;

mod entities;
use entities::{blockers, prelude::*, schedules};
use timegate_lib::{AppState, Blocker, Schedule, ScheduleLoadError};

async fn load_schedules(state: &mut AppState) -> Result<(), ScheduleLoadError> {
    let loaded_schedules: Vec<schedules::Model> = Schedules::find().all(&state.db).await?;

    let loaded_blockers: Vec<Vec<blockers::Model>> =
        loaded_schedules.load_many(Blockers, &state.db).await?;

    let loaded_schedules = zip(loaded_schedules, loaded_blockers);

    let schedules: Result<Vec<Schedule>, ScheduleLoadError> = loaded_schedules
        .map(|(schedule_model, blockers_model)| schedule_model.to_schedule(&blockers_model))
        .collect();

    state.schedules = schedules?;

    println!("{:?}", state.schedules);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ScheduleLoadError> {
    let mut state = AppState {
        ..Default::default()
    };
    state.db = Database::connect("sqlite://timegate.sqlite?mode=rwc").await?;
    println!("Database connection successful.");
    load_schedules(&mut state).await?;
    timegate_lib::run(state);
    Ok(())
}
