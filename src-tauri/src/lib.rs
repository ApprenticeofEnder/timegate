use std::convert::TryFrom;
use std::num::TryFromIntError;
use std::process::Command;

use chrono::prelude::*;
use chrono::TimeDelta;
use sea_orm::*;
use std::sync::Mutex;
use tauri::{async_runtime, Emitter};
use tauri::{AppHandle, Listener, Manager};
use thiserror::Error;
use tokio::time::{sleep, Duration};

#[derive(Default)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub schedules: Vec<Schedule>,
    pub update_schedules: bool,
}

#[derive(Error, Debug)]
pub enum ScheduleLoadError {
    #[error("Database error occurred")]
    DatabaseError(#[from] DbErr),
    #[error("Numeric conversion error")]
    NumericError(#[from] TryFromIntError),
    #[error("Time conversion error")]
    TimeError(#[from] chrono::ParseError),
    #[error("Unknown data processing error")]
    Unknown,
}

#[derive(Default, Debug)]
pub struct Schedule {
    id: i32,
    name: String,
    blockers: Vec<Blocker>,
    active: bool,
}

impl Schedule {
    pub fn new(id: i32, name: &String, blockers: Vec<Blocker>, active: bool) -> Schedule {
        Schedule {
            id,
            name: name.clone(),
            blockers,
            active,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_blockers(&self) -> &Vec<Blocker> {
        &self.blockers
    }

    pub fn get_active(&self) -> bool {
        self.active
    }

    pub fn should_block(&self, now: DateTime<Local>) -> bool {
        let mut blocker_found = false;
        if !self.active {
            return false;
        }

        self.blockers
            .iter()
            .scan(&mut blocker_found, |blocker_found, blocker| {
                match blocker.should_block(now) {
                    true => {
                        **blocker_found = true;
                        None
                    }
                    false => Some(false),
                }
            })
            .for_each(|_| {});
        return blocker_found;
    }
}

#[derive(Default, Debug)]
pub struct Blocker {
    id: i32,
    weekday: u32,
    start_time: NaiveTime,
    duration: TimeDelta,
}

impl Blocker {
    pub fn new(
        id: i32,
        weekday: i32,
        start_time: &String,
        duration: i32,
    ) -> Result<Blocker, ScheduleLoadError> {
        Ok(Blocker {
            id,
            weekday: u32::try_from(weekday)?,
            start_time: NaiveTime::parse_from_str(start_time, "%H:%M:%S")?,
            duration: TimeDelta::new(duration as i64, 0).unwrap(),
            ..Default::default()
        })
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_weekday(&self) -> u32 {
        self.weekday
    }

    pub fn get_start_time(&self) -> NaiveTime {
        self.start_time
    }

    pub fn get_duration(&self) -> TimeDelta {
        self.duration
    }

    pub fn should_block(&self, now: DateTime<Local>) -> bool {
        let weekday = now.weekday().number_from_monday();
        let start = now.naive_local().date().and_time(self.start_time);
        let end = start + self.duration;

        let time_match = now.naive_local() > start && now.naive_local() < end;
        let weekday_match = weekday == self.weekday;

        return time_match && weekday_match;
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn shutdown_windows() {
    Command::new("cmd")
        .args(["/C", "shutdown /s"])
        .output()
        .expect("failed to execute process");
}

fn shutdown_linux() {
    Command::new("sh")
        .arg("-c")
        .arg("shutdown")
        .output()
        .expect("failed to execute process");
}

fn shutdown() {
    if cfg!(target_os = "windows") {
        shutdown_windows();
    } else {
        shutdown_linux();
    };
}

fn shutdown_watcher(app: &AppHandle) {
    let app = app.clone();
    println!("Hello from the watcher!");
    async_runtime::spawn(async move {
        loop {
            app.emit("shutdown_check", ()).unwrap();
            sleep(Duration::from_secs(1)).await;
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(state: AppState) {
    tauri::Builder::default()
        // .manage(Mutex::new(state))
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            app.manage(Mutex::new(state));
            let app_handle = app.handle().clone();
            app.listen_any("shutdown_check", move |event| {
                let state = app_handle.state::<Mutex<AppState>>();
                let mut shutting_down = false;

                let state = state.lock().unwrap();

                let now = chrono::Local::now();

                state
                    .schedules
                    .iter()
                    .scan(&mut shutting_down, |shutting_down, schedule| {
                        match schedule.should_block(now) {
                            true => {
                                **shutting_down = true;
                                println!("Shutting down...");
                                // shutdown();
                                None
                            }
                            _ => Some(false),
                        }
                    })
                    .for_each(|_| {});
            });
            shutdown_watcher(app.handle());
            println!("Watcher started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
