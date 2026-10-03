//! Bounded command-line configuration for smoke, latency and allocator runs.
use crate::Failure;
use std::path::PathBuf;

pub struct Settings {
    pub directory: PathBuf,
    pub sizes: Vec<usize>,
    pub repetitions: usize,
    pub warmups: usize,
    pub seed: u64,
    pub label: String,
    pub heap: bool,
    pub smoke: bool,
    pub case: Option<String>,
}
impl Settings {
    pub fn smoke(directory: PathBuf) -> Self {
        Self {
            directory,
            sizes: vec![16],
            repetitions: 1,
            warmups: 1,
            seed: 11,
            label: "smoke".into(),
            heap: false,
            smoke: true,
            case: None,
        }
    }
    pub(crate) fn arguments() -> Result<Self, Failure> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let mut settings = Self {
            directory: std::env::temp_dir()
                .join(format!("rom-query-measure-{}-{stamp}", std::process::id())),
            sizes: vec![128, 1024, 8192],
            repetitions: 11,
            warmups: 2,
            seed: 11,
            label: "unlabelled".into(),
            heap: false,
            smoke: false,
            case: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--smoke"=>{settings.sizes=vec![16];settings.repetitions=1;settings.warmups=1;settings.smoke=true;},
                "--heap"=>settings.heap=true,
                "--directory"=>settings.directory=args.next().ok_or("missing directory")?.into(),
                "--sizes"=>settings.sizes=args.next().ok_or("missing sizes")?.split(',').map(str::parse).collect::<Result<_,_>>()?,
                "--repetitions"=>settings.repetitions=args.next().ok_or("missing repetitions")?.parse()?,
                "--warmups"=>settings.warmups=args.next().ok_or("missing warmups")?.parse()?,
                "--seed"=>settings.seed=args.next().ok_or("missing seed")?.parse()?,
                "--label"=>settings.label=args.next().ok_or("missing label")?,
                "--case"=>settings.case=Some(args.next().ok_or("missing case")?),
                _=>return Err(format!("unknown argument {arg}; use --smoke or --sizes 128,1024,8192 --repetitions 11 --seed 11 --directory FRESH_PATH [--heap] [--case NAME] [--label SOURCE]").into()),
            }
        }
        settings.validate()?;
        Ok(settings)
    }
    pub(crate) fn validate(&self) -> Result<(), Failure> {
        if self
            .sizes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != self.sizes.len()
        {
            return Err(
                "dataset sizes must be distinct; every trial needs a fresh database".into(),
            );
        }
        if self.sizes.is_empty()
            || self.sizes.iter().any(|n| *n == 0 || *n > 100_000)
            || !(1..=1000).contains(&self.repetitions)
            || !(1..=100).contains(&self.warmups)
        {
            return Err("invalid bounded trial dimensions".into());
        }
        if cfg!(feature = "heap") != self.heap {
            return Err("heap builds require --heap; latency trials require a build without the heap feature".into());
        }
        if cfg!(debug_assertions) && !self.smoke {
            return Err("non-smoke trials require a release build".into());
        }
        Ok(())
    }
}
