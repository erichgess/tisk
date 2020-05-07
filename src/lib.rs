extern crate chrono;

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
