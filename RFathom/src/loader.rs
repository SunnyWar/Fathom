//! Internal table file discovery and indexing.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::WdlValue;

/// Tablebase file kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableKind {
    Wdl,
    Dtz,
}

/// Metadata for a discovered tablebase file.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct TableFile {
    pub(crate) path: PathBuf,
    pub(crate) material: String,
    pub(crate) kind: TableKind,
    pub(crate) piece_count: usize,
    pub(crate) header: TableHeader,
}

/// Minimal parsed table file header metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableHeader {
    pub(crate) magic: [u8; 4],
    pub(crate) file_len: u64,
}

/// WDL/DTZ pair for a material key.
#[derive(Debug, Clone, Default)]
pub(crate) struct MaterialTables {
    pub(crate) wdl: Option<PathBuf>,
    pub(crate) dtz: Option<PathBuf>,
    pub(crate) piece_count: usize,
}

/// Discovered tablebase index.
#[derive(Debug, Clone, Default)]
pub(crate) struct TableIndex {
    pub(crate) files: Vec<TableFile>,
    pub(crate) by_material: HashMap<String, MaterialTables>,
    pub(crate) largest: usize,
}

impl TableIndex {
    #[cfg(test)]
    pub(crate) fn file_count(&self) -> usize {
        self.files.len()
    }
}

/// Errors produced by table discovery and indexing.
#[derive(Debug)]
pub(crate) enum LoaderError {
    PathNotFound(PathBuf),
    NotADirectory(PathBuf),
    ReadDirFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadDirEntryFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    OpenFileFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadHeaderFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    MapFileFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidHeaderTooSmall {
        path: PathBuf,
        len: u64,
    },
    InvalidWdlPayload {
        path: PathBuf,
    },
    InvalidDtzPayload {
        path: PathBuf,
    },
    InvalidReadRange {
        path: PathBuf,
        offset: usize,
        len: usize,
        file_len: usize,
    },
}

impl fmt::Display for LoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoaderError::PathNotFound(path) => {
                write!(f, "tablebase path does not exist: {}", path.display())
            }
            LoaderError::NotADirectory(path) => {
                write!(f, "tablebase path is not a directory: {}", path.display())
            }
            LoaderError::ReadDirFailed { path, source } => {
                write!(
                    f,
                    "failed to read tablebase directory {}: {}",
                    path.display(),
                    source
                )
            }
            LoaderError::ReadDirEntryFailed { path, source } => {
                write!(
                    f,
                    "failed to read an entry in tablebase directory {}: {}",
                    path.display(),
                    source
                )
            }
            LoaderError::OpenFileFailed { path, source } => {
                write!(
                    f,
                    "failed to open table file {}: {}",
                    path.display(),
                    source
                )
            }
            LoaderError::ReadHeaderFailed { path, source } => {
                write!(
                    f,
                    "failed to read table header {}: {}",
                    path.display(),
                    source
                )
            }
            LoaderError::MapFileFailed { path, source } => {
                write!(
                    f,
                    "failed to mmap table file {}: {}",
                    path.display(),
                    source
                )
            }
            LoaderError::InvalidHeaderTooSmall { path, len } => {
                write!(
                    f,
                    "invalid table header in {}: file too small ({} bytes)",
                    path.display(),
                    len
                )
            }
            LoaderError::InvalidWdlPayload { path } => {
                write!(f, "invalid WDL payload in {}: no entries", path.display())
            }
            LoaderError::InvalidDtzPayload { path } => {
                write!(
                    f,
                    "invalid DTZ payload in {}: malformed entries",
                    path.display()
                )
            }
            LoaderError::InvalidReadRange {
                path,
                offset,
                len,
                file_len,
            } => {
                write!(
                    f,
                    "invalid read range in {}: offset {} len {} (file len {})",
                    path.display(),
                    offset,
                    len,
                    file_len
                )
            }
        }
    }
}

impl std::error::Error for LoaderError {}

/// Load and index table files under a single directory.
pub(crate) fn load_table_index(path: &Path) -> Result<TableIndex, LoaderError> {
    if !path.exists() {
        return Err(LoaderError::PathNotFound(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(LoaderError::NotADirectory(path.to_path_buf()));
    }

    let entries = fs::read_dir(path).map_err(|source| LoaderError::ReadDirFailed {
        path: path.to_path_buf(),
        source,
    })?;

    let mut index = TableIndex::default();

    for entry_result in entries {
        let entry = entry_result.map_err(|source| LoaderError::ReadDirEntryFailed {
            path: path.to_path_buf(),
            source,
        })?;

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }

        let file_name_os = entry.file_name();
        let Some(file_name) = file_name_os.to_str() else {
            continue;
        };

        let Some((material, kind, piece_count)) = parse_table_filename(file_name) else {
            continue;
        };

        let file_path = entry.path();
        let header = parse_table_header(&file_path)?;

        index.largest = index.largest.max(piece_count);

        let table_file = TableFile {
            path: file_path,
            material: material.clone(),
            kind,
            piece_count,
            header,
        };
        index.files.push(table_file.clone());

        let slot = index.by_material.entry(material).or_default();
        slot.piece_count = slot.piece_count.max(piece_count);
        match kind {
            TableKind::Wdl => slot.wdl = Some(table_file.path),
            TableKind::Dtz => slot.dtz = Some(table_file.path),
        }
    }

    Ok(index)
}

/// Look up a WDL value from a table file using a deterministic index key.
///
/// Current vertical-slice format:
/// - bytes [0..4] are header magic (already validated by loader)
/// - bytes [4..] are direct WDL entries encoded as 0..=4
pub(crate) fn probe_wdl_value(
    table_path: &Path,
    key: u64,
) -> Result<Option<WdlValue>, LoaderError> {
    let reader = TableReader::open(table_path)?;
    if reader.len() <= 4 {
        return Err(LoaderError::InvalidWdlPayload {
            path: table_path.to_path_buf(),
        });
    }

    let payload_len = reader.len() - 4;
    let idx = (key as usize) % payload_len;
    let mut raw = [0u8; 1];
    reader.read_exact_at(4 + idx, &mut raw)?;

    Ok(WdlValue::from_u32(raw[0] as u32))
}

/// Look up a DTZ value from a table file using a deterministic index key.
///
/// Current vertical-slice format:
/// - bytes [0..4] are header magic (already validated by loader)
/// - bytes [4..] are little-endian i16 DTZ entries
pub(crate) fn probe_dtz_value(table_path: &Path, key: u64) -> Result<i32, LoaderError> {
    let reader = TableReader::open(table_path)?;
    if reader.len() <= 5 {
        return Err(LoaderError::InvalidDtzPayload {
            path: table_path.to_path_buf(),
        });
    }

    let payload_len = reader.len() - 4;
    if payload_len % 2 != 0 {
        return Err(LoaderError::InvalidDtzPayload {
            path: table_path.to_path_buf(),
        });
    }

    let entry_count = payload_len / 2;
    let idx = (key as usize) % entry_count;
    let mut raw = [0u8; 2];
    reader.read_exact_at(4 + (idx * 2), &mut raw)?;
    Ok(i16::from_le_bytes(raw) as i32)
}

fn parse_table_header(path: &Path) -> Result<TableHeader, LoaderError> {
    let reader = TableReader::open(path)?;
    let len = reader.len() as u64;
    if len < 4 {
        return Err(LoaderError::InvalidHeaderTooSmall {
            path: path.to_path_buf(),
            len,
        });
    }

    let mut magic = [0u8; 4];
    reader.read_exact_at(0, &mut magic)?;

    Ok(TableHeader {
        magic,
        file_len: len,
    })
}

enum TableData {
    Mmap(memmap2::Mmap),
    Buffered(Vec<u8>),
}

impl TableData {
    fn as_slice(&self) -> &[u8] {
        match self {
            TableData::Mmap(m) => m,
            TableData::Buffered(v) => v,
        }
    }
}

struct TableReader {
    path: PathBuf,
    data: TableData,
}

impl TableReader {
    fn open(path: &Path) -> Result<Self, LoaderError> {
        let mut file = fs::File::open(path).map_err(|source| LoaderError::OpenFileFailed {
            path: path.to_path_buf(),
            source,
        })?;

        // Mmap is preferred for probing workloads; fallback keeps tests and edge systems working.
        let data = match unsafe { memmap2::MmapOptions::new().map(&file) } {
            Ok(mmap) => TableData::Mmap(mmap),
            Err(map_err) => {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|source| LoaderError::ReadHeaderFailed {
                        path: path.to_path_buf(),
                        source,
                    })?;

                if buf.is_empty() {
                    return Err(LoaderError::MapFileFailed {
                        path: path.to_path_buf(),
                        source: map_err,
                    });
                }

                TableData::Buffered(buf)
            }
        };

        Ok(Self {
            path: path.to_path_buf(),
            data,
        })
    }

    fn len(&self) -> usize {
        self.data.as_slice().len()
    }

    fn read_exact_at(&self, offset: usize, out: &mut [u8]) -> Result<(), LoaderError> {
        let len = self.len();
        let end = offset.saturating_add(out.len());
        if end > len {
            return Err(LoaderError::InvalidReadRange {
                path: self.path.clone(),
                offset,
                len: out.len(),
                file_len: len,
            });
        }
        out.copy_from_slice(&self.data.as_slice()[offset..end]);
        Ok(())
    }
}

fn parse_table_filename(file_name: &str) -> Option<(String, TableKind, usize)> {
    let lower = file_name.to_ascii_lowercase();
    let (stem, kind) = if let Some(stem) = lower.strip_suffix(".rtbw") {
        (stem, TableKind::Wdl)
    } else if let Some(stem) = lower.strip_suffix(".rtbz") {
        (stem, TableKind::Dtz)
    } else {
        return None;
    };

    if !stem.contains('v') {
        return None;
    }

    let piece_count = stem
        .chars()
        .filter(|c| c.is_ascii_alphabetic() && *c != 'v')
        .count();
    if piece_count == 0 {
        return None;
    }

    Some((stem.to_string(), kind, piece_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic enough for tests")
            .as_nanos();
        path.push(format!("rfathom_loader_test_{}", nonce));
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn parses_known_table_name() {
        let (material, kind, pieces) =
            parse_table_filename("KQvK.rtbw").expect("table name should parse");
        assert_eq!(material, "kqvk");
        assert_eq!(kind, TableKind::Wdl);
        assert_eq!(pieces, 3);
    }

    #[test]
    fn ignores_non_table_name() {
        assert!(parse_table_filename("README.md").is_none());
        assert!(parse_table_filename("KQvK.bin").is_none());
    }

    #[test]
    fn loads_and_indexes_tables() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), b"WDL0payload").expect("wdl file should be created");
        std::fs::write(dir.join("KQvK.rtbz"), b"DTZ0payload").expect("dtz file should be created");
        std::fs::write(dir.join("ignore.txt"), b"x").expect("other file should be created");

        let index = load_table_index(&dir).expect("index load should succeed");
        assert_eq!(index.file_count(), 2);
        assert_eq!(index.largest, 3);

        let tables = index
            .by_material
            .get("kqvk")
            .expect("material key should exist");
        assert!(tables.wdl.is_some());
        assert!(tables.dtz.is_some());

        let wdl = index
            .files
            .iter()
            .find(|f| f.kind == TableKind::Wdl)
            .expect("wdl file entry should exist");
        assert_eq!(wdl.header.magic, *b"WDL0");

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn missing_path_is_error() {
        let mut missing = std::env::temp_dir();
        missing.push("rfathom_loader_definitely_missing_path");

        let err = load_table_index(&missing).expect_err("missing path should error");
        assert!(matches!(err, LoaderError::PathNotFound(_)));
    }

    #[test]
    fn malformed_header_is_error() {
        let dir = create_temp_dir();
        std::fs::write(dir.join("KQvK.rtbw"), b"x").expect("short file should be created");

        let err = load_table_index(&dir).expect_err("short header should error");
        assert!(matches!(err, LoaderError::InvalidHeaderTooSmall { .. }));

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn reader_rejects_out_of_bounds_read() {
        let dir = create_temp_dir();
        let path = dir.join("KQvK.rtbw");
        std::fs::write(&path, b"WDL0").expect("file should be created");

        let reader = TableReader::open(&path).expect("reader should open");
        let mut out = [0u8; 4];
        let err = reader
            .read_exact_at(2, &mut out)
            .expect_err("out of bounds read should fail");
        assert!(matches!(err, LoaderError::InvalidReadRange { .. }));

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn probes_wdl_value_from_payload() {
        let dir = create_temp_dir();
        let path = dir.join("KQvK.rtbw");
        std::fs::write(&path, [b'W', b'D', b'L', b'0', 4, 2, 0])
            .expect("table file should be created");

        let v0 = probe_wdl_value(&path, 0).expect("probe should succeed");
        let v1 = probe_wdl_value(&path, 1).expect("probe should succeed");
        let v2 = probe_wdl_value(&path, 2).expect("probe should succeed");

        assert_eq!(v0, Some(WdlValue::Win));
        assert_eq!(v1, Some(WdlValue::Draw));
        assert_eq!(v2, Some(WdlValue::Loss));

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }

    #[test]
    fn probes_dtz_value_from_payload() {
        let dir = create_temp_dir();
        let path = dir.join("KQvK.rtbz");
        std::fs::write(&path, [b'D', b'T', b'Z', b'0', 1, 0, 0xFC, 0xFF, 0, 0])
            .expect("table file should be created");

        let v0 = probe_dtz_value(&path, 0).expect("probe should succeed");
        let v1 = probe_dtz_value(&path, 1).expect("probe should succeed");
        let v2 = probe_dtz_value(&path, 2).expect("probe should succeed");

        assert_eq!(v0, 1);
        assert_eq!(v1, -4);
        assert_eq!(v2, 0);

        std::fs::remove_dir_all(&dir).expect("temp dir should be removed");
    }
}
