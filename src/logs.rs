use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result};

const FOLLOW_INTERVAL: Duration = Duration::from_millis(200);

pub fn read(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading log {}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn tail_lines(text: &str, count: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(count);
    lines[start..].iter().map(|line| line.to_string()).collect()
}

/// Last lines of a log, or nothing when the log cannot be read.
pub fn tail(path: &str, count: usize) -> Vec<String> {
    read(Path::new(path))
        .map(|text| tail_lines(&text, count))
        .unwrap_or_default()
}

pub fn follow(path: &Path, mut offset: u64) -> Result<()> {
    let mut stdout = std::io::stdout();
    loop {
        sleep(FOLLOW_INTERVAL);
        let mut file =
            File::open(path).with_context(|| format!("reading log {}", path.display()))?;
        let len = file.metadata()?.len();
        if len < offset {
            offset = 0;
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut chunk = Vec::new();
        file.read_to_end(&mut chunk)?;
        offset += chunk.len() as u64;
        stdout.write_all(&chunk)?;
        stdout.flush()?;
    }
}
