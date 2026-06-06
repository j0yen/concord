//! [`SourceGatherer`] trait and the offline [`FixtureGatherer`] implementation.
//!
//! # Network guarantee
//! [`FixtureGatherer`] never opens a socket. The trait surface is designed so
//! any gatherer that would open a socket must be explicitly constructed by the
//! caller — the default paths in `concord corpus build --fixtures` only reach
//! [`FixtureGatherer`].

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A raw document as returned by a gatherer before normalisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDocument {
    /// Best-guess title (from `<title>` tag, JSON field, or filename stem).
    pub title: String,
    /// Source URL or `file:///<path>` for fixture files.
    pub url: String,
    /// Publisher / domain (empty string if unknown).
    pub publisher: String,
    /// Full body text (HTML stripped by the gatherer if applicable).
    pub body: String,
    /// Optional author string (empty if unknown).
    pub author: String,
    /// Optional publication date in ISO-8601 (empty if unknown).
    pub published_at: String,
}

/// Gather raw documents for a query string.
///
/// Implementors **must not** perform outbound network I/O unless explicitly
/// designed to do so; document this clearly. [`FixtureGatherer`] is the
/// canonical no-network implementation used in tests and offline runs.
pub trait SourceGatherer: Send + Sync {
    /// Return a human-readable name for this gatherer (used in provenance records).
    fn name(&self) -> &str;

    /// Gather documents relevant to `query`.
    ///
    /// # Errors
    /// Returns an error if gathering fails (I/O, network, parse, etc.).
    fn gather(&self, query: &str) -> Result<Vec<RawDocument>>;
}

/// A gatherer that reads pre-downloaded fixture files from a local directory.
///
/// Expected directory layout:
/// ```text
/// <fixtures_dir>/
///   <case_name>/
///     sources.json        ← Vec<RawDocument> (primary format)
///   OR
///     *.json              ← individual RawDocument files
///     *.txt               ← plain-text documents (url = "file:///<path>")
/// ```
///
/// No outbound connections are made. This is the default gatherer for
/// `concord corpus build --fixtures <dir>` and for all tests.
pub struct FixtureGatherer {
    dir: PathBuf,
}

impl FixtureGatherer {
    /// Create a [`FixtureGatherer`] rooted at `dir`.
    ///
    /// # Errors
    /// Returns an error if `dir` does not exist or is not a directory.
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        if !dir.is_dir() {
            anyhow::bail!("fixtures directory does not exist: {}", dir.display());
        }
        Ok(Self { dir })
    }
}

impl SourceGatherer for FixtureGatherer {
    fn name(&self) -> &str {
        "fixture"
    }

    fn gather(&self, query: &str) -> Result<Vec<RawDocument>> {
        // Try sources.json first.
        let sources_json = self.dir.join("sources.json");
        if sources_json.exists() {
            let raw = std::fs::read_to_string(&sources_json)
                .with_context(|| format!("reading {}", sources_json.display()))?;
            let docs: Vec<RawDocument> = serde_json::from_str(&raw)
                .with_context(|| format!("parsing {}", sources_json.display()))?;
            return Ok(docs);
        }

        // Fall back: read individual *.json files in the directory.
        let mut docs = Vec::new();
        let entries = std::fs::read_dir(&self.dir)
            .with_context(|| format!("reading dir {}", self.dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let Some(ext) = path.extension() else { continue };
            if ext == "json" {
                let raw = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                // Try as a single RawDocument.
                if let Ok(doc) = serde_json::from_str::<RawDocument>(&raw) {
                    docs.push(doc);
                }
                // Try as a Vec<RawDocument>.
                else if let Ok(mut ds) = serde_json::from_str::<Vec<RawDocument>>(&raw) {
                    docs.append(&mut ds);
                }
            } else if ext == "txt" {
                let body = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                docs.push(RawDocument {
                    title: stem.to_string(),
                    url: format!("file://{}", path.display()),
                    publisher: String::new(),
                    body,
                    author: String::new(),
                    published_at: String::new(),
                });
            }
        }

        // Tag provenance with the query string by embedding it in url comment field
        // (not stored separately — caller uses ProvenanceRecord for that).
        let _ = query; // query used by real gatherers; fixture uses filesystem
        docs.sort_by(|a, b| a.url.cmp(&b.url)); // deterministic order
        Ok(docs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_fixture(dir: &Path, name: &str, doc: &RawDocument) {
        let path = dir.join(format!("{name}.json"));
        std::fs::write(path, serde_json::to_string_pretty(doc).unwrap()).unwrap();
    }

    fn sample_doc(n: usize) -> RawDocument {
        RawDocument {
            title: format!("Doc {n}"),
            url: format!("https://example.com/{n}"),
            publisher: "example.com".to_string(),
            body: format!("Body of document {n}."),
            author: String::new(),
            published_at: String::new(),
        }
    }

    #[test]
    fn fixture_gatherer_reads_json_files() {
        let tmp = TempDir::new().unwrap();
        write_fixture(tmp.path(), "doc1", &sample_doc(1));
        write_fixture(tmp.path(), "doc2", &sample_doc(2));

        let gatherer = FixtureGatherer::new(tmp.path()).unwrap();
        let docs = gatherer.gather("test query").unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn fixture_gatherer_reads_sources_json() {
        let tmp = TempDir::new().unwrap();
        let docs = vec![sample_doc(1), sample_doc(2), sample_doc(3)];
        std::fs::write(
            tmp.path().join("sources.json"),
            serde_json::to_string_pretty(&docs).unwrap(),
        )
        .unwrap();

        let gatherer = FixtureGatherer::new(tmp.path()).unwrap();
        let result = gatherer.gather("anything").unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn fixture_gatherer_rejects_missing_dir() {
        let result = FixtureGatherer::new("/tmp/definitely_does_not_exist_concord_fixture_zzz");
        assert!(result.is_err());
    }
}
