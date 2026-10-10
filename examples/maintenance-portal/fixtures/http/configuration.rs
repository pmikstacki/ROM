use std::path::PathBuf;

pub(super) struct Configuration {
    pub(super) database: String,
    pub(super) path: PathBuf,
    pub(super) ready: PathBuf,
    pub(super) stop: PathBuf,
}

impl Configuration {
    pub(super) fn read() -> Result<Self, Box<dyn std::error::Error>> {
        let arguments: Vec<_> = std::env::args_os().collect();
        if arguments.len() != 6 || arguments[1] != "--fixture-only" {
            return Err("expected --fixture-only <sqlite|redb> <database> <ready> <stop>".into());
        }
        let database = arguments[2].to_str().ok_or("invalid adapter")?.to_owned();
        if !matches!(database.as_str(), "sqlite" | "redb") {
            return Err("unsupported fixture adapter".into());
        }
        let paths: Vec<PathBuf> = arguments[3..].iter().map(PathBuf::from).collect();
        let parent = paths[0]
            .parent()
            .ok_or("fixture directory required")?
            .canonicalize()?;
        for path in &paths {
            if !path.is_absolute()
                || path
                    .parent()
                    .ok_or("fixture directory required")?
                    .canonicalize()?
                    != parent
            {
                return Err("fixture paths must share one existing absolute directory".into());
            }
        }
        if paths[0] == paths[1]
            || paths[0] == paths[2]
            || paths[1] == paths[2]
            || paths[1].exists()
            || paths[2].exists()
        {
            return Err("fixture ready/stop paths must be distinct and unused".into());
        }
        Ok(Self {
            database,
            path: paths[0].clone(),
            ready: paths[1].clone(),
            stop: paths[2].clone(),
        })
    }
}
