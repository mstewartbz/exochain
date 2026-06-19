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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir(name: &str) -> PathBuf {
        env::temp_dir()
            .join("exo_dagdb_kg_markdown_manifest_tests")
            .join(format!("{name}_{}", std::process::id()))
    }

    fn reset_dir(path: &Path) {
        let _ = fs::remove_dir_all(path);
        fs::create_dir_all(path).expect("test manifest dir");
    }

    #[test]
    fn kg_markdown_manifest_builds_sorted_manifest_from_markdown_tree() {
        let root = fixture_dir("sorted_tree");
        reset_dir(&root);
        fs::create_dir_all(root.join("nested")).expect("nested dir");

        let alpha = concat!(
            "---\n",
            "frontmatter_title: \"Alpha Frontmatter\"\n",
            "owner: 'dagdb'\n",
            "ignored line without colon\n",
            "---\n",
            "# Alpha Title\n",
            "Links [[b-beta|Beta alias]] and [[nested/c-gamma#Details]].\n",
            "Duplicate [[b-beta]] and empty [[#Only Heading]].\n",
            "`inline [[ignored-inline]] code`\n",
            "```rust\n",
            "[[ignored-fence]]\n",
            "```\n",
            "## Details\n",
            "#### Deep Cut\n",
        );
        let beta = "# Beta Title\nBody [[a-alpha]]\n";
        let gamma = "preamble\n#### Gamma Deep\n";

        fs::write(root.join("a-alpha.md"), alpha).expect("alpha markdown");
        fs::write(root.join("b-beta.md"), beta).expect("beta markdown");
        fs::write(root.join("nested").join("c-gamma.md"), gamma).expect("gamma markdown");
        fs::write(root.join("ignored.txt"), "# Ignored\n").expect("ignored text");

        let first = build_manifest(&root).expect("manifest");
        let second = build_manifest(&root).expect("manifest again");

        assert_eq!(
            serde_json::to_value(&first).expect("first json"),
            serde_json::to_value(&second).expect("second json")
        );
        assert_eq!(first.schema_version, MANIFEST_SCHEMA_VERSION);
        assert_eq!(first.file_count, 3);
        assert!(
            first
                .graph_root
                .ends_with(root.file_name().unwrap().to_str().unwrap())
        );
        assert_eq!(
            first
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["a-alpha.md", "b-beta.md", "nested/c-gamma.md"]
        );

        let alpha_file = &first.files[0];
        assert_eq!(alpha_file.byte_length, alpha.len());
        assert_eq!(alpha_file.sha256, sha256_hex(alpha.as_bytes()));
        assert_eq!(
            alpha_file.frontmatter.get("frontmatter_title"),
            Some(&"Alpha Frontmatter".to_owned())
        );
        assert_eq!(
            alpha_file.frontmatter.get("owner"),
            Some(&"dagdb".to_owned())
        );
        assert!(
            !alpha_file
                .frontmatter
                .contains_key("ignored line without colon")
        );
        assert_eq!(alpha_file.title, "Alpha Title");
        assert_eq!(
            alpha_file
                .headings
                .iter()
                .map(|heading| (heading.level, heading.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "Alpha Title"), (2, "Details"), (4, "Deep Cut")]
        );
        assert_eq!(
            alpha_file.wikilinks,
            vec!["b-beta".to_owned(), "nested/c-gamma".to_owned()]
        );

        assert_eq!(first.files[1].title, "Beta Title");
        assert_eq!(first.files[1].wikilinks, vec!["a-alpha".to_owned()]);
        assert_eq!(first.files[2].title, "Gamma Deep");

        fs::remove_dir_all(root).expect("cleanup manifest dir");
    }

    #[test]
    fn kg_markdown_manifest_reports_invalid_roots_and_non_utf8_markdown() {
        let missing_root = fixture_dir("missing_root");
        let _ = fs::remove_dir_all(&missing_root);
        let missing_error = build_manifest(&missing_root).expect_err("missing root");
        assert!(missing_error.contains("graph root does not exist"));

        let file_root = fixture_dir("file_root");
        let _ = fs::remove_file(&file_root);
        fs::create_dir_all(file_root.parent().expect("file root parent")).expect("parent dir");
        fs::write(&file_root, b"not a directory").expect("file root");
        let file_error = build_manifest(&file_root).expect_err("file root");
        assert!(file_error.contains("graph root is not a directory"));
        fs::remove_file(&file_root).expect("cleanup file root");

        let invalid_utf8_root = fixture_dir("invalid_utf8");
        reset_dir(&invalid_utf8_root);
        fs::write(invalid_utf8_root.join("bad.md"), [0xff]).expect("bad markdown");
        let utf8_error = build_manifest(&invalid_utf8_root).expect_err("utf8 markdown");
        assert!(utf8_error.contains("markdown file is not UTF-8"));
        fs::remove_dir_all(invalid_utf8_root).expect("cleanup invalid utf8 root");
    }

    #[test]
    fn kg_markdown_manifest_parses_frontmatter_headings_and_wikilink_edges() {
        let frontmatter = parse_frontmatter(concat!(
            "---\n",
            "title: 'Quoted Title'\n",
            "owner: \"DAG DB\"\n",
            "# comment\n",
            "no colon\n",
            ": missing key\n",
            "spaced : value: with colon\n",
            "---\n",
            "# Body\n",
        ));
        assert_eq!(frontmatter.get("title"), Some(&"Quoted Title".to_owned()));
        assert_eq!(frontmatter.get("owner"), Some(&"DAG DB".to_owned()));
        assert_eq!(
            frontmatter.get("spaced"),
            Some(&"value: with colon".to_owned())
        );
        assert!(!frontmatter.contains_key("no colon"));
        assert!(!frontmatter.contains_key(""));
        assert!(parse_frontmatter("# Body\n").is_empty());
        assert!(parse_frontmatter("---\ntitle: missing end\n# Body\n").is_empty());

        let headings = extract_headings(concat!(
            "preamble\n",
            "# One\n",
            "#### Four\n",
            "####### Too Many\n",
            "#NoSpace\n",
            "##   \n",
        ));
        assert_eq!(
            headings
                .iter()
                .map(|heading| (heading.level, heading.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "One"), (4, "Four")]
        );

        let wikilinks = extract_wikilinks(concat!(
            "[[Alpha]] [[Alpha|alias]] [[Beta#Section]] [[ spaced ]] [[ ]] ",
            "[[\n",
            "nope]] `[[Code]]`\n",
            "```\n",
            "[[Fence]]\n",
            "```\n",
            "[[NoClose",
        ));
        assert_eq!(
            wikilinks,
            vec!["Alpha".to_owned(), "Beta".to_owned(), "spaced".to_owned()]
        );
        assert_eq!(strip_inline_code("a `hidden [[Link]]` b"), "a  b");
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            repo_relative_to(Path::new("outside.md"), &fixture_dir("root")),
            "outside.md"
        );
        assert!(!display_root(Path::new(".")).contains('\\'));
    }
}
