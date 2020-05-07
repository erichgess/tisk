use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;

use super::list::{TableFormatter, TableRow};

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

    pub fn print_notes(notes: Vec<&Note>) {
        use console::Term;

        // Get terminal dimensions so that we can compute how wide columns can be and
        // how to format text properly
        // Assume that we'll always have at least 20 columns in the terminal (as even that small
        // would be unuseable for a person.
        let (_, cols) = Term::stdout()
            .size_checked()
            .expect("Could not get terminal details");

        let id_width: usize = 4;

        // Print the column headers
        let mut tf = TableFormatter::new(cols as usize);
        tf.set_columns(vec![("ID", Some(id_width)), ("Note", None)]);
        tf.print_header();

        // print each task, in the order given by the input vector
        let mut idx = 1;
        for note in notes.iter() {
            //Note::print_note(task, idx, id_width, note_width);
            let mut row = TableRow::new();
            row.push(idx);
            row.push(note.note());
            tf.print_row(row);
            idx += 1;
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    id: u32,
    name: String,
    status: Status,

    #[serde(default = "Utc::now")]
    created_at: DateTime<Utc>,

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

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
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

        let s = serde_yaml::to_string(task).unwrap();

        file.write_all(s.as_bytes())
    }

    pub fn read(path: &std::path::PathBuf) -> std::io::Result<Task> {
        let mut file = File::open(path)?;

        let mut s = String::new();
        file.read_to_string(&mut s)?;

        let y = serde_yaml::from_str::<Task>(&s).unwrap(); // TODO: pass this result up to the caller
        Ok(y)
    }
}
