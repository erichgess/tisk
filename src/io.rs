#[macro_export]
macro_rules! ferror {
    () => {{
        use console::style;
        Err(format!("{}: ", style("Error").red()))
    }};
    ($($arg:tt)*) => {{
        use console::style;
        let preface = format!("{}: ", style("Error").red());
        let msg = format!($($arg)*);
        Err(format!("{}{}", preface, msg))
    }};
}

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

pub fn find_task_dir() -> Result<std::path::PathBuf, String> {
    match up_search(".", ".tisk") {
        Err(why) => ferror!("Failure while searching for .tisk dir: {}", why),
        Ok(path) => match path {
            None => ferror!("Invalid tisk project, could not find .tisk dir in the current directory or any parent directory"),
            Some(path) => Ok(path),
        }
    }
}

pub fn write_checkout(id: u32, path: &std::path::PathBuf) -> std::io::Result<()> {
    use std::io::prelude::*;

    let mut path = std::path::PathBuf::from(path);
    path.push(".checkout");
    let mut file = std::fs::File::create(path)?;

    let s = format!("{}", id);

    file.write_all(s.as_bytes())
}

pub fn read_checkout(path: &std::path::PathBuf) -> std::io::Result<Option<u32>> {
    use std::io::prelude::*;
    let mut path = std::path::PathBuf::from(path);
    path.push(".checkout");
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err @ std::io::Error { .. }) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None)
        }
        Err(err) => return Err(err),
    };

    let mut s = String::new();
    file.read_to_string(&mut s)?;
    s.parse::<u32>()
        .map(|id| Some(id))
        .or_else(|e| Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
}

pub fn write_checkin(path: &std::path::PathBuf) -> std::io::Result<()> {
    let mut path = std::path::PathBuf::from(path);
    path.push(".checkout");

    match std::fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(err @ std::io::Error { .. }) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(())
        }
        Err(err) => return Err(err),
    }
}
