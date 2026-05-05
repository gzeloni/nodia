use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfig {
    pub name: String,
    pub entry: PathBuf,
}

pub fn find_project_config(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?
    } else {
        start
    };
    loop {
        let candidate = current.join("dobra.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = current.parent()?;
    }
}

pub fn read_project_config(path: &Path) -> io::Result<ProjectConfig> {
    let content = fs::read_to_string(path)?;
    let mut name = None;
    let mut entry = None;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "name" => name = Some(unquote(value.trim()).to_string()),
            "entry" => entry = Some(PathBuf::from(unquote(value.trim()))),
            _ => {}
        }
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    Ok(ProjectConfig {
        name: name.unwrap_or_else(|| "dobra-project".to_string()),
        entry: base.join(entry.unwrap_or_else(|| PathBuf::from("src/main.dob"))),
    })
}

pub fn init_project(dir: &Path) -> io::Result<()> {
    let src = dir.join("src");
    fs::create_dir_all(&src)?;

    let config = dir.join("dobra.toml");
    if !config.exists() {
        fs::write(
            &config,
            "name = \"dobra-project\"\nentry = \"src/main.dob\"\n",
        )?;
    }

    let main = src.join("main.dob");
    if !main.exists() {
        fs::write(&main, "const name = input.name\n\nemit \"Hello, {name}\"\n")?;
    }

    Ok(())
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}
