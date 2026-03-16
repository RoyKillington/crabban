use chrono::Local;
use clap::{Parser, Subcommand};
use colored::{ColoredString, Colorize};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

// --- TASK --- //
#[derive(Serialize, Deserialize, Debug)]
struct Task {
    uid: u32,
    name: String,
    description: Option<String>,
    project: String,
    status: Status,
    story_points: Option<u8>,
    created_at: String,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    elapsed_secs: u64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Status {
    Backlog,
    OnDeck,
    Sprint,
    Done,
}

impl Status {
    fn from_str(s: &str) -> Option<Status> {
        match s {
            "backlog" => Some(Status::Backlog),
            "ondeck" => Some(Status::OnDeck),
            "sprint" => Some(Status::Sprint),
            "done" => Some(Status::Done),
            _ => None,
        }
    }
}

impl Task {
    fn new(
        uid: u32,
        name: &str,
        description: Option<&str>,
        project: &str,
        status: Status,
        story_points: Option<u8>,
        created_at: &str,
    ) -> Task {
        Task {
            uid,
            name: String::from(name),
            description: description.map(String::from),
            project: String::from(project),
            status,
            story_points,
            created_at: String::from(created_at),
            started_at: None,
            elapsed_secs: 0,
        }
    }

    fn display_string(&self) -> String {
        let base = format!(
            "[{}] {} - {} ({}pts)",
            self.uid,
            self.name,
            self.project,
            self.story_points.unwrap_or(0)
        );
        let total = compute_total_secs(self);
        if total > 0 {
            format!("{} [{}]", base, format_duration(total))
        } else {
            base
        }
    }
}

// --- TIME HELPERS --- //
fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn compute_total_secs(task: &Task) -> u64 {
    let mut total = task.elapsed_secs;
    if let Some(ref started) = task.started_at {
        if let Ok(start_time) = started.parse::<chrono::DateTime<Local>>() {
            let delta = Local::now() - start_time;
            total += delta.num_seconds().max(0) as u64;
        }
    }
    total
}

// --- BOARD --- //
#[derive(Serialize, Deserialize, Debug)]
struct Board {
    tasks: Vec<Task>,
    next_uid: u32,
    #[serde(skip)]
    path: PathBuf,
}

impl Board {
    // the board is the brain of the app, utlimately
    // it is responsibile for manaing the tasks

    // load (or create) a board
    fn load(path: PathBuf) -> Board {
        match fs::read_to_string(&path) {
            Ok(contents) => {
                let mut board: Board = serde_json::from_str(&contents).expect("Bad JSON");
                board.path = path;
                board
            }
            Err(_) => Board {
                tasks: Vec::new(),
                next_uid: 1,
                path,
            },
        }
    }

    // save a board
    fn save(&self) {
        fs::write(&self.path, serde_json::to_string_pretty(&self).unwrap()).unwrap();
    }

    // add task to board
    fn add_task(
        &mut self,
        name: &str,
        description: Option<&str>,
        project: &str,
        story_points: Option<u8>,
    ) {
        let task = Task::new(
            self.next_uid,
            name,
            description,
            project,
            Status::Backlog,
            story_points,
            &Local::now().to_string(),
        );
        self.tasks.push(task);
        self.next_uid += 1;
        self.save();
    }

    // delete task from board
    fn delete_task(&mut self, uid: u32) {
        let index = self.tasks.iter().position(|task| task.uid == uid);
        match index {
            Some(i) => {
                self.tasks.remove(i);
                println!("Deleted task {}", uid);
            }
            None => {
                println!("No task with uid {}", uid);
                return;
            }
        }
        self.save();
    }

    // remove all done tasks
    fn clean(&mut self) {
        let before = self.tasks.len();
        self.tasks.retain(|task| task.status != Status::Done);
        let removed = before - self.tasks.len();
        if removed > 0 {
            println!("Cleaned {} done task(s)", removed);
            self.save();
        } else {
            println!("No done tasks to clean");
        }
    }

    // mark task as done quickly
    fn done_task(&mut self, uid: u32) {
        let found = self.tasks.iter_mut().find(|task| task.uid == uid);
        match found {
            Some(task) => {
                task.status = Status::Done;
                // finalize timer if running
                if let Some(ref started) = task.started_at {
                    if let Ok(start_time) = started.parse::<chrono::DateTime<Local>>() {
                        let delta = (Local::now() - start_time).num_seconds().max(0) as u64;
                        task.elapsed_secs += delta;
                    }
                    task.started_at = None;
                }
                let total = task.elapsed_secs;
                if total > 0 {
                    println!(
                        "Task {} done. Total time: {}",
                        uid,
                        format_duration(total)
                    );
                } else {
                    println!("Task {} done.", uid);
                }
            }
            None => {
                println!("No task with uid {}", uid);
                return;
            }
        }
        self.save();
    }

    // start timer on a task
    fn start_task(&mut self, uid: u32) {
        let found = self.tasks.iter_mut().find(|task| task.uid == uid);
        match found {
            Some(task) => {
                if task.started_at.is_some() {
                    println!("Timer already running on task {}", uid);
                    return;
                }
                task.started_at = Some(Local::now().to_string());
                println!("Started timer on task {}: {}", uid, task.name);
            }
            None => {
                println!("No task with uid {}", uid);
                return;
            }
        }
        self.save();
    }

    // pause timer on a task
    fn pause_task(&mut self, uid: u32) {
        let found = self.tasks.iter_mut().find(|task| task.uid == uid);
        match found {
            Some(task) => {
                if task.started_at.is_none() {
                    println!("Timer is not running on task {}", uid);
                    return;
                }
                let start_time = task
                    .started_at
                    .as_ref()
                    .unwrap()
                    .parse::<chrono::DateTime<Local>>()
                    .expect("Could not parse start time");
                let delta = (Local::now() - start_time).num_seconds().max(0) as u64;
                task.elapsed_secs += delta;
                task.started_at = None;
                println!(
                    "Paused timer on task {}: {} (total: {})",
                    uid,
                    task.name,
                    format_duration(task.elapsed_secs)
                );
            }
            None => {
                println!("No task with uid {}", uid);
                return;
            }
        }
        self.save();
    }

    // modify task Staus
    fn move_task(&mut self, uid: u32, status: Status) {
        let found = self.tasks.iter_mut().find(|task| task.uid == uid);
        match found {
            Some(task) => task.status = status,
            None => {
                println!("No task with uid {}", uid);
                return;
            }
        }
        self.save();
    }

    // show board
    fn show(&self, project: Option<&str>, status: Option<Status>) {
        match status {
            Some(status) => {
                self.show_column(status, project);
            }
            None => {
                let backlog: Vec<&Task> = self
                    .tasks
                    .iter()
                    .filter(|task| task.status == Status::Backlog)
                    .collect();
                let ondeck: Vec<&Task> = self
                    .tasks
                    .iter()
                    .filter(|task| task.status == Status::OnDeck)
                    .collect();
                let sprint: Vec<&Task> = self
                    .tasks
                    .iter()
                    .filter(|task| task.status == Status::Sprint)
                    .collect();
                let done: Vec<&Task> = self
                    .tasks
                    .iter()
                    .filter(|task| task.status == Status::Done)
                    .collect();
                let rows: usize = backlog
                    .len()
                    .max(ondeck.len())
                    .max(sprint.len())
                    .max(done.len());
                let longest_string = self
                    .tasks
                    .iter()
                    .map(|t| t.display_string().len())
                    .max()
                    .unwrap_or(20);
                let col_width: usize = longest_string;
                print!("{}", format!("{:<w$}", "BACKLOG", w = col_width).blue());
                print!("{}", format!("{:<w$}", "ON DECK", w = col_width).yellow());
                print!("{}", format!("{:<w$}", "SPRINT", w = col_width).green());
                println!("{}", format!("{:<w$}", "DONE", w = col_width).magenta());
                for i in 0..rows {
                    print!("{}", Self::format_cell(backlog.get(i), col_width));
                    print!("{}", Self::format_cell(ondeck.get(i), col_width));
                    print!("{}", Self::format_cell(sprint.get(i), col_width));
                    println!("{}", Self::format_cell(done.get(i), col_width));
                }
            }
        }
    }

    fn show_column(&self, status: Status, project: Option<&str>) {
        println!("{}", Self::status_header(&status));
        for task in self.tasks.iter().filter(|task| {
            task.status == status
                && match project {
                    Some(p) => task.project == p,
                    None => true,
                }
        }) {
            println!("  {}", task.display_string())
        }
    }

    fn format_cell(task: Option<&&Task>, width: usize) -> String {
        match task {
            Some(t) => format!("  {}", t.display_string()),
            None => format!("{:<width$}", "", width = width),
        }
    }

    fn view_task(&self, uid: u32) {
        let found = self.tasks.iter().find(|task| task.uid == uid);
        match found {
            Some(task) => {
                let status_str = match task.status {
                    Status::Backlog => "Backlog",
                    Status::OnDeck => "On Deck",
                    Status::Sprint => "Sprint",
                    Status::Done => "Done",
                };
                let desc = task
                    .description
                    .as_deref()
                    .unwrap_or("—");
                let points = task
                    .story_points
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "—".to_string());
                let total = compute_total_secs(task);
                let time = if total > 0 {
                    format_duration(total)
                } else {
                    "—".to_string()
                };
                let timer = if task.started_at.is_some() {
                    "running"
                } else {
                    "paused"
                };

                println!("  UID:          {}", task.uid);
                println!("  Name:         {}", task.name);
                println!("  Description:  {}", desc);
                println!("  Project:      {}", task.project);
                println!("  Status:       {}", status_str);
                println!("  Points:       {}", points);
                println!("  Created:      {}", task.created_at);
                println!("  Time:         {}", time);
                println!("  Timer:        {}", timer);
            }
            None => {
                println!("No task with uid {}", uid);
            }
        }
    }

    fn status_header(status: &Status) -> ColoredString {
        match status {
            Status::Backlog => "BACKLOG".blue(),
            Status::OnDeck => "ON DECK".yellow(),
            Status::Sprint => "SPRINT".green(),
            Status::Done => "DONE".magenta(),
        }
    }
}

// --- CLI --- //
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        title: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        desc: Option<String>,
        #[arg(long)]
        points: Option<u8>,
    },
    Delete {
        uid: u32,
    },
    Done {
        uid: u32,
    },
    Start {
        uid: u32,
    },
    Pause {
        uid: u32,
    },
    Move {
        uid: u32,
        status: String,
    },
    Show {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    View {
        uid: u32,
    },
    Clean,
}

fn main() {
    let cli = Cli::parse();
    let data_dir = dirs::data_dir().unwrap().join("crabban");
    std::fs::create_dir_all(&data_dir).expect("Could not create data directory");
    let path = data_dir.join("board.json");
    let mut board: Board = Board::load(path);

    match cli.command {
        Commands::Add {
            title,
            project,
            desc,
            points,
        } => {
            board.add_task(&title, desc.as_deref(), &project, points);
        }
        Commands::Delete { uid } => {
            board.delete_task(uid);
        }
        Commands::Done { uid } => {
            board.done_task(uid);
        }
        Commands::Start { uid } => {
            board.start_task(uid);
        }
        Commands::Pause { uid } => {
            board.pause_task(uid);
        }
        Commands::Move { uid, status } => {
            let status = Status::from_str(&status).expect("Invalid status");
            board.move_task(uid, status);
        }
        Commands::Show { project, status } => {
            let status_filter = status.as_deref().and_then(Status::from_str);
            board.show(project.as_deref(), status_filter);
        }
        Commands::View { uid } => {
            board.view_task(uid);
        }
        Commands::Clean => {
            board.clean();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_task() {
        let path = std::env::temp_dir().join("crabban_test.json");
        let mut board: Board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task(
            "Test Task 1",
            Some("Test task 1 description"),
            "crabban",
            Some(1),
        );
        assert_eq!(board.tasks.len(), 1);
        assert_eq!(board.tasks[0].uid, 1);
        assert_eq!(board.next_uid, 2);
        assert_eq!(board.tasks[0].status, Status::Backlog);
    }

    #[test]
    fn test_delete_task() {
        let path = std::env::temp_dir().join("crabban_test.json");
        let mut board: Board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task(
            "Test Task 1",
            Some("Test task 1 description"),
            "crabban",
            Some(1),
        );
        board.add_task("Test Task 2", None, "crabban", Some(1));
        board.delete_task(1);
        assert_eq!(board.tasks.len(), 1);
        assert_eq!(board.tasks[0].uid, 2);
    }

    #[test]
    fn test_delete_nonexistent_task() {
        let path = std::env::temp_dir().join("crabban_test.json");
        let mut board: Board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task(
            "Test Task 1",
            Some("Test task 1 description"),
            "crabban",
            Some(1),
        );
        board.delete_task(2);
        assert_eq!(board.tasks.len(), 1);
    }

    #[test]
    fn test_move_task() {
        let path = std::env::temp_dir().join("crabban_test.json");
        let mut board: Board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task(
            "Test Task 1",
            Some("Test task 1 description"),
            "crabban",
            Some(1),
        );
        board.move_task(1, Status::Sprint);
        assert_eq!(board.tasks[0].status, Status::Sprint);
    }

    #[test]
    fn test_move_nonexistent_task() {
        let path = std::env::temp_dir().join("crabban_test.json");
        let mut board: Board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task(
            "Test Task 1",
            Some("Test task 1 description"),
            "crabban",
            Some(1),
        );
        board.move_task(2, Status::Sprint);
        assert_eq!(board.tasks[0].status, Status::Backlog);
    }

    // --- Time tracking tests --- //

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3661), "1h 1m 1s");
        assert_eq!(format_duration(7200), "2h 0m 0s");
    }

    #[test]
    fn test_start_task() {
        let path = std::env::temp_dir().join("crabban_test_start.json");
        let mut board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task("Timer Task", None, "test", None);
        board.start_task(1);
        assert!(board.tasks[0].started_at.is_some());
        assert_eq!(board.tasks[0].elapsed_secs, 0);
    }

    #[test]
    fn test_start_already_started() {
        let path = std::env::temp_dir().join("crabban_test_start_twice.json");
        let mut board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task("Timer Task", None, "test", None);
        board.start_task(1);
        let original_started = board.tasks[0].started_at.clone();
        board.start_task(1); // should be a no-op
        assert_eq!(board.tasks[0].started_at, original_started);
    }

    #[test]
    fn test_pause_task() {
        let path = std::env::temp_dir().join("crabban_test_pause.json");
        let mut board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task("Timer Task", None, "test", None);
        // manually set started_at to 60 seconds ago to avoid sleeping
        let past = (Local::now() - chrono::Duration::seconds(60)).to_string();
        board.tasks[0].started_at = Some(past);
        board.pause_task(1);
        assert!(board.tasks[0].started_at.is_none());
        // should be approximately 60 seconds (allow some tolerance)
        assert!(board.tasks[0].elapsed_secs >= 59);
        assert!(board.tasks[0].elapsed_secs <= 62);
    }

    #[test]
    fn test_pause_not_started() {
        let path = std::env::temp_dir().join("crabban_test_pause_none.json");
        let mut board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task("Timer Task", None, "test", None);
        board.pause_task(1); // should be a no-op
        assert!(board.tasks[0].started_at.is_none());
        assert_eq!(board.tasks[0].elapsed_secs, 0);
    }

    #[test]
    fn test_done_with_timer() {
        let path = std::env::temp_dir().join("crabban_test_done_timer.json");
        let mut board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task("Timer Task", None, "test", None);
        // simulate 120 seconds of prior work + a running timer started 30s ago
        board.tasks[0].elapsed_secs = 120;
        let past = (Local::now() - chrono::Duration::seconds(30)).to_string();
        board.tasks[0].started_at = Some(past);
        board.done_task(1);
        assert_eq!(board.tasks[0].status, Status::Done);
        assert!(board.tasks[0].started_at.is_none());
        // should be approximately 150 seconds
        assert!(board.tasks[0].elapsed_secs >= 149);
        assert!(board.tasks[0].elapsed_secs <= 152);
    }

    #[test]
    fn test_done_without_timer() {
        let path = std::env::temp_dir().join("crabban_test_done_no_timer.json");
        let mut board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task("Plain Task", None, "test", None);
        board.done_task(1);
        assert_eq!(board.tasks[0].status, Status::Done);
        assert_eq!(board.tasks[0].elapsed_secs, 0);
        assert!(board.tasks[0].started_at.is_none());
    }

    #[test]
    fn test_view_task() {
        let path = std::env::temp_dir().join("crabban_test_view.json");
        let mut board = Board {
            tasks: Vec::new(),
            next_uid: 1,
            path,
        };
        board.add_task("View Me", Some("A detailed description"), "demo", Some(5));
        // should not panic
        board.view_task(1);
        // nonexistent task should not panic either
        board.view_task(999);
    }

    #[test]
    fn test_backwards_compat_deserialize() {
        // JSON without the new time-tracking fields — simulates an old board.json
        let json = r#"{
            "tasks": [{
                "uid": 1,
                "name": "Old Task",
                "description": null,
                "project": "legacy",
                "status": "Backlog",
                "story_points": null,
                "created_at": "2025-01-01"
            }],
            "next_uid": 2
        }"#;
        let board: Board = serde_json::from_str(json).unwrap();
        assert_eq!(board.tasks[0].elapsed_secs, 0);
        assert!(board.tasks[0].started_at.is_none());
    }
}
