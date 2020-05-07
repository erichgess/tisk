extern crate chrono;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;

pub fn up_search(dir: &str, file_name: &str) -> std::io::Result<Option<std::path::PathBuf>> {
    let path = std::fs::canonicalize(dir)?;

    let mut found = None;

    for parent in path.ancestors() {
        let mut files = parent.read_dir()?;
        found = files.find(|f| {
            let file = f.as_ref().unwrap();
            let meta = file.metadata();
            match meta {
                Ok(md) => {
                    let ty = md.file_type();
                    if ty.is_dir() {
                        file.file_name() == file_name
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        });

        if found.is_some() {
            break;
        }
    }

    match found {
        Some(result) => match result {
            Err(why) => Err(why),
            Ok(v) => Ok(Some(v.path())),
        },
        None => Ok(None),
    }
}

#[derive(Debug, PartialEq)]
pub enum InitResult {
    Initialized,
    AlreadyInitialized,
}

pub fn initialize() -> std::io::Result<InitResult> {
    match std::fs::read_dir("./.tisk") {
        Ok(_) => Ok(InitResult::AlreadyInitialized),
        Err(_) => match std::fs::create_dir("./.tisk") {
            Err(why) => Err(why),
            Ok(_) => Ok(InitResult::Initialized),
        },
    }
}

fn get_files(path: &std::path::PathBuf) -> std::io::Result<Vec<std::path::PathBuf>> {
    use std::fs;

    let contents = fs::read_dir(path)?;
    let yaml_files = contents.filter(|f| {
        f.as_ref()
            .unwrap()
            .path()
            .extension()
            .map(|e| e == "yaml")
            .unwrap_or(false)
    });
    let mut files = vec![];
    for yaml in yaml_files {
        let file = yaml?;
        files.push(file.path());
    }

    Ok(files)
}

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
        let note_width: usize = if (cols as usize - (id_width + 1)) < 16 {
            16
        } else {
            cols as usize - (id_width + 1)
        }; // subtract id_width + 1 to account for a space between columns

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
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }

    fn write(task: &Task, path: &std::path::PathBuf) -> std::io::Result<()> {
        let mut path = std::path::PathBuf::from(path);
        path.push(format!("{}.yaml", task.id));
        let mut file = File::create(path)?;

        let s = serde_yaml::to_string(task).unwrap();

        file.write_all(s.as_bytes())
    }

    fn read(path: &std::path::PathBuf) -> std::io::Result<Task> {
        let mut file = File::open(path)?;

        let mut s = String::new();
        file.read_to_string(&mut s)?;

        let y = serde_yaml::from_str::<Task>(&s).unwrap(); // TODO: pass this result up to the caller
        Ok(y)
    }

    pub fn add_note(&mut self, note: &str) {
        self.notes.push(Note::new(note));
    }

    pub fn notes(&self) -> Vec<&Note> {
        self.notes.iter().collect()
    }
}

pub struct TaskList {
    tasks: Vec<Task>,
}

impl TaskList {
    pub fn new() -> TaskList {
        TaskList { tasks: vec![] }
    }
    pub fn read_tasks(path: &std::path::PathBuf) -> std::io::Result<TaskList> {
        let paths = get_files(path)?;
        let mut tasks = vec![];
        for p in paths.into_iter() {
            let task = Task::read(&p)?;
            tasks.push(task);
        }
        Ok(TaskList { tasks })
    }

    pub fn next_id(&self) -> u32 {
        if self.tasks.len() == 0 {
            1
        } else {
            let mut largest_id = self.tasks[0].id;
            for task in self.tasks.iter() {
                if task.id > largest_id {
                    largest_id = task.id;
                }
            }
            largest_id + 1
        }
    }

    /**
     * Searches the `TaskList` for a task with the given ID.  If a matching
     * task is found: then return a mutable reference to that task and mark
     * the task as modified.
     *
     * If no task is found with the given ID then return `None`.
     */
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn get(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn add_task(&mut self, name: &str, priority: u32) -> u32 {
        let id = self.next_id();
        let t = Task {
            id: id,
            name: String::from(name),
            status: Status::Open,
            created_at: Utc::now(),
            priority: priority,
            notes: vec![],
        };
        self.tasks.push(t);

        id
    }

    pub fn close_task(&mut self, id: u32) -> Option<&Task> {
        match self.get_mut(id) {
            None => None,
            Some(task) => {
                task.status = Status::Closed;
                Some(task)
            }
        }
    }

    pub fn set_priority(&mut self, id: u32, priority: u32) -> Option<(Task, &Task)> {
        match self.get_mut(id) {
            None => None,
            Some(task) => {
                let old = task.clone();
                task.priority = priority;
                Some((old, task))
            }
        }
    }

    pub fn add_note(&mut self, id: u32, note: &str) -> Option<&Task> {
        match self.get_mut(id) {
            Some(task) => {
                task.add_note(note);
                Some(task)
            }
            None => None,
        }
    }

    pub fn write_all(&self, task_path: &std::path::PathBuf) -> std::io::Result<u32> {
        let mut count = 0;
        for task in self.tasks.iter() {
            Task::write(task, &task_path)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn print(tasks: Vec<&Task>) {
        use console::Term;

        // Get terminal dimensions so that we can compute how wide columns can be and
        // how to format text properly
        // Assume that we'll always have at least 20 columns in the terminal (as even that small
        // would be unuseable for a person.
        let (_, cols) = Term::stdout()
            .size_checked()
            .expect("Could not get terminal details");

        let id_width: usize = 4;
        let date_width: usize = 10; // YYYY-mm-dd
        let priority_width: usize = 3;
        let name_width: usize =
            if (cols as usize - (id_width + 1 + date_width + 1 + priority_width + 1)) < 16 {
                16
            } else {
                cols as usize - (id_width + 1) - (date_width + 1) - (priority_width + 1)
            }; // subtract id_width + 1 to account for a space between columns

        // Print the column headers
        TaskList::print_header(id_width, date_width, priority_width, name_width);

        // print each task, in the order given by the input vector
        for task in tasks.iter() {
            TaskList::print_task(task, id_width, date_width, priority_width, name_width);
        }
    }

    fn print_header(id_width: usize, date_width: usize, priority_width: usize, name_width: usize) {
        use console::Style;
        let ul = Style::new().underlined();
        println!(
            "{0: <id_width$} {1: <date_width$} {2: <name_width$} {3: <priority_width$}",
            ul.apply_to("ID"),
            ul.apply_to("Date"),
            ul.apply_to("Name"),
            ul.apply_to("Pri"),
            id_width = id_width,
            date_width = date_width,
            name_width = name_width,
            priority_width = priority_width,
        );
    }

    fn print_task(
        task: &Task,
        id_width: usize,
        date_width: usize,
        priority_width: usize,
        name_width: usize,
    ) {
        // Check the length of the name, if it is longer than `name_width` it will need to be
        // printed on multiple lines
        let lines = TaskList::format_to_column(&task.name, name_width, 7);
        let mut first_line = true;
        for line in lines {
            if first_line {
                print!("{0: <id_width$} ", task.id, id_width = id_width);
                let date = task.created_at.format("%Y-%m-%d");
                print!("{0: <date_width$} ", date, date_width = date_width);
            } else {
                print!("{0: <id_width$} ", "", id_width = id_width);
                print!("{0: <date_width$} ", "", date_width = date_width);
            }

            print!("{0: <name_width$} ", line, name_width = name_width);

            if first_line {
                print!(
                    "{0: <priority_width$}",
                    task.priority,
                    priority_width = priority_width
                );
            } else {
                print!("{0: <priority_width$}", "", priority_width = priority_width);
            }
            println!();
            first_line = false;
        }
    }

    pub fn get_all(&self) -> Vec<&Task> {
        self.tasks.iter().collect()
    }

    pub fn get_open(&self) -> Vec<&Task> {
        self.filter(Status::Open)
    }

    pub fn get_closed(&self) -> Vec<&Task> {
        self.filter(Status::Closed)
    }

    pub fn filter(&self, status: Status) -> Vec<&Task> {
        let iter = self.tasks.iter();
        let filtered_tasks: Vec<&Task> = iter.filter(|t| t.status == status).collect();
        filtered_tasks
    }

    /**
     * Takes a given string and formats it into a vector of strings
     * such that each string is no longer than the given width.  It will
     * attempt to break lines at spaces but if a word is longer than
     * the given column width it will split on the word.
     */
    fn format_to_column(text: &String, width: usize, split_limit: usize) -> Vec<String> {
        let mut index = 0;
        let mut chars = text.chars();
        let mut breaks = vec![];
        let mut start = 0;
        let mut end = 0;
        let mut word_start = 0;
        let mut word_end;

        while let Some(c) = chars.next() {
            index += 1;

            // if is whitespace then we are at the end of a word
            //    if word + length of current line < width then add word to line
            //    if else if word > width then hyphenate word
            //    else start new line and add word to that
            if c.is_whitespace() || index == text.len() || (index - word_start) > width {
                word_end = index; // whitespace will be added to the current word until a new word starts or the end of the column is reached
                let word_len = word_end - word_start;

                if word_len + (end - start) <= width {
                    end = word_end;
                    if index == text.len() {
                        breaks.push((start, end));
                    }
                } else {
                    let splittable = if split_limit < width {
                        word_len > split_limit
                    } else {
                        true
                    };
                    if splittable && word_len + (end - start) > width {
                        end = word_start + (width - (end - start));
                        breaks.push((start, end));
                        start = end;
                        end = word_end;
                    } else {
                        breaks.push((start, end));
                        start = word_start;
                        end = word_end;
                    }
                    if end == text.len() {
                        breaks.push((start, end));
                    }
                }

                word_start = word_end;
            }
        }

        let mut lines = vec![];
        for b in breaks {
            let start = b.0;
            let end = if b.1 > text.len() { text.len() } else { b.1 };
            lines.push(text.get(start..end).unwrap().into());
        }
        lines
    }
}

struct TableRow<'a> {
    row: Vec<Box<dyn std::fmt::Display + 'a>>,
}

impl<'a> TableRow<'a> {
    pub fn new() -> Self {
        Self { row: Vec::new() }
    }

    pub fn push<S: std::fmt::Display + 'a>(&mut self, col: S) {
        self.row.push(Box::new(col))
    }
}

struct TableFormatter {
    width: usize, // the width, in characters, of the table
    col_widths: Vec<usize>,
    cols: Vec<String>,
}

impl TableFormatter {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            col_widths: Vec::new(),
            cols: Vec::new(),
        }
    }

    pub fn set_columns(&mut self, cols: Vec<(&str, Option<usize>)>) {
        // Add up the widths of the explicitly defined columns
        // adding 1 to account for a space between each column
        let with_width = cols.iter().filter(|x| x.1.is_some());
        let allocated_width: usize = with_width.map(|x| x.1.unwrap() + 1).sum();

        // Count the number of columsn without a width
        let without_width = cols.iter().filter(|x| x.1.is_none());
        let num_without_width = without_width.count();

        // Get the amount of space which is not explicitly assigned to a column
        // Divide evenly between the columns without width
        if self.width < allocated_width {
            panic!("Total width of columns is greater than the width of the table")
        }
        let remaining_space = self.width - allocated_width;
        let width_per_col = remaining_space / num_without_width;

        // Record the columns and their widths
        for (label, width) in cols {
            match width {
                Some(w) => {
                    self.cols.push(String::from(label));
                    self.col_widths.push(w);
                }
                None => {
                    self.cols.push(String::from(label));
                    self.col_widths.push(width_per_col);
                }
            }
        }
    }

    pub fn print_header(&self) {
        use console::Style;
        let ul = Style::new().underlined();
        let num_cols = self.cols.len();
        for i in 0..num_cols {
            print!(
                "{0: <width$}",
                ul.apply_to(&self.cols[i]),
                width = self.col_widths[i]
            );
            if i < num_cols - 1 {
                print!(" ");
            }
        }
        println!();
    }

    pub fn print_row(&self, cols: TableRow) {
        let mut longest_column = 1;
        let mut col_text = vec![];
        for i in 0..cols.row.len() {
            let text = cols.row[i].to_string().clone();
            let fitted_text = TaskList::format_to_column(&text, self.col_widths[i], 7);
            if fitted_text.len() > longest_column {
                longest_column = fitted_text.len();
            }
            col_text.push(fitted_text.clone());
        }

        for line in 0..longest_column {
            for col in 0..cols.row.len() {
                if line < col_text[col].len() {
                    print!(
                        "{0: <width$}",
                        col_text[col][line],
                        width = self.col_widths[col]
                    );
                } else {
                    print!("{0: <width$}", "", width = self.col_widths[col]);
                }

                if col < cols.row.len() - 1 {
                    print!(" ");
                }
            }
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_short_words() {
        let text = String::from("the quick brown fox");
        let lines = TaskList::format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox", lines[1]);
    }

    #[test]
    fn split_short_words_multiple_spaces() {
        let text = String::from("the quick  brown fox   jumped   ");
        let lines = TaskList::format_to_column(&text, 10, 5);
        assert_eq!(4, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!(" brown ", lines[1]);
        assert_eq!("fox   jump", lines[2]);
        assert_eq!("ed   ", lines[3]);
    }

    #[test]
    fn split_short_words_whitepsace_longer_than_column() {
        let text = String::from("the            fox");
        let lines = TaskList::format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the       ", lines[0]);
        assert_eq!("     fox", lines[1]);
    }

    #[test]
    fn no_split() {
        let text = String::from("the quick");
        let lines = TaskList::format_to_column(&text, 10, 5);
        assert_eq!(1, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick", lines[0]);
    }

    #[test]
    fn split_many_words() {
        let text = String::from("the quick brown fox jumped over the lazy dog");
        let lines = TaskList::format_to_column(&text, 10, 5);
        assert_eq!(5, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox ", lines[1]);
        assert_eq!("jumped ", lines[2]);
        assert_eq!("over the ", lines[3]);
        assert_eq!("lazy dog", lines[4]);
    }

    #[test]
    fn split_word_longer_than_min_but_smaller_than_column_width() {
        let text = String::from("the quick brown fox fast jumped over the lazy dog");
        let lines = TaskList::format_to_column(&text, 10, 5);
        assert_eq!(6, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox ", lines[1]);
        assert_eq!("fast jumpe", lines[2]);
        assert_eq!("d over ", lines[3]);
        assert_eq!("the lazy ", lines[4]);
        assert_eq!("dog", lines[5]);
    }

    #[test]
    fn split_word_longer_than_column_width() {
        let text = String::from("argleybargley");
        let lines = TaskList::format_to_column(&text, 10, 5);
        assert_eq!(2, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("argleybarg", lines[0]);
        assert_eq!("ley", lines[1]);
    }

    #[test]
    fn split_word_longer_than_column_width_shorter_than_min_word() {
        let text = String::from("bark");
        let lines = TaskList::format_to_column(&text, 3, 5);
        assert_eq!(2, lines.len());
        //          123    <- column numbers
        assert_eq!("bar", lines[0]);
        assert_eq!("k", lines[1]);
    }

    #[test]
    fn split_word_change_limit() {
        let text = String::from("the quick brown fox fast jumped over the lazy dog");
        let lines = TaskList::format_to_column(&text, 10, 7);
        assert_eq!(6, lines.len());
        //          1234567890    <- column numbers
        assert_eq!("the quick ", lines[0]);
        assert_eq!("brown fox ", lines[1]);
        assert_eq!("fast ", lines[2]);
        assert_eq!("jumped ", lines[3]);
        assert_eq!("over the ", lines[4]);
        assert_eq!("lazy dog", lines[5]);
    }

    #[test]
    fn add_task() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            tasks = mtasks;
        }

        let t = tasks.get(1).unwrap();
        assert_eq!(1, t.id());
        assert_eq!(1, t.priority());
        assert_eq!("test", t.name());
        assert_eq!(Status::Open, t.status());

        let t2 = tasks.get(2).unwrap();
        assert_eq!(2, t2.id());
        assert_eq!(2, t2.priority());
        assert_eq!("test 2", t2.name());
        assert_eq!(Status::Open, t2.status());
    }

    #[test]
    fn close_task() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let t = tasks.get(1).unwrap();
        assert_eq!(1, t.id());
        assert_eq!(1, t.priority());
        assert_eq!("test", t.name());
        assert_eq!(Status::Closed, t.status());

        let t2 = tasks.get(2).unwrap();
        assert_eq!(2, t2.id());
        assert_eq!(2, t2.priority());
        assert_eq!("test 2", t2.name());
        assert_eq!(Status::Open, t2.status());
    }

    #[test]
    fn change_priority() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            let (old, new) = mtasks.set_priority(t, 4).expect("The old and new tasks");
            assert_eq!(1, old.priority());
            assert_eq!(1, old.id());
            assert_eq!("test", old.name());
            assert_eq!(4, new.priority());
            assert_eq!(1, new.id());
            assert_eq!("test", new.name());

            // set priority for a task which does not exist
            match mtasks.set_priority(3, 2) {
                None => (),
                Some(_) => panic!("None should be returned when changing priority for a task which does not exist"),
            }

            tasks = mtasks;
        }

        let t = tasks.get(1).unwrap();
        assert_eq!(1, t.id());
        assert_eq!(4, t.priority());
        assert_eq!("test", t.name());
        assert_eq!(Status::Open, t.status());

        let t2 = tasks.get(2).unwrap();
        assert_eq!(2, t2.id());
        assert_eq!(2, t2.priority());
        assert_eq!("test 2", t2.name());
        assert_eq!(Status::Open, t2.status());
    }

    #[test]
    fn get_open() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let filtered_tasks = tasks.get_open();
        assert_eq!(1, filtered_tasks.len());
        assert_eq!("test 2", filtered_tasks[0].name());
        assert_eq!(Status::Open, filtered_tasks[0].status());
    }

    #[test]
    fn get_closed() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let filtered_tasks = tasks.get_closed();
        assert_eq!(1, filtered_tasks.len());
        assert_eq!("test", filtered_tasks[0].name());
        assert_eq!(Status::Closed, filtered_tasks[0].status());
    }

    #[test]
    fn get_all() {
        let tasks;
        {
            let mut mtasks = TaskList::new();
            let t = mtasks.add_task("test", 1);
            mtasks.add_task("test 2", 2);
            mtasks.close_task(t);
            tasks = mtasks;
        }

        let filtered_tasks = tasks.get_all();
        assert_eq!(2, filtered_tasks.len());
        assert_eq!("test", filtered_tasks[0].name());
        assert_eq!(Status::Closed, filtered_tasks[0].status());
        assert_eq!("test 2", filtered_tasks[1].name());
        assert_eq!(Status::Open, filtered_tasks[1].status());
    }

    #[test]
    fn task_notes() {
        let mut mtasks = TaskList::new();
        let t = mtasks.add_task("test", 1);
        let task = mtasks.get_mut(t).expect("Task not created");

        task.add_note("Test Note");
        let notes = task.notes();
        assert_eq!(1, notes.len());
        assert_eq!("Test Note", notes[0].note);

        task.add_note("Second Note");
        let notes = task.notes();
        assert_eq!(2, notes.len());
        assert_eq!("Test Note", notes[0].note);
        assert_eq!("Second Note", notes[1].note);
    }
}
