use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Status {
    Open,
    Closed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    created_at: DateTime<Utc>,
    note: String,
}

impl Note {
    pub fn new(note: &str) -> Note {
        Note {
            created_at: Utc::now(),
            note: String::from(note),
        }
    }

    pub fn note(&self) -> &str {
        &self.note
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    id: u32,
    name: String,
    status: Status,

    #[serde(default = "Utc::now")]
    created_at: DateTime<Utc>,

    closed_at: Option<DateTime<Utc>>,

    #[serde(default)]
    priority: u32,

    #[serde(default)]
    notes: Vec<Note>,
}

impl Task {
    pub fn new(id: u32, name: String, status: Status, priority: u32) -> Self {
        Self {
            id,
            name,
            status,
            priority,
            created_at: Utc::now(),
            closed_at: None,
            notes: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn close(&mut self) {
        self.closed_at = Some(Utc::now());
        self.status = Status::Closed;
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }

    pub fn set_priority(&mut self, priority: u32) {
        self.priority = priority
    }

    pub fn notes(&self) -> Vec<&Note> {
        self.notes.iter().collect()
    }

    pub fn add_note(&mut self, note: &str) {
        self.notes.push(Note::new(note));
    }

    pub fn write(task: &Task, path: &std::path::PathBuf) -> std::io::Result<()> {
        let mut path = std::path::PathBuf::from(path);
        path.push(format!("{}.yaml", task.id));
        let mut file = File::create(path)?;

        let s = serde_yaml::to_string(task)
            .map_err(|why| IoError::new(IoErrorKind::InvalidData, why))?;

        file.write_all(s.as_bytes())
    }

    pub fn read(path: &std::path::PathBuf) -> std::io::Result<Task> {
        let mut file = File::open(path)?;

        let mut s = String::new();
        file.read_to_string(&mut s)?;

        let y = serde_yaml::from_str::<Task>(&s)
            .map_err(|why| IoError::new(IoErrorKind::InvalidData, why))?;
        Ok(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_task() {
        let mut task = Task::new(1, "test".into(), Status::Open, 1);
        task.close();
        assert_eq!(Status::Closed, task.status);
        assert_eq!(true, task.closed_at.is_some());
    }

    #[test]
    fn notes() {
        // add a note
        let mut task = Task::new(1, "test".into(), Status::Open, 1);
        task.add_note("test note");
        assert_eq!(1, task.notes.len());
        assert_eq!("test note", task.notes[0].note);
    }
}
