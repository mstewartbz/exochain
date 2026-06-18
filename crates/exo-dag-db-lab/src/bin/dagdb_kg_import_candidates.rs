use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use exo_dag_db_lab::kg_markdown_manifest::{
    MANIFEST_SCHEMA_VERSION, Manifest, ManifestFile, build_manifest,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: &str = "dagdb_markdown_kg_import_candidates_v1";

fn main() {
    if let Err(error) = run() {
        eprintln!("dagdb_kg_import_candidates_error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args().skip(1).collect())?;
    let manifest = load_manifest(&args)?;
    let candidates = build_candidates(manifest)?;
    let encoded = serde_json::to_string_pretty(&candidates)
        .map_err(|error| format!("serialize candidates: {error}"))?
        + "\n";

    if let Some(output) = args.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create output directory: {error}"))?;
        }
        fs::write(&output, encoded).map_err(|error| format!("write output: {error}"))?;
    } else {
        io::stdout()
            .write_all(encoded.as_bytes())
            .map_err(|error| format!("write stdout: {error}"))?;
    }
    Ok(())
}

struct Args {
    root: PathBuf,
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
}

fn parse_args(raw: Vec<String>) -> Result<Args, String> {
    let mut root = PathBuf::from("KnowledgeGraphs/dag-db");
    let mut manifest = None;
    let mut output = None;
    let mut index = 0;
    while index < raw.len() {
        match raw[index].as_str() {
            "--root" => {
                index += 1;
                root = PathBuf::from(
                    raw.get(index)
                        .ok_or_else(|| "--root requires a value".to_owned())?,
                );
            }
            "--manifest" => {
                index += 1;
                manifest = Some(PathBuf::from(
                    raw.get(index)
                        .ok_or_else(|| "--manifest requires a value".to_owned())?,
                ));
            }
            "--output" => {
                index += 1;
                output = Some(PathBuf::from(
                    raw.get(index)
                        .ok_or_else(|| "--output requires a value".to_owned())?,
                ));
            }
            "-h" | "--help" => {
                println!(
                    "usage: dagdb_kg_import_candidates [--root <path>] [--manifest <path>] [--output <path>]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 1;
    }
    Ok(Args {
        root,
        manifest,
        output,
    })
}

fn load_manifest(args: &Args) -> Result<Manifest, String> {
    let manifest = if let Some(path) = &args.manifest {
        let text = fs::read_to_string(path).map_err(|error| format!("read manifest: {error}"))?;
        serde_json::from_str(&text).map_err(|error| format!("parse manifest: {error}"))?
    } else {
        build_manifest(&args.root)?
    };
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(format!(
            "unsupported manifest schema: {:?}",
            manifest.schema_version
        ));
    }
    Ok(manifest)
}

#[derive(Debug, Serialize)]
struct CandidateReport {
    schema_version: &'static str,
    source_manifest_schema_version: String,
    graph_root: String,
    node_count: usize,
    edge_count: usize,
    unresolved_wikilink_count: usize,
    nodes: Vec<NodeCandidate>,
    edges: Vec<EdgeCandidate>,
    unresolved_wikilinks: Vec<UnresolvedWikilink>,
}

#[derive(Debug, Serialize)]
struct NodeCandidate {
    candidate_id: String,
    path: String,
    title: String,
    document_type: String,
    status: String,
    project_id: String,
    content_sha256: String,
    byte_length: usize,
    catalog_path: Vec<String>,
    frontmatter: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct EdgeCandidate {
    candidate_id: String,
    edge_kind: &'static str,
    source_candidate_id: String,
    source_path: String,
    target_wikilink: String,
    target_candidate_id: String,
    target_path: String,
    resolution_status: String,
}

#[derive(Debug, Serialize)]
struct UnresolvedWikilink {
    source_path: String,
    target_wikilink: String,
    resolution_status: String,
}

fn build_candidates(mut manifest: Manifest) -> Result<CandidateReport, String> {
    manifest
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
    let link_index = build_link_index(&manifest.files);

    let mut nodes = Vec::new();
    let mut path_to_node_id = BTreeMap::new();
    for file_entry in &manifest.files {
        let candidate_id = stable_id("kg_node", &[&file_entry.path]);
        path_to_node_id.insert(file_entry.path.clone(), candidate_id.clone());
        nodes.push(NodeCandidate {
            candidate_id,
            path: file_entry.path.clone(),
            title: file_entry.title.clone(),
            document_type: document_type_for(&file_entry.path, &file_entry.frontmatter),
            status: file_entry
                .frontmatter
                .get("status")
                .cloned()
                .unwrap_or_else(|| "unknown".to_owned()),
            project_id: file_entry
                .frontmatter
                .get("project_id")
                .cloned()
                .unwrap_or_default(),
            content_sha256: file_entry.sha256.clone(),
            byte_length: file_entry.byte_length,
            catalog_path: catalog_path(&file_entry.path),
            frontmatter: file_entry.frontmatter.clone(),
        });
    }

    let mut edges = Vec::new();
    let mut unresolved = Vec::new();
    for file_entry in &manifest.files {
        let source_id = path_to_node_id
            .get(&file_entry.path)
            .ok_or_else(|| format!("missing node id for {}", file_entry.path))?
            .clone();
        for target in &file_entry.wikilinks {
            let matched_paths = link_index.get(target).cloned().unwrap_or_default();
            let (resolution_status, target_path, target_id) = match matched_paths.as_slice() {
                [path] => (
                    "resolved".to_owned(),
                    path.clone(),
                    path_to_node_id
                        .get(path)
                        .ok_or_else(|| format!("missing node id for {path}"))?
                        .clone(),
                ),
                [] => ("unresolved".to_owned(), String::new(), String::new()),
                _ => ("ambiguous".to_owned(), String::new(), String::new()),
            };
            edges.push(EdgeCandidate {
                candidate_id: stable_id("kg_edge", &[&file_entry.path, target]),
                edge_kind: "wikilink",
                source_candidate_id: source_id.clone(),
                source_path: file_entry.path.clone(),
                target_wikilink: target.clone(),
                target_candidate_id: target_id,
                target_path,
                resolution_status: resolution_status.clone(),
            });
            if resolution_status != "resolved" {
                unresolved.push(UnresolvedWikilink {
                    source_path: file_entry.path.clone(),
                    target_wikilink: target.clone(),
                    resolution_status,
                });
            }
        }
    }
    edges.sort_by(|left, right| {
        (left.source_path.as_str(), left.target_wikilink.as_str())
            .cmp(&(right.source_path.as_str(), right.target_wikilink.as_str()))
    });
    unresolved.sort_by(|left, right| {
        (left.source_path.as_str(), left.target_wikilink.as_str())
            .cmp(&(right.source_path.as_str(), right.target_wikilink.as_str()))
    });

    Ok(CandidateReport {
        schema_version: SCHEMA_VERSION,
        source_manifest_schema_version: manifest.schema_version,
        graph_root: manifest.graph_root,
        node_count: nodes.len(),
        edge_count: edges.len(),
        unresolved_wikilink_count: unresolved.len(),
        nodes,
        edges,
        unresolved_wikilinks: unresolved,
    })
}

fn build_link_index(files: &[ManifestFile]) -> BTreeMap<String, Vec<String>> {
    let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file_entry in files {
        for key in link_keys_for_file(file_entry) {
            index.entry(key).or_default().push(file_entry.path.clone());
        }
    }
    for paths in index.values_mut() {
        paths.sort();
    }
    index
}

fn link_keys_for_file(file_entry: &ManifestFile) -> Vec<String> {
    let path = file_entry.path.as_str();
    let path_without_ext = path.strip_suffix(".md").unwrap_or(path);
    let basename_without_ext = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let mut keys = BTreeSet::from([
        path.to_owned(),
        path_without_ext.to_owned(),
        basename_without_ext.to_owned(),
    ]);
    if !file_entry.title.trim().is_empty() {
        keys.insert(file_entry.title.clone());
    }
    keys.into_iter()
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
        .collect()
}

fn catalog_path(path: &str) -> Vec<String> {
    let without_ext = path.strip_suffix(".md").unwrap_or(path);
    without_ext
        .split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn document_type_for(path: &str, frontmatter: &BTreeMap<String, String>) -> String {
    if let Some(explicit) = frontmatter.get("type").map(|value| value.trim()) {
        if !explicit.is_empty() && explicit != "unknown" {
            return explicit.to_owned();
        }
    }

    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let lower_path = path.to_ascii_lowercase();
    let basename = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if stem == "00_Index" {
        "index"
    } else if stem == "01_Project_Brief" {
        "project_brief"
    } else if stem == "00_Pinned_Mission" || lower_path.contains("pinned_mission") {
        "pinned_mission"
    } else if path.ends_with(".plan.md") {
        "plan"
    } else if path.ends_with(".schema.md") {
        "export_contract"
    } else if basename.ends_with("-status.md") {
        "batch_report"
    } else if basename.ends_with("-contract.md") {
        "requirement"
    } else if lower_path.contains("/03_decisions/") || stem.eq_ignore_ascii_case("decision log") {
        "decision"
    } else if lower_path.contains("/08_open_questions/") || lower_path.contains("open-question") {
        "open_question"
    } else if lower_path.contains("milestone") && lower_path.contains("ladder") {
        "milestone_ladder"
    } else if lower_path.contains("/09_exports/") || lower_path.starts_with("09_exports/") {
        "export"
    } else {
        "technical_note"
    }
    .to_owned()
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(parts.join("\0").as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}_{}", &hex[..24])
}
