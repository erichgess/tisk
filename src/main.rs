use log4rs;
use log::{debug};
use clap::{App, Arg};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashSet;

fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default())
        .expect("Failed to configure logger");

    let args = App::new("Tisk")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Task Management with scoping")
        .subcommand(
            App::new("add")
            .arg(
                Arg::with_name("input")
                .index(1)
                .required(true)))
        .subcommand(
            App::new("close")
            .arg(
                Arg::with_name("ID")
                .index(1)
            )
        )
        .subcommand(
            App::new("init")
        )
        .get_matches();

    if args.subcommand_matches("init").is_some() {
        match initialize() {
            Ok(InitResult::Initialized) => println!("Initialized directory"),
            Ok(InitResult::AlreadyInitialized) => println!("Already initialized"),
            Err(why) => panic!("Failed to initialize tisk project: {}", why),
        }
    } else {
        let task_path = match up_search(".task") {
            Ok(path) => match path {
                Some(p) => p,
                None => panic!("Invalid tisk project, could not found .task in this directory or any parent directory"),
            },
            Err(why) => panic!("Failure while searching for .task dir: {}", why),
        };

        let mut tasks = match get_files(&task_path) {
            Err(why) => panic!("Failed to get YAML files: {}", why),
            Ok(files) => {
                TaskList::read_tasks(files).unwrap()
            }
        };

        if let Some(ref matches) = args.subcommand_matches("add") {
            debug!("Adding new task to task list");
            let name =  matches.value_of("input").unwrap();
            tasks.add_task(name).expect("Failed to create new task");
        } else if let Some(ref done) = args.subcommand_matches("close") {
            let id: u32 = done.value_of("ID").unwrap().parse().unwrap();
            debug!("Closing task with ID: {}", id);
            tasks.close_task(id).expect("Could not find given ID");
        } else {
            tasks.print();
        }
        debug!("Writing tasks");
        match tasks.write_all(&task_path) {
            Ok(_) => (),
            Err(why) => panic!("Failed to write tasks: {}", why),
        }
    }
}

fn get_files(path: &std::path::PathBuf) -> std::io::Result<Vec<std::path::PathBuf>> {
    use std::fs;

    let contents = fs::read_dir(path)?;
    let yaml_files = contents.filter(|f| f.as_ref().unwrap().path().extension().map(|e| e == "yaml").unwrap_or(false));
    let mut files = vec![];
    for yaml in yaml_files {
        let file = yaml?;
        files.push(file.path());
    }

    Ok(files)
}

enum InitResult {
    Initialized,
    AlreadyInitialized,
}

fn initialize() -> std::io::Result<InitResult> {
        match std::fs::read_dir("./.task") {
            Ok(_) => Ok(InitResult::AlreadyInitialized),
            Err(_) => match std::fs::create_dir("./.task") {
                Err(why) => Err(why),
                Ok(_) => Ok(InitResult::Initialized),
            }
        }
}

fn up_search(file_name: &str) -> std::io::Result<Option<std::path::PathBuf>> {
    let path = std::fs::canonicalize(".")?;

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
                },
                Err(_) => false
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

#[derive(Debug, Serialize, Deserialize)]
enum Status {
    Open,
    Closed,
}

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: u32,
    name: String,
    status: Status,
}

impl Task {
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

        let y = serde_yaml::from_str::<Task>(&s).unwrap();
        Ok(y)
    }

}

struct TaskList {
    tasks: Vec<Task>,
    modified_tasks: HashSet<u32>,
}

impl TaskList {
    fn read_tasks(paths: Vec<std::path::PathBuf>) -> std::io::Result<TaskList> {
        let mut tasks = vec![];
        for path in paths.into_iter() {
            let task = Task::read(&path)?;
            tasks.push(task);
        }
        Ok(TaskList{
            tasks,
            modified_tasks: HashSet::new(),
        })
    }

    fn next_id(&self) -> u32 {
        if self.tasks.len() == 0 {
            1
        } else {
            let mut largest_id = self.tasks[0].id;
            for task in self.tasks.iter() {
                if task.id > largest_id {
                    largest_id = task.id;
                }
            }
            largest_id+1
        }
    }

    /**
     * Searches the `TaskList` for a task with the given ID.  If a matching
     * task is found: then return a mutable reference to that task and mark
     * the task as modified.
     *
     * If no task is found with the given ID then return `None`.
     */
    fn get_mut(&mut self, id: u32) -> Option<&mut Task> {
        debug!("Get mut");
        let task = self.tasks.iter_mut().find(|t| t.id == id);

        // Assume that any call to get a mutable reference to a task
        // will result in that task being modified.
        match &task {
            Some(task) => {
                debug!("Adding id to the set of modified tasks");
                self.modified_tasks.insert(task.id);
            },
            None => ()
        }
        task
    }

    fn get(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    fn add_task(&mut self, name: &str) -> Option<&Task> {
        let id = self.next_id();
        let t = Task{
            id: id,
            name: String::from(name),
            status: Status::Open,
        };
        self.tasks.push(t);
        self.modified_tasks.insert(id);
        self.get(id)
    }

    fn close_task(&mut self, id: u32) -> Option<&Task> {
        match self.get_mut(id) {
            None => None,
            Some(task) => {
                task.status = Status::Closed;
                Some(task)
            }
        }
    }

    fn write_all(self, task_path: &std::path::PathBuf) -> std::io::Result<u32> {
        debug!("Tasks to write: {}", self.modified_tasks.len());
        let mut count = 0;
        for id in self.modified_tasks.iter() {
            match self.get(*id) {
                Some(task) => {
                    Task::write(task, &task_path)?;
                    count += 1;
                },
                None => (),
            }
        }
        Ok(count)
    }

    #[allow(dead_code)]
    fn print(&self) {
        use console::Style;
        use console::Term;

        // Get terminal dimensions so that we can compute how wide columns can be and 
        // how to format text properly
        // Assume that we'll always have at least 20 columns in the terminal (as even that small
        // would be unuseable for a person.
        let (_, cols) = Term::stdout().size_checked().expect("Could not get terminal details");

        let id_width = 4;
        let name_width:usize = if (cols - 5) < 16 {16} else {cols as usize-5}; // subtract id_width + 1 to account for a space between columns

        // Print the column headers
        let ul = Style::new().underlined();
        println!("{0: <id_width$} {1: <name_width$}", 
            ul.apply_to("ID"), ul.apply_to("Name"), 
            id_width = id_width, name_width = name_width);

        for task in self.tasks.iter() {
            // Check the length of the name, if it is longer than `name_width` it will need to be
            // printed on multiple lines
            let lines = TaskList::format_to_column(&task.name, name_width, 5);
            let mut first_line = true;
            for line in lines {
                if first_line {
                    print!("{0: <id_width$} ", task.id, id_width = id_width);
                } else {
                    print!("{0: <id_width$} ", "", id_width = id_width);
                }
                println!("{0: <name_width$}", 
                    line,
                    name_width = name_width);
                first_line = false;
            }
        }
    }

    /**
     * Takes a given string and formats it into a vector of strings
     * such that each string is no longer than the given width.  It will
     * attempt to break lines at spaces but if a word is longer than
     * the given column width it will split on the word.
     */
    fn format_to_column(text: &String, width: usize, split_limit: usize) -> Vec<&str>{
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
            if c.is_whitespace() || index == text.len() || (index - word_start) > width{
                word_end = index;  // whitespace will be added to the current word until a new word starts or the end of the column is reached
                let word_len = word_end - word_start;

                if word_len + (end - start) <= width {
                    end = word_end;
                    if index == text.len() {
                        breaks.push((start, end));
                    }
                } else {
                    let splittable = if split_limit < width {word_len > split_limit} else {true};
                    if  splittable && word_len + (end - start) > width {
                        end = word_start + (width - (end-start));
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
            let end = if b.1 > text.len() {text.len()} else {b.1};
            lines.push(text.get(start..end).unwrap());
        }
        lines
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
}
