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
        }
    }
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
                self.save();
                println!("Deleted task {}", uid)
            }
            None => {
                println!("No task with uid {}", uid)
            }
        }
    }

    // mark task as done quickly
    fn done_task(&mut self, uid: u32) {
        let found = self.tasks.iter_mut().find(|task| task.uid == uid);
        match found {
            Some(task) => task.status = Status::Done,
            None => {
                println!("No task with uid {}", uid)
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
                println!("No task with uid {}", uid)
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
                    .map(|t| {
                        format!(
                            "[{}] {} - {} ({}pts)",
                            t.uid,
                            t.name,
                            t.project,
                            t.story_points.unwrap_or(0)
                        )
                        .len()
                    })
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
            println!(
                "  [{}] {} - {} ({}pts)",
                task.uid,
                task.name,
                task.project,
                task.story_points.unwrap_or(0)
            )
        }
    }

    fn format_cell(task: Option<&&Task>, width: usize) -> String {
        match task {
            Some(t) => {
                let task = *t;
                format!(
                    "  [{}] {} - {} ({}pts)",
                    task.uid,
                    task.name,
                    task.project,
                    task.story_points.unwrap_or(0)
                )
            }
            None => {
                format!("{:<width$}", "", width = width)
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
        Commands::Move { uid, status } => {
            let status = Status::from_str(&status).expect("Invalid status");
            board.move_task(uid, status);
        }
        Commands::Show { project, status } => {
            let status_filter = status.as_deref().and_then(Status::from_str);
            board.show(project.as_deref(), status_filter);
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
}
