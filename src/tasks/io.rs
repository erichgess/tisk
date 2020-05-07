pub fn get_files(path: &std::path::PathBuf) -> std::io::Result<Vec<std::path::PathBuf>> {
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
