use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MANIFEST_SCHEMA_VERSION: &str = "dagdb_markdown_kg_manifest_v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub schema_version: String,
    pub graph_root: String,
    pub file_count: usize,
    pub files: Vec<ManifestFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManifestFile {
    pub path: String,
    pub sha256: String,
    pub byte_length: usize,
    pub frontmatter: BTreeMap<String, String>,
    pub title: String,
    pub headings: Vec<Heading>,
    pub wikilinks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Heading {
    pub level: usize,
    pub text: String,
}

pub fn build_manifest(root: &Path) -> Result<Manifest, String> {
    if !root.exists() {
        return Err(format!("graph root does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("graph root is not a directory: {}", root.display()));
    }

    let mut paths = Vec::new();
    collect_markdown_files(root, &mut paths)?;
    paths.sort_by_key(|path| repo_relative_to(path, root));

    let mut files = Vec::new();
    for path in paths {
        let data = fs::read(&path).map_err(|error| format!("read markdown file: {error}"))?;
        let text = String::from_utf8(data.clone())
            .map_err(|error| format!("markdown file is not UTF-8: {error}"))?;
        let headings = extract_headings(&text);
        files.push(ManifestFile {
            path: repo_relative_to(&path, root),
            sha256: sha256_hex(&data),
            byte_length: data.len(),
            frontmatter: parse_frontmatter(&text),
            title: headings
                .first()
                .map(|heading| heading.text.clone())
                .unwrap_or_default(),
            headings,
            wikilinks: extract_wikilinks(&text),
        });
    }

    Ok(Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_owned(),
        graph_root: display_root(root),
        file_count: files.len(),
        files,
    })
}

fn collect_markdown_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| format!("read directory: {error}"))? {
        let entry = entry.map_err(|error| format!("read directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
    Ok(())
}

fn repo_relative_to(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn display_root(root: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match root.strip_prefix(cwd.canonicalize().unwrap_or(cwd)) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_) => root.to_string_lossy().replace('\\', "/"),
    }
}

fn parse_frontmatter(text: &str) -> BTreeMap<String, String> {
    let mut frontmatter = BTreeMap::new();
    let Some(rest) = text.strip_prefix("---\n") else {
        return frontmatter;
    };
    let Some(end) = rest.find("\n---\n") else {
        return frontmatter;
    };
    for raw_line in rest[..end].lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains(':') {
            continue;
        }
        let mut parts = line.splitn(2, ':');
        let key = parts.next().unwrap_or_default().trim();
        let value = parts
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if !key.is_empty() {
            frontmatter.insert(key.to_owned(), value.to_owned());
        }
    }
    frontmatter
}

fn extract_headings(text: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    for line in text.lines() {
        let hashes = line.chars().take_while(|ch| *ch == '#').count();
        if !(1..=6).contains(&hashes) {
            continue;
        }
        let rest = &line[hashes..];
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let title = rest.trim();
        if !title.is_empty() {
            headings.push(Heading {
                level: hashes,
                text: title.to_owned(),
            });
        }
    }
    headings
}

fn extract_wikilinks(text: &str) -> Vec<String> {
    let text = strip_inline_code(&strip_fenced_code(text));
    let mut links = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if &bytes[index..index + 2] != b"[[" {
            index += 1;
            continue;
        }
        let Some(end) = text[index + 2..].find("]]") else {
            break;
        };
        let raw = &text[index + 2..index + 2 + end];
        if !raw.contains('\n') {
            let without_alias = raw.split_once('|').map(|(left, _)| left).unwrap_or(raw);
            let target = without_alias
                .split_once('#')
                .map(|(left, _)| left)
                .unwrap_or(without_alias)
                .trim();
            if !target.is_empty() {
                links.insert(target.to_owned());
            }
        }
        index += end + 4;
    }
    links.into_iter().collect()
}

fn strip_fenced_code(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_fence = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_fence = !in_fence;
            output.push('\n');
            continue;
        }
        if !in_fence {
            output.push_str(line);
        }
        output.push('\n');
    }
    output
}

fn strip_inline_code(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_code = false;
    for ch in text.chars() {
        if ch == '`' {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            output.push(ch);
        }
    }
    output
}

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
