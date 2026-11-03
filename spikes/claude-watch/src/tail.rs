use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Follows an append-only JSONL file and hands back only complete lines.
pub struct Tail {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
}

impl Tail {
    pub fn new(path: PathBuf, from_start: bool) -> Self {
        let offset = if from_start {
            0
        } else {
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        };
        Self {
            path,
            offset,
            partial: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn poll(&mut self) -> io::Result<Vec<Vec<u8>>> {
        let mut file = File::open(&self.path)?;
        let len = file.metadata()?.len();
        if len < self.offset {
            // Rewritten from scratch: start over rather than read from the middle of a different file.
            self.offset = 0;
            self.partial.clear();
        }
        if len == self.offset {
            return Ok(Vec::new());
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut fresh = Vec::new();
        file.read_to_end(&mut fresh)?;
        self.offset += fresh.len() as u64;
        Ok(split_complete(&mut self.partial, &fresh))
    }
}

/// Appends `fresh` to `partial` and returns every finished line; an unfinished last line stays in `partial`.
pub fn split_complete(partial: &mut Vec<u8>, fresh: &[u8]) -> Vec<Vec<u8>> {
    partial.extend_from_slice(fresh);
    let mut lines = Vec::new();
    while let Some(pos) = partial.iter().position(|&b| b == b'\n') {
        let mut line: Vec<u8> = partial.drain(..=pos).collect();
        line.pop();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_unfinished_line_for_later() {
        let mut partial = Vec::new();
        assert_eq!(
            split_complete(&mut partial, b"{\"a\":1}\n{\"b\""),
            vec![b"{\"a\":1}".to_vec()]
        );
        assert_eq!(partial, b"{\"b\"".to_vec());
        assert_eq!(
            split_complete(&mut partial, b":2}\r\n\n"),
            vec![b"{\"b\":2}".to_vec()]
        );
        assert!(partial.is_empty());
    }
}
