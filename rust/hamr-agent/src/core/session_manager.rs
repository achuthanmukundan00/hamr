//! Session persistence — append-only trees stored in JSONL files.
//! Ported from `packages/coding-agent/src/core/session-manager.ts`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const CURRENT_SESSION_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    pub id: String,
    pub timestamp: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessageEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(default)]
    pub message: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub summary: String,
    #[serde(rename = "firstKeptEntryId")]
    pub first_kept_entry_id: String,
    #[serde(rename = "tokensBefore")]
    pub tokens_before: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "fromHook")]
    pub from_hook: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummaryEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "fromId")]
    pub from_id: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "fromHook")]
    pub from_hook: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingLevelChangeEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChangeEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub provider: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "customType")]
    pub custom_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMessageEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "customType")]
    pub custom_type: String,
    pub content: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub display: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "targetId")]
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfoEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPointEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "childSessionId")]
    pub child_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "childSessionPath")]
    pub child_session_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "runId")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "taskPreview")]
    pub task_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SessionEntry {
    Message(SessionMessageEntry),
    ThinkingLevelChange(ThinkingLevelChangeEntry),
    ModelChange(ModelChangeEntry),
    Compaction(CompactionEntry),
    BranchSummary(BranchSummaryEntry),
    Custom(CustomEntry),
    CustomMessage(CustomMessageEntry),
    Label(LabelEntry),
    SessionInfo(SessionInfoEntry),
    SpawnPoint(SpawnPointEntry),
}

impl SessionEntry {
    pub fn id(&self) -> &str {
        match self {
            Self::Message(e) => &e.id,
            Self::ThinkingLevelChange(e) => &e.id,
            Self::ModelChange(e) => &e.id,
            Self::Compaction(e) => &e.id,
            Self::BranchSummary(e) => &e.id,
            Self::Custom(e) => &e.id,
            Self::CustomMessage(e) => &e.id,
            Self::Label(e) => &e.id,
            Self::SessionInfo(e) => &e.id,
            Self::SpawnPoint(e) => &e.id,
        }
    }
    pub fn parent_id(&self) -> Option<&str> {
        match self {
            Self::Message(e) => e.parent_id.as_deref(),
            Self::ThinkingLevelChange(e) => e.parent_id.as_deref(),
            Self::ModelChange(e) => e.parent_id.as_deref(),
            Self::Compaction(e) => e.parent_id.as_deref(),
            Self::BranchSummary(e) => e.parent_id.as_deref(),
            Self::Custom(e) => e.parent_id.as_deref(),
            Self::CustomMessage(e) => e.parent_id.as_deref(),
            Self::Label(e) => e.parent_id.as_deref(),
            Self::SessionInfo(e) => e.parent_id.as_deref(),
            Self::SpawnPoint(e) => e.parent_id.as_deref(),
        }
    }
    pub fn timestamp(&self) -> &str {
        match self {
            Self::Message(e) => &e.timestamp,
            Self::ThinkingLevelChange(e) => &e.timestamp,
            Self::ModelChange(e) => &e.timestamp,
            Self::Compaction(e) => &e.timestamp,
            Self::BranchSummary(e) => &e.timestamp,
            Self::Custom(e) => &e.timestamp,
            Self::CustomMessage(e) => &e.timestamp,
            Self::Label(e) => &e.timestamp,
            Self::SessionInfo(e) => &e.timestamp,
            Self::SpawnPoint(e) => &e.timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub messages: Vec<serde_json::Value>,
    pub thinking_level: String,
    pub model: Option<ModelInfo>,
}
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub provider: String,
    pub model_id: String,
}
#[derive(Debug, Clone)]
pub struct SessionTreeNode {
    pub entry: SessionEntry,
    pub children: Vec<SessionTreeNode>,
    pub label: Option<String>,
    pub label_timestamp: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct NewSessionOptions {
    pub id: Option<String>,
    pub parent_session: Option<String>,
}

fn create_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn assert_valid_session_id(id: &str) {
    let valid = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !id.is_empty()
        && id
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_alphanumeric())
        && id
            .chars()
            .last()
            .map_or(false, |c| c.is_ascii_alphanumeric());
    if !valid {
        panic!("Invalid session id");
    }
}

fn generate_entry_id(existing: &HashSet<String>) -> String {
    for _ in 0..100 {
        let id: String = uuid::Uuid::new_v4().to_string().chars().take(8).collect();
        if !existing.contains(&id) {
            return id;
        }
    }
    uuid::Uuid::new_v4().to_string()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn resolve_cwd(raw: &str) -> String {
    Path::new(raw)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(raw))
        .to_string_lossy()
        .to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileEntry {
    Session(SessionHeader),
    Entry(serde_json::Value),
}

pub fn parse_session_entries(content: &str) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for line in content.trim().lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if val["type"] == "session" {
                if let Ok(h) = serde_json::from_value::<SessionHeader>(val) {
                    entries.push(FileEntry::Session(h));
                }
            } else {
                entries.push(FileEntry::Entry(val));
            }
        }
    }
    entries
}

pub fn load_entries_from_file(file_path: &Path) -> Vec<FileEntry> {
    if !file_path.exists() {
        return vec![];
    }
    let file = match fs::File::open(file_path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut entries = Vec::new();
    for line in BufReader::new(file).lines() {
        if let Ok(line) = line {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if val["type"] == "session" {
                    if let Ok(h) = serde_json::from_value::<SessionHeader>(val) {
                        entries.push(FileEntry::Session(h));
                    }
                } else {
                    entries.push(FileEntry::Entry(val));
                }
            }
        }
    }
    if entries.is_empty() {
        return entries;
    }
    match &entries[0] {
        FileEntry::Session(h) if h.entry_type == "session" => entries,
        _ => vec![],
    }
}

fn read_session_header(file_path: &Path) -> Option<SessionHeader> {
    let file = fs::File::open(file_path).ok()?;
    let mut fl = String::new();
    BufReader::new(file).read_line(&mut fl).ok()?;
    if fl.is_empty() {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(&fl).ok()?;
    if val["type"] != "session" {
        return None;
    }
    serde_json::from_value(val).ok()
}

pub fn find_most_recent_session(session_dir: &Path, cwd: Option<&str>) -> Option<String> {
    let dir_entries = fs::read_dir(session_dir).ok()?;
    let resolved_cwd = cwd
        .and_then(|c| Path::new(c).canonicalize().ok())
        .map(|p| p.to_string_lossy().to_string());
    let mut files: Vec<(String, std::time::SystemTime)> = Vec::new();
    for entry in dir_entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "jsonl") {
            if let Some(header) = read_session_header(&path) {
                if let Some(ref target) = resolved_cwd {
                    if !header.cwd.is_empty() {
                        if let Ok(resolved) = Path::new(&header.cwd).canonicalize() {
                            if resolved.to_string_lossy() != *target {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                }
                if let Ok(meta) = fs::metadata(&path) {
                    if let Ok(mtime) = meta.modified() {
                        files.push((path.to_string_lossy().to_string(), mtime));
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| b.1.cmp(&a.1));
    files.first().map(|(p, _)| p.clone())
}

pub fn get_default_session_dir_path(cwd: &str) -> String {
    get_default_session_dir_path_for_agent(cwd, &crate::config::get_agent_dir())
        .to_string_lossy()
        .to_string()
}

/// Compute the cwd-specific session directory under an explicit agent dir.
pub fn get_default_session_dir_path_for_agent(cwd: &str, agent_dir: &Path) -> PathBuf {
    let resolved = resolve_cwd(cwd);
    let safe = format!(
        "--{}--",
        resolved
            .trim_start_matches('/')
            .replace(['/', '\\', ':'], "-")
    );
    agent_dir.join("sessions").join(safe)
}

fn get_default_session_dir(cwd: &str) -> String {
    let d = get_default_session_dir_path(cwd);
    let _ = fs::create_dir_all(&d);
    d
}

pub fn get_latest_compaction_entry(entries: &[SessionEntry]) -> Option<&CompactionEntry> {
    entries.iter().rev().find_map(|e| {
        if let SessionEntry::Compaction(c) = e {
            Some(c)
        } else {
            None
        }
    })
}

fn entry_to_message(entry: &SessionEntry) -> Option<serde_json::Value> {
    match entry {
        SessionEntry::Message(e) => {
            if e.message.get("role").and_then(|r| r.as_str()) == Some("bashExecution") {
                return None;
            }
            Some(e.message.clone())
        }
        SessionEntry::CustomMessage(e) => Some(
            serde_json::json!({"role":"custom","customType":e.custom_type,"content":e.content,"display":e.display,"details":e.details,"timestamp":e.timestamp}),
        ),
        SessionEntry::BranchSummary(e) if !e.summary.is_empty() => Some(
            serde_json::json!({"role":"branchSummary","summary":e.summary,"fromId":e.from_id,"timestamp":e.timestamp}),
        ),
        _ => None,
    }
}

pub fn build_session_context(
    entries: &[SessionEntry],
    leaf_id: Option<&str>,
    by_id_arg: Option<&HashMap<String, usize>>,
) -> SessionContext {
    let leaf = match leaf_id {
        None => entries.last(),
        Some("null") | Some("") => None,
        Some(id) => {
            let found = if let Some(m) = by_id_arg {
                m.get(id).and_then(|&i| entries.get(i))
            } else {
                entries.iter().find(|e| e.id() == id)
            };
            found.or_else(|| entries.last())
        }
    };
    let Some(leaf) = leaf else {
        return SessionContext {
            messages: vec![],
            thinking_level: "off".to_string(),
            model: None,
        };
    };
    let by_id: HashMap<String, usize> = match by_id_arg {
        Some(m) => m.clone(),
        None => entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id().to_string(), i))
            .collect(),
    };
    let mut path = Vec::new();
    let mut cur = by_id.get(leaf.id()).copied();
    while let Some(i) = cur {
        path.push(i);
        cur = entries[i].parent_id().and_then(|p| by_id.get(p)).copied();
    }
    path.reverse();
    let (mut tl, mut model, mut comp_idx) = ("off".to_string(), None, None);
    for &i in &path {
        match &entries[i] {
            SessionEntry::ThinkingLevelChange(e) => tl = e.thinking_level.clone(),
            SessionEntry::ModelChange(e) => {
                model = Some(ModelInfo {
                    provider: e.provider.clone(),
                    model_id: e.model_id.clone(),
                })
            }
            SessionEntry::Compaction(_) => comp_idx = Some(i),
            _ => {}
        }
    }
    let msgs = if let Some(ci) = comp_idx {
        let mut m = Vec::new();
        if let SessionEntry::Compaction(c) = &entries[ci] {
            m.push(serde_json::json!({"role":"compactionSummary","summary":c.summary,"tokensBefore":c.tokens_before,"timestamp":c.timestamp}));
        }
        if let SessionEntry::Compaction(c) = &entries[ci] {
            let fk = &c.first_kept_entry_id;
            let cp = path.iter().position(|&x| x == ci).unwrap_or(0);
            let mut found = false;
            for (p, &i) in path.iter().enumerate() {
                if p >= cp {
                    break;
                }
                if entries[i].id() == fk {
                    found = true;
                }
                if found {
                    if let Some(msg) = entry_to_message(&entries[i]) {
                        m.push(msg);
                    }
                }
            }
        }
        let cp = path.iter().position(|&x| x == ci).unwrap_or(0);
        for &i in path.iter().skip(cp + 1) {
            if let Some(msg) = entry_to_message(&entries[i]) {
                m.push(msg);
            }
        }
        m
    } else {
        path.iter()
            .filter_map(|&i| entry_to_message(&entries[i]))
            .collect()
    };
    SessionContext {
        messages: msgs,
        thinking_level: tl,
        model,
    }
}

pub struct SessionManager {
    session_id: String,
    session_file: Option<PathBuf>,
    session_dir: PathBuf,
    cwd: String,
    persist: bool,
    flushed: bool,
    file_entries: Vec<FileEntry>,
    by_id: HashMap<String, usize>,
    labels_by_id: HashMap<String, String>,
    label_timestamps_by_id: HashMap<String, String>,
    leaf_id: Option<String>,
}

fn file_entry_to_session(entry: &FileEntry) -> Option<SessionEntry> {
    match entry {
        FileEntry::Session(_) => None,
        FileEntry::Entry(val) => {
            let t = val["type"].as_str()?;
            match t {
                "message" => serde_json::from_value::<SessionMessageEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::Message),
                "thinking_level_change" => {
                    serde_json::from_value::<ThinkingLevelChangeEntry>(val.clone())
                        .ok()
                        .map(SessionEntry::ThinkingLevelChange)
                }
                "model_change" => serde_json::from_value::<ModelChangeEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::ModelChange),
                "compaction" => serde_json::from_value::<CompactionEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::Compaction),
                "branch_summary" => serde_json::from_value::<BranchSummaryEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::BranchSummary),
                "custom" => serde_json::from_value::<CustomEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::Custom),
                "custom_message" => serde_json::from_value::<CustomMessageEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::CustomMessage),
                "label" => serde_json::from_value::<LabelEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::Label),
                "session_info" => serde_json::from_value::<SessionInfoEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::SessionInfo),
                "spawn_point" => serde_json::from_value::<SpawnPointEntry>(val.clone())
                    .ok()
                    .map(SessionEntry::SpawnPoint),
                _ => None,
            }
        }
    }
}

fn migrate_v1_to_v2(entries: &mut Vec<FileEntry>) {
    let mut ids = HashSet::new();
    // Pre-extract all entry IDs (before they get new ones) for compaction index resolution
    let entry_ids: Vec<Option<String>> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            if i == 0 {
                None
            } else {
                match e {
                    FileEntry::Entry(val) => val["id"].as_str().map(|s| s.to_string()),
                    _ => None,
                }
            }
        })
        .collect();
    let mut prev_id: Option<String> = None;
    for entry in entries.iter_mut() {
        match entry {
            FileEntry::Session(header) => {
                header.version = Some(2);
            }
            FileEntry::Entry(val) => {
                let id = generate_entry_id(&ids);
                ids.insert(id.clone());
                val["id"] = serde_json::Value::String(id.clone());
                val["parentId"] = match &prev_id {
                    Some(pid) => serde_json::Value::String(pid.clone()),
                    None => serde_json::Value::Null,
                };
                prev_id = Some(id.clone());
                if val["type"] == "compaction" {
                    if let Some(index) = val["firstKeptEntryIndex"].as_u64() {
                        let idx = index as usize;
                        if let Some(Some(tid)) = entry_ids.get(idx) {
                            val["firstKeptEntryId"] = serde_json::Value::String(tid.clone());
                        }
                        if let Some(obj) = val.as_object_mut() {
                            obj.remove("firstKeptEntryIndex");
                        }
                    }
                }
            }
        }
    }
}

fn migrate_v2_to_v3(entries: &mut Vec<FileEntry>) {
    for entry in entries.iter_mut() {
        match entry {
            FileEntry::Session(header) => {
                header.version = Some(3);
            }
            FileEntry::Entry(val) => {
                if val["type"] == "message"
                    && val["message"]["role"].as_str() == Some("hookMessage")
                {
                    val["message"]["role"] = serde_json::Value::String("custom".to_string());
                }
            }
        }
    }
}

fn migrate_to_current_version(entries: &mut Vec<FileEntry>) -> bool {
    let version = entries
        .first()
        .and_then(|e| match e {
            FileEntry::Session(h) => h.version,
            _ => None,
        })
        .unwrap_or(1);
    if version >= CURRENT_SESSION_VERSION {
        return false;
    }
    if version < 2 {
        migrate_v1_to_v2(entries);
    }
    if version < 3 {
        migrate_v2_to_v3(entries);
    }
    true
}

pub fn migrate_session_entries(entries: &mut Vec<FileEntry>) {
    migrate_to_current_version(entries);
}

impl SessionManager {
    fn new_internal(
        cwd: &str,
        sd: &Path,
        sf: Option<&Path>,
        persist: bool,
        opts: Option<&NewSessionOptions>,
    ) -> Self {
        let rc = resolve_cwd(cwd);
        let sdir = sd.to_path_buf();
        let mut s = Self {
            session_id: String::new(),
            session_file: None,
            session_dir: sdir.clone(),
            cwd: rc,
            persist,
            flushed: false,
            file_entries: Vec::new(),
            by_id: HashMap::new(),
            labels_by_id: HashMap::new(),
            label_timestamps_by_id: HashMap::new(),
            leaf_id: None,
        };
        if persist && !s.session_dir.exists() {
            let _ = fs::create_dir_all(&s.session_dir);
        }
        if let Some(fp) = sf {
            s.set_session_file(fp);
        } else {
            s.new_session(opts);
        }
        s
    }

    pub fn create(cwd: &str, sd: Option<&Path>) -> Self {
        let d = sd
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(get_default_session_dir(cwd)));
        Self::new_internal(cwd, &d, None, true, None)
    }

    pub fn create_with_options(
        cwd: &str,
        sd: Option<&Path>,
        opts: Option<&NewSessionOptions>,
    ) -> Self {
        let d = sd
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(get_default_session_dir(cwd)));
        Self::new_internal(cwd, &d, None, true, opts)
    }

    pub fn open(path: &Path, sd: Option<&Path>, cwd_override: Option<&str>) -> Self {
        let rp = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let entries = load_entries_from_file(&rp);
        let hdr = entries.first().and_then(|e| match e {
            FileEntry::Session(h) => Some(h),
            _ => None,
        });
        let c = cwd_override
            .map(|c| c.to_string())
            .or_else(|| hdr.map(|h| h.cwd.clone()).filter(|c| !c.is_empty()))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default()
            });
        let d = sd
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| rp.parent().map(|p| p.to_path_buf()).unwrap_or_default());
        Self::new_internal(&c, &d, Some(&rp), true, None)
    }

    pub fn continue_recent(cwd: &str, sd: Option<&Path>) -> Self {
        let d = sd
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(get_default_session_dir(cwd)));
        let dd = get_default_session_dir_path(cwd);
        let fc = sd.is_some() && d.to_string_lossy() != dd;
        if let Some(r) = find_most_recent_session(&d, if fc { Some(cwd) } else { None }) {
            Self::new_internal(cwd, &d, Some(Path::new(&r)), true, None)
        } else {
            Self::new_internal(cwd, &d, None, true, None)
        }
    }

    pub fn in_memory(cwd: Option<&str>) -> Self {
        let c = cwd.map(|c| c.to_string()).unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        });
        Self::new_internal(&c, Path::new(""), None, false, None)
    }

    pub fn fork_from(
        source_path: &Path,
        target_cwd: &str,
        sd: Option<&Path>,
        opts: Option<&NewSessionOptions>,
    ) -> Self {
        let rs = source_path
            .canonicalize()
            .unwrap_or_else(|_| source_path.to_path_buf());
        let rt = Path::new(target_cwd)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(target_cwd));
        let se = load_entries_from_file(&rs);
        if se.is_empty() {
            panic!("Cannot fork: source session file is empty or invalid");
        }
        if se
            .first()
            .and_then(|e| match e {
                FileEntry::Session(_) => Some(()),
                _ => None,
            })
            .is_none()
        {
            panic!("Cannot fork: source session has no header");
        }
        let d = sd
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(get_default_session_dir(target_cwd)));
        if !d.exists() {
            let _ = fs::create_dir_all(&d);
        }
        let nsid = opts
            .and_then(|o| o.id.clone())
            .unwrap_or_else(create_session_id);
        if let Some(ref id) = opts.and_then(|o| o.id.as_ref()) {
            assert_valid_session_id(id);
        }
        let ts = now_iso();
        let fts = ts.replace([':', '.'], "-");
        let nf = d.join(format!("{}_{}.jsonl", fts, nsid));
        let nh = SessionHeader {
            entry_type: "session".to_string(),
            version: Some(CURRENT_SESSION_VERSION),
            id: nsid,
            timestamp: ts,
            cwd: rt.to_string_lossy().to_string(),
            parent_session: Some(rs.to_string_lossy().to_string()),
        };
        let mut ct = serde_json::to_string(&nh).unwrap();
        ct.push('\n');
        for e in &se {
            if let FileEntry::Entry(v) = e {
                ct.push_str(&serde_json::to_string(v).unwrap());
                ct.push('\n');
            }
        }
        let _ = fs::write(&nf, &ct);
        Self::new_internal(&rt.to_string_lossy().to_string(), &d, Some(&nf), true, None)
    }

    pub fn is_persisted(&self) -> bool {
        self.persist
    }
    pub fn get_cwd(&self) -> String {
        self.cwd.clone()
    }
    pub fn get_session_dir(&self) -> String {
        self.session_dir.to_string_lossy().to_string()
    }
    pub fn get_session_id(&self) -> String {
        self.session_id.clone()
    }
    pub fn get_session_file(&self) -> Option<String> {
        self.session_file
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
    }
    pub fn get_leaf_id(&self) -> Option<String> {
        self.leaf_id.clone()
    }
    pub fn get_header(&self) -> Option<&SessionHeader> {
        self.file_entries.first().and_then(|e| match e {
            FileEntry::Session(h) => Some(h),
            _ => None,
        })
    }
    pub fn get_entries(&self) -> Vec<SessionEntry> {
        self.file_entries
            .iter()
            .filter_map(|e| file_entry_to_session(e))
            .collect()
    }
    pub fn get_entry(&self, id: &str) -> Option<SessionEntry> {
        self.by_id.get(id).and_then(|&i| {
            if i < self.file_entries.len() {
                file_entry_to_session(&self.file_entries[i])
            } else {
                None
            }
        })
    }
    pub fn get_leaf_entry(&self) -> Option<SessionEntry> {
        self.leaf_id.as_ref().and_then(|id| self.get_entry(id))
    }
    pub fn get_branch(&self, from_id: Option<&str>) -> Vec<SessionEntry> {
        let sid = from_id.or(self.leaf_id.as_deref());
        let Some(start) = sid else { return vec![] };
        let mut path = Vec::new();
        let mut cur = self.by_id.get(start).copied();
        while let Some(i) = cur {
            if let Some(e) = file_entry_to_session(&self.file_entries[i]) {
                path.push(e.clone());
                cur = path
                    .last()
                    .and_then(|e| e.parent_id())
                    .and_then(|p| self.by_id.get(p))
                    .copied();
            } else {
                break;
            }
        }
        path.reverse();
        path
    }
    pub fn build_session_context(&self) -> SessionContext {
        let entries = self.get_entries();
        build_session_context(&entries, self.leaf_id.as_deref(), None)
    }
    pub fn get_session_name(&self) -> Option<String> {
        self.get_entries().iter().rev().find_map(|e| {
            if let SessionEntry::SessionInfo(s) = e {
                s.name.clone().filter(|n| !n.is_empty())
            } else {
                None
            }
        })
    }
    pub fn get_children(&self, parent_id: &str) -> Vec<SessionEntry> {
        self.file_entries
            .iter()
            .filter_map(|e| {
                if let FileEntry::Entry(v) = e {
                    if v["parentId"].as_str() == Some(parent_id) {
                        file_entry_to_session(e)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn get_label(&self, id: &str) -> Option<String> {
        self.labels_by_id.get(id).cloned()
    }

    fn set_session_file(&mut self, fp: &Path) {
        self.session_file = Some(fp.canonicalize().unwrap_or_else(|_| fp.to_path_buf()));
        if self.session_file.as_ref().map_or(false, |f| f.exists()) {
            self.file_entries = load_entries_from_file(self.session_file.as_ref().unwrap());
            if self.file_entries.is_empty() {
                let sv = self.session_file.clone();
                self.new_session(None);
                self.session_file = sv;
                self._rewrite_file();
                self.flushed = true;
                return;
            }
            self.session_id = self
                .get_header()
                .map(|h| h.id.clone())
                .unwrap_or_else(create_session_id);
            if migrate_to_current_version(&mut self.file_entries) {
                self._rewrite_file();
            }
            self._build_index();
            self.flushed = true;
        } else {
            let sv = self.session_file.clone();
            self.new_session(None);
            self.session_file = sv;
        }
    }

    pub fn new_session(&mut self, opts: Option<&NewSessionOptions>) -> Option<String> {
        if let Some(id) = opts.and_then(|o| o.id.as_ref()) {
            assert_valid_session_id(id);
        }
        self.session_id = opts
            .and_then(|o| o.id.clone())
            .unwrap_or_else(create_session_id);
        let ts = now_iso();
        let hdr = SessionHeader {
            entry_type: "session".to_string(),
            version: Some(CURRENT_SESSION_VERSION),
            id: self.session_id.clone(),
            timestamp: ts.clone(),
            cwd: self.cwd.clone(),
            parent_session: opts.and_then(|o| o.parent_session.clone()),
        };
        self.file_entries = vec![FileEntry::Session(hdr)];
        self.by_id.clear();
        self.labels_by_id.clear();
        self.leaf_id = None;
        self.flushed = false;
        if self.persist {
            let fts = ts.replace([':', '.'], "-");
            self.session_file = Some(
                self.session_dir
                    .join(format!("{}_{}.jsonl", fts, self.session_id)),
            );
        }
        self.session_file
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
    }

    fn _build_index(&mut self) {
        self.by_id.clear();
        self.labels_by_id.clear();
        self.label_timestamps_by_id.clear();
        self.leaf_id = None;
        for (i, e) in self.file_entries.iter().enumerate() {
            if let FileEntry::Entry(v) = e {
                if let Some(id) = v["id"].as_str() {
                    self.by_id.insert(id.to_string(), i);
                    self.leaf_id = Some(id.to_string());
                }
            }
        }
    }

    fn _rewrite_file(&mut self) {
        if !self.persist {
            return;
        }
        let Some(ref p) = self.session_file else {
            return;
        };
        let mut c = String::new();
        for e in &self.file_entries {
            match e {
                FileEntry::Session(h) => {
                    c.push_str(&serde_json::to_string(h).unwrap());
                }
                FileEntry::Entry(v) => {
                    c.push_str(&serde_json::to_string(v).unwrap());
                }
            }
            c.push('\n');
        }
        let _ = fs::write(p, &c);
    }

    fn _append_entry(&mut self, entry: serde_json::Value) {
        self.file_entries.push(FileEntry::Entry(entry.clone()));
        let id = entry["id"].as_str().unwrap_or("").to_string();
        self.by_id.insert(id.clone(), self.file_entries.len() - 1);
        self.leaf_id = Some(id);
        if self.persist {
            if let Some(ref p) = self.session_file {
                let line = serde_json::to_string(&entry).unwrap();
                if let Some(parent) = p.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(p)
                {
                    let _ = writeln!(f, "{}", line);
                }
            }
        }
    }

    pub fn append_message(&mut self, msg: &serde_json::Value) -> String {
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"message","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"message":msg});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }

    pub fn append_thinking_level_change(&mut self, level: &str) -> String {
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"thinking_level_change","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"thinkingLevel":level});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }

    pub fn append_model_change(&mut self, prov: &str, mid: &str) -> String {
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"model_change","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"provider":prov,"modelId":mid});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }

    pub fn append_compaction(&mut self, summary: &str, fkei: &str, tb: u64) -> String {
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"compaction","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"summary":summary,"firstKeptEntryId":fkei,"tokensBefore":tb});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }

    pub fn append_custom_entry(&mut self, ct: &str) -> String {
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"custom","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"customType":ct});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }

    pub fn append_custom_message_entry(
        &mut self,
        ct: &str,
        content: &serde_json::Value,
        display: bool,
    ) -> String {
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"custom_message","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"customType":ct,"content":content,"display":display});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }

    pub fn append_session_info(&mut self, name: &str) -> String {
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"session_info","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"name":name.trim()});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }

    pub fn append_label_change(&mut self, tid: &str, label: Option<&str>) -> String {
        if !self.by_id.contains_key(tid) {
            panic!("Entry {} not found", tid);
        }
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"label","id":id,"parentId":self.leaf_id,"timestamp":now_iso(),"targetId":tid,"label":label});
        let ic = id.clone();
        self._append_entry(e);
        if let Some(l) = label {
            self.labels_by_id.insert(tid.to_string(), l.to_string());
            self.label_timestamps_by_id
                .insert(tid.to_string(), now_iso());
        } else {
            self.labels_by_id.remove(tid);
            self.label_timestamps_by_id.remove(tid);
        }
        ic
    }

    pub fn branch(&mut self, bfi: &str) {
        if !self.by_id.contains_key(bfi) {
            panic!("Entry {} not found", bfi);
        }
        self.leaf_id = Some(bfi.to_string());
    }

    pub fn reset_leaf(&mut self) {
        self.leaf_id = None;
    }

    pub fn branch_with_summary(&mut self, bfi: Option<&str>, summary: &str) -> String {
        if let Some(id) = bfi {
            if !self.by_id.contains_key(id) {
                panic!("Entry {} not found", id);
            }
        }
        self.leaf_id = bfi.map(|s| s.to_string());
        let eids: HashSet<String> = self.by_id.keys().cloned().collect();
        let id = generate_entry_id(&eids);
        let e = serde_json::json!({"type":"branch_summary","id":id,"parentId":bfi,"timestamp":now_iso(),"fromId":bfi.unwrap_or("root"),"summary":summary});
        let ic = id.clone();
        self._append_entry(e);
        ic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn make_in_memory(cwd: &str) -> SessionManager {
        SessionManager::in_memory(Some(cwd))
    }

    // --- Basic append/traversal (port of tree-traversal.test.ts) ---

    #[test]
    fn test_append_message_creates_correct_parent_chain() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"first","timestamp":1}));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"second","timestamp":2}));
        let id3 = sm.append_message(&json!({"role":"user","content":"third","timestamp":3}));
        let entries = sm.get_entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id().to_string(), id1);
        assert_eq!(entries[0].parent_id(), None);
        assert_eq!(entries[1].parent_id(), Some(id1.as_str()));
        assert_eq!(entries[2].parent_id(), Some(id2.as_str()));
    }

    #[test]
    fn test_append_thinking_level_change_integrates_into_tree() {
        let mut sm = make_in_memory("/tmp");
        let msg_id = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        let thinking_id = sm.append_thinking_level_change("high");
        let _msg2_id =
            sm.append_message(&json!({"role":"assistant","content":"response","timestamp":2}));
        let entries = sm.get_entries();
        let te = entries
            .iter()
            .find(|e| matches!(e, SessionEntry::ThinkingLevelChange(_)))
            .unwrap();
        assert_eq!(te.id(), thinking_id);
        assert_eq!(te.parent_id(), Some(msg_id.as_str()));
        assert_eq!(entries[2].parent_id(), Some(thinking_id.as_str()));
    }

    #[test]
    fn test_append_model_change_integrates_into_tree() {
        let mut sm = make_in_memory("/tmp");
        let msg_id = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        let model_id = sm.append_model_change("openai", "gpt-4");
        let _msg2_id =
            sm.append_message(&json!({"role":"assistant","content":"response","timestamp":2}));
        let entries = sm.get_entries();
        let me = entries
            .iter()
            .find(|e| matches!(e, SessionEntry::ModelChange(_)))
            .unwrap();
        assert_eq!(me.id(), model_id);
        assert_eq!(me.parent_id(), Some(msg_id.as_str()));
        assert_eq!(entries[2].parent_id(), Some(model_id.as_str()));
    }

    #[test]
    fn test_append_compaction_integrates() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let cid = sm.append_compaction("summary", &id1, 1000);
        let _id3 = sm.append_message(&json!({"role":"user","content":"3","timestamp":3}));
        let entries = sm.get_entries();
        let ce = entries
            .iter()
            .find(|e| matches!(e, SessionEntry::Compaction(_)))
            .unwrap();
        assert_eq!(ce.id(), cid);
        assert_eq!(ce.parent_id(), Some(id2.as_str()));
    }

    #[test]
    fn test_leaf_pointer_advances() {
        let mut sm = make_in_memory("/tmp");
        assert_eq!(sm.get_leaf_id(), None);
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        assert_eq!(sm.get_leaf_id(), Some(id1.clone()));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        assert_eq!(sm.get_leaf_id(), Some(id2.clone()));
        let id3 = sm.append_thinking_level_change("high");
        assert_eq!(sm.get_leaf_id(), Some(id3));
    }

    // --- get_branch (port of getPath/getBranch in tree-traversal) ---

    #[test]
    fn test_get_branch_empty_session() {
        let sm = make_in_memory("/tmp");
        assert!(sm.get_branch(None).is_empty());
    }

    #[test]
    fn test_get_branch_returns_full_path() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let id3 = sm.append_thinking_level_change("high");
        let id4 = sm.append_message(&json!({"role":"user","content":"3","timestamp":3}));
        let path = sm.get_branch(None);
        assert_eq!(path.len(), 4);
        let ids: Vec<&str> = path.iter().map(|e| e.id()).collect();
        assert_eq!(
            ids,
            vec![id1.as_str(), id2.as_str(), id3.as_str(), id4.as_str()]
        );
    }

    #[test]
    fn test_get_branch_from_specific_entry() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let _id3 = sm.append_message(&json!({"role":"user","content":"3","timestamp":3}));
        let path = sm.get_branch(Some(&id2));
        assert_eq!(path.len(), 2);
        let ids: Vec<&str> = path.iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![id1.as_str(), id2.as_str()]);
    }

    // --- branch and branch_with_summary ---

    #[test]
    fn test_branch_moves_leaf_pointer() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let _id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let id3 = sm.append_message(&json!({"role":"user","content":"3","timestamp":3}));
        assert_eq!(sm.get_leaf_id(), Some(id3.clone()));
        sm.branch(&id1);
        assert_eq!(sm.get_leaf_id(), Some(id1));
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn test_branch_throws_for_nonexistent() {
        let mut sm = make_in_memory("/tmp");
        sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        sm.branch("nonexistent");
    }

    #[test]
    fn test_branch_new_appends_become_children() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let _id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        sm.branch(&id1);
        let id3 = sm.append_message(&json!({"role":"user","content":"branched","timestamp":3}));
        let entries = sm.get_entries();
        let branched = entries.iter().find(|e| e.id() == id3.as_str()).unwrap();
        assert_eq!(branched.parent_id(), Some(id1.as_str()));
    }

    #[test]
    fn test_branch_with_summary_inserts_and_advances() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let _id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let sid = sm.branch_with_summary(Some(&id1), "Summary of work");
        assert_eq!(sm.get_leaf_id(), Some(sid.clone()));
        let entries = sm.get_entries();
        let se = entries
            .iter()
            .find(|e| matches!(e, SessionEntry::BranchSummary(_)))
            .unwrap();
        assert_eq!(se.id(), sid);
        assert_eq!(se.parent_id(), Some(id1.as_str()));
    }

    // --- labels (port of labels.test.ts) ---

    #[test]
    fn test_labels_set_and_get() {
        let mut sm = make_in_memory("/tmp");
        let msg_id = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        assert_eq!(sm.get_label(&msg_id), None);
        sm.append_label_change(&msg_id, Some("checkpoint"));
        assert_eq!(sm.get_label(&msg_id), Some("checkpoint".to_string()));
    }

    #[test]
    fn test_labels_clear_with_none() {
        let mut sm = make_in_memory("/tmp");
        let msg_id = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        sm.append_label_change(&msg_id, Some("checkpoint"));
        assert_eq!(sm.get_label(&msg_id), Some("checkpoint".to_string()));
        sm.append_label_change(&msg_id, None);
        assert_eq!(sm.get_label(&msg_id), None);
    }

    #[test]
    fn test_labels_last_wins() {
        let mut sm = make_in_memory("/tmp");
        let msg_id = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        sm.append_label_change(&msg_id, Some("first"));
        sm.append_label_change(&msg_id, Some("second"));
        sm.append_label_change(&msg_id, Some("third"));
        assert_eq!(sm.get_label(&msg_id), Some("third".to_string()));
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn test_labels_throws_for_nonexistent() {
        let mut sm = make_in_memory("/tmp");
        sm.append_label_change("non-existent", Some("label"));
    }

    #[test]
    fn test_labels_not_in_session_context() {
        let mut sm = make_in_memory("/tmp");
        let msg_id = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        sm.append_label_change(&msg_id, Some("checkpoint"));
        let ctx = sm.build_session_context();
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0]["role"], "user");
    }

    // --- get_leaf_entry ---

    #[test]
    fn test_get_leaf_entry_none_for_empty() {
        let sm = make_in_memory("/tmp");
        assert!(sm.get_leaf_entry().is_none());
    }

    #[test]
    fn test_get_leaf_entry_returns_current() {
        let mut sm = make_in_memory("/tmp");
        sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let leaf = sm.get_leaf_entry().unwrap();
        assert_eq!(leaf.id(), id2);
    }

    // --- get_entry ---

    #[test]
    fn test_get_entry_none_for_missing() {
        let sm = make_in_memory("/tmp");
        assert!(sm.get_entry("nonexistent").is_none());
    }

    #[test]
    fn test_get_entry_returns_by_id() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        let entry = sm.get_entry(&id1).unwrap();
        assert_eq!(entry.id(), id1);
    }

    // --- get_children ---

    #[test]
    fn test_get_children_returns_entries_with_parent() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"root","timestamp":1}));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"resp","timestamp":2}));
        let children = sm.get_children(&id1);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id(), id2);
    }

    // --- build_session_context (port of build-context.test.ts) ---

    #[test]
    fn test_build_context_empty() {
        let ctx = build_session_context(&[], None, None);
        assert!(ctx.messages.is_empty());
        assert_eq!(ctx.thinking_level, "off");
        assert!(ctx.model.is_none());
    }

    #[test]
    fn test_build_context_simple_conversation() {
        let entries = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "2025-01-01T00:00:00Z".into(),
                message: json!({"role":"user","content":"hello","timestamp":1}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "2025-01-01T00:00:01Z".into(),
                message: json!({"role":"assistant","content":"hi","timestamp":2}),
            }),
        ];
        let ctx = build_session_context(&entries, None, None);
        assert_eq!(ctx.messages.len(), 2);
        assert_eq!(ctx.messages[0]["role"], "user");
        assert_eq!(ctx.messages[1]["role"], "assistant");
    }

    #[test]
    fn test_build_context_tracks_thinking_level() {
        let entries = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "2025-01-01T00:00:00Z".into(),
                message: json!({"role":"user","content":"hello","timestamp":1}),
            }),
            SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
                entry_type: "thinking_level_change".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "2025-01-01T00:00:01Z".into(),
                thinking_level: "high".into(),
            }),
        ];
        let ctx = build_session_context(&entries, None, None);
        assert_eq!(ctx.thinking_level, "high");
    }

    #[test]
    fn test_build_context_tracks_model_change() {
        let entries = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "2025-01-01T00:00:00Z".into(),
                message: json!({"role":"user","content":"hello","timestamp":1}),
            }),
            SessionEntry::ModelChange(ModelChangeEntry {
                entry_type: "model_change".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "2025-01-01T00:00:01Z".into(),
                provider: "openai".into(),
                model_id: "gpt-4".into(),
            }),
        ];
        let ctx = build_session_context(&entries, None, None);
        assert_eq!(
            ctx.model.as_ref().map(|m| m.provider.as_str()),
            Some("openai")
        );
    }

    // --- file_operations: parse_session_entries (port of file-operations.test.ts) ---

    #[test]
    fn test_parse_session_entries_valid() {
        let content = r#"{"type":"session","id":"abc","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}
{"type":"message","id":"1","parentId":null,"timestamp":"2025-01-01T00:00:01Z","message":{"role":"user","content":"hi","timestamp":1}}
"#;
        let entries = parse_session_entries(content);
        assert_eq!(entries.len(), 2);
        match &entries[0] {
            FileEntry::Session(h) => assert_eq!(h.id, "abc"),
            _ => panic!("expected session"),
        }
        match &entries[1] {
            FileEntry::Entry(v) => assert_eq!(v["type"], "message"),
            _ => panic!("expected entry"),
        }
    }

    #[test]
    fn test_parse_session_entries_skips_malformed_lines() {
        let content = r#"{"type":"session","id":"abc","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}
not valid json
{"type":"message","id":"1","parentId":null,"timestamp":"2025-01-01T00:00:01Z","message":{"role":"user","content":"hi","timestamp":1}}
"#;
        let entries = parse_session_entries(content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_parse_session_entries_empty_string() {
        let entries = parse_session_entries("");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_session_entries_no_valid_header() {
        let content = r#"{"type":"message","id":"1","parentId":null,"timestamp":"2025-01-01T00:00:01Z","message":{"role":"assistant","content":"test"}}
"#;
        let entries = parse_session_entries(content);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_parse_session_entries_malformed_json() {
        let entries = parse_session_entries("not json\n");
        assert!(entries.is_empty());
    }

    // --- assert_valid_session_id (port of custom-session-id.test.ts) ---

    #[test]
    fn test_valid_session_id() {
        assert_valid_session_id("my-custom-id");
        assert_valid_session_id("abc-123_def.456");
    }

    #[test]
    #[should_panic(expected = "Invalid session id")]
    fn test_invalid_session_id_empty() {
        assert_valid_session_id("");
    }

    #[test]
    #[should_panic(expected = "Invalid session id")]
    fn test_invalid_session_id_starts_with_hyphen() {
        assert_valid_session_id("-abc");
    }

    #[test]
    #[should_panic(expected = "Invalid session id")]
    fn test_invalid_session_id_ends_with_hyphen() {
        assert_valid_session_id("abc-");
    }

    #[test]
    #[should_panic(expected = "Invalid session id")]
    fn test_invalid_session_id_with_slash() {
        assert_valid_session_id("abc/def");
    }

    // --- migration tests (port of migration.test.ts) ---

    #[test]
    fn test_migration_v1_to_v2_adds_ids() {
        let raw = r#"{"type":"session","id":"sess-1","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}
{"type":"message","timestamp":"2025-01-01T00:00:01Z","message":{"role":"user","content":"hi","timestamp":1}}
{"type":"message","timestamp":"2025-01-01T00:00:02Z","message":{"role":"assistant","content":"hello","timestamp":2}}
"#;
        let mut entries = parse_session_entries(raw);
        migrate_session_entries(&mut entries);
        // header should have version set
        match &entries[0] {
            FileEntry::Session(h) => assert_eq!(h.version, Some(3)),
            _ => panic!("expected session"),
        }
        // entries should have id/parentId
        match &entries[1] {
            FileEntry::Entry(v) => {
                assert!(v["id"].as_str().unwrap_or("").len() >= 8);
                assert!(v["parentId"].is_null());
            }
            _ => panic!("expected entry"),
        }
        match &entries[2] {
            FileEntry::Entry(v) => {
                assert!(v["id"].as_str().unwrap_or("").len() >= 8);
                assert!(v["parentId"].as_str().is_some());
            }
            _ => panic!("expected entry"),
        }
    }

    #[test]
    fn test_migration_idempotent() {
        let raw = r#"{"type":"session","id":"sess-1","version":2,"timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}
{"type":"message","id":"abc12345","parentId":null,"timestamp":"2025-01-01T00:00:01Z","message":{"role":"user","content":"hi","timestamp":1}}
{"type":"message","id":"def67890","parentId":"abc12345","timestamp":"2025-01-01T00:00:02Z","message":{"role":"assistant","content":"hello","timestamp":2}}
"#;
        let mut entries = parse_session_entries(raw);
        migrate_session_entries(&mut entries);
        match &entries[1] {
            FileEntry::Entry(v) => assert_eq!(v["id"], "abc12345"),
            _ => panic!("expected entry"),
        }
        match &entries[2] {
            FileEntry::Entry(v) => {
                assert_eq!(v["id"], "def67890");
                assert_eq!(v["parentId"], "abc12345");
            }
            _ => panic!("expected entry"),
        }
    }

    // --- build_session_context with branches ---

    #[test]
    fn test_build_context_follows_specified_leaf() {
        let entries: Vec<SessionEntry> = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "".into(),
                message: json!({"role":"user","content":"start","timestamp":1}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"response","timestamp":2}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "3".into(),
                parent_id: Some("2".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"branch A","timestamp":3}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "4".into(),
                parent_id: Some("2".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"branch B","timestamp":3}),
            }),
        ];
        let ctx_a = build_session_context(&entries, Some("3"), None);
        assert_eq!(ctx_a.messages.len(), 3);
        let ctx_b = build_session_context(&entries, Some("4"), None);
        assert_eq!(ctx_b.messages.len(), 3);
    }

    #[test]
    fn test_build_context_uses_fallback_when_leaf_not_found() {
        let entries: Vec<SessionEntry> = vec![SessionEntry::Message(SessionMessageEntry {
            entry_type: "message".into(),
            id: "1".into(),
            parent_id: None,
            timestamp: "".into(),
            message: json!({"role":"user","content":"hello","timestamp":1}),
        })];
        let ctx = build_session_context(&entries, Some("nonexistent"), None);
        assert_eq!(ctx.messages.len(), 1);
    }

    // --- find_most_recent_session ---

    #[test]
    fn test_find_most_recent_nonexistent_dir() {
        let result = find_most_recent_session(Path::new("/__nonexistent_dir_xyz__/sessions"), None);
        assert!(result.is_none());
    }

    // --- get_default_session_dir_path ---

    #[test]
    fn test_get_default_session_dir_path_contains_cwd() {
        let path = get_default_session_dir_path("/tmp");
        // resolve_cwd canonicalizes /tmp to /private/tmp on macOS
        assert!(path.contains("--tmp--") || path.contains("--private-tmp--"));
    }

    #[test]
    fn test_default_session_dir_uses_explicit_agent_dir() {
        let path = get_default_session_dir_path_for_agent("/tmp/project", Path::new("/tmp/agent"));
        assert!(path.starts_with("/tmp/agent/sessions"));
        assert!(path.ends_with("--tmp-project--"));
    }

    // --- get_session_name ---

    #[test]
    fn test_get_session_name_none() {
        let sm = make_in_memory("/tmp");
        assert_eq!(sm.get_session_name(), None);
    }

    #[test]
    fn test_get_session_name_from_session_info() {
        let mut sm = make_in_memory("/tmp");
        sm.append_session_info("my test session");
        assert_eq!(sm.get_session_name(), Some("my test session".to_string()));
    }

    // --- get_latest_compaction_entry ---

    #[test]
    fn test_get_latest_compaction_entry_none() {
        let entries: Vec<SessionEntry> = vec![];
        assert!(get_latest_compaction_entry(&entries).is_none());
    }

    // --- custom session id ---

    #[test]
    fn test_new_session_with_custom_id() {
        let mut sm = make_in_memory("/tmp");
        sm.new_session(Some(&NewSessionOptions {
            id: Some("my-custom-id".into()),
            parent_session: None,
        }));
        assert_eq!(sm.get_session_id(), "my-custom-id");
    }

    #[test]
    fn test_new_session_with_valid_punctuation() {
        let mut sm = make_in_memory("/tmp");
        sm.new_session(Some(&NewSessionOptions {
            id: Some("abc-123_def.456".into()),
            parent_session: None,
        }));
        assert_eq!(sm.get_session_id(), "abc-123_def.456");
    }

    #[test]
    #[should_panic(expected = "Invalid session id")]
    fn test_new_session_rejects_invalid_id() {
        let mut sm = make_in_memory("/tmp");
        sm.new_session(Some(&NewSessionOptions {
            id: Some("-abc".into()),
            parent_session: None,
        }));
    }

    #[test]
    fn test_header_includes_custom_id() {
        let mut sm = make_in_memory("/tmp");
        sm.new_session(Some(&NewSessionOptions {
            id: Some("header-test-id".into()),
            parent_session: None,
        }));
        let header = sm.get_header().unwrap();
        assert_eq!(header.id, "header-test-id");
    }

    #[test]
    fn test_new_session_generates_id() {
        let mut sm = make_in_memory("/tmp");
        sm.new_session(None);
        assert!(!sm.get_session_id().is_empty());
        let header = sm.get_header().unwrap();
        assert_eq!(header.id, sm.get_session_id());
    }

    #[test]
    fn test_new_session_with_parent() {
        let mut sm = make_in_memory("/tmp");
        sm.new_session(Some(&NewSessionOptions {
            id: None,
            parent_session: Some("parent.jsonl".into()),
        }));
        assert!(!sm.get_session_id().is_empty());
        let header = sm.get_header().unwrap();
        assert_eq!(header.parent_session, Some("parent.jsonl".to_string()));
    }

    // --- session context with compaction ---

    #[test]
    fn test_build_context_with_compaction() {
        let entries: Vec<SessionEntry> = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "".into(),
                message: json!({"role":"user","content":"first","timestamp":1}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"response1","timestamp":2}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "3".into(),
                parent_id: Some("2".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"second","timestamp":3}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "4".into(),
                parent_id: Some("3".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"response2","timestamp":4}),
            }),
            SessionEntry::Compaction(CompactionEntry {
                entry_type: "compaction".into(),
                id: "5".into(),
                parent_id: Some("4".into()),
                timestamp: "".into(),
                summary: "Summary of first two turns".into(),
                first_kept_entry_id: "3".into(),
                tokens_before: 1000,
                details: None,
                from_hook: None,
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "6".into(),
                parent_id: Some("5".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"third","timestamp":5}),
            }),
        ];
        let ctx = build_session_context(&entries, None, None);
        // summary + kept(3,4) + after(6) = 4 messages
        assert_eq!(ctx.messages.len(), 4);
        assert_eq!(ctx.messages[0]["role"], "compactionSummary");
        assert_eq!(ctx.messages[1]["content"], "second");
        assert_eq!(ctx.messages[3]["content"], "third");
    }

    // --- reset_leaf ---

    #[test]
    fn test_reset_leaf_clears_leaf() {
        let mut sm = make_in_memory("/tmp");
        sm.append_message(&json!({"role":"user","content":"hi","timestamp":1}));
        assert!(sm.get_leaf_id().is_some());
        sm.reset_leaf();
        assert_eq!(sm.get_leaf_id(), None);
    }

    // --- is_persisted and get_cwd ---

    #[test]
    fn test_in_memory_is_not_persisted() {
        let sm = make_in_memory("/tmp");
        assert!(!sm.is_persisted());
        let cwd = sm.get_cwd();
        // Canonicalized path: /tmp -> /private/tmp on macOS
        assert!(cwd == "/tmp" || cwd == "/private/tmp");
    }

    // --- get_branch from leaf after branching ---

    #[test]
    fn test_get_branch_from_leaf_after_branching() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let _id3 = sm.append_message(&json!({"role":"user","content":"3","timestamp":3}));
        sm.branch(&id2);
        let id4 = sm.append_message(&json!({"role":"user","content":"4-branch","timestamp":4}));
        let path = sm.get_branch(None);
        let ids: Vec<&str> = path.iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![id1.as_str(), id2.as_str(), id4.as_str()]);
    }

    // --- get_branch includes non-message entries ---

    #[test]
    fn test_get_branch_includes_non_message_entries() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let custom_id = sm.append_custom_entry("test_type");
        let id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let path = sm.get_branch(None);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id(), id1);
        assert_eq!(path[1].id(), custom_id);
        assert_eq!(path[2].id(), id2);
    }

    // --- custom message entries (port of save-entry.test.ts) ---

    #[test]
    fn test_custom_entry_skipped_in_session_context() {
        let mut sm = make_in_memory("/tmp");
        sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        sm.append_custom_entry("my_data");
        sm.append_message(&json!({"role":"assistant","content":"hi","timestamp":2}));
        let ctx = sm.build_session_context();
        assert_eq!(ctx.messages.len(), 2);
    }

    // --- append_session_info ---

    #[test]
    fn test_session_info_stores_name() {
        let mut sm = make_in_memory("/tmp");
        let id = sm.append_session_info("My Session");
        let entry = sm.get_entry(&id).unwrap();
        match &entry {
            SessionEntry::SessionInfo(s) => assert_eq!(s.name.as_deref(), Some("My Session")),
            _ => panic!("expected session_info"),
        }
    }

    // --- load_entries_from_file (port of file-operations.test.ts) ---

    #[test]
    fn test_load_entries_from_file_nonexistent() {
        let entries = load_entries_from_file(Path::new("/__nonexistent_xyz__/file.jsonl"));
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_entries_from_file_empty() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("empty.jsonl");
        std::fs::write(&file, "").unwrap();
        let entries = load_entries_from_file(&file);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_entries_from_file_no_header() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("no-header.jsonl");
        std::fs::write(&file, r#"{"type":"message","id":"1"}"#).unwrap();
        let entries = load_entries_from_file(&file);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_entries_from_file_malformed_json() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("malformed.jsonl");
        std::fs::write(&file, "not json\n").unwrap();
        let entries = load_entries_from_file(&file);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_entries_from_file_valid() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("valid.jsonl");
        std::fs::write(
            &file,
            r#"{"type":"session","id":"abc","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}
{"type":"message","id":"1","parentId":null,"timestamp":"2025-01-01T00:00:01Z","message":{"role":"user","content":"hi","timestamp":1}}
"#,
        )
        .unwrap();
        let entries = load_entries_from_file(&file);
        assert_eq!(entries.len(), 2);
        match &entries[0] {
            FileEntry::Session(h) => assert_eq!(h.id, "abc"),
            _ => panic!("expected session"),
        }
        match &entries[1] {
            FileEntry::Entry(v) => assert_eq!(v["type"], "message"),
            _ => panic!("expected entry"),
        }
    }

    #[test]
    fn test_load_entries_from_file_mixed_lines() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("mixed.jsonl");
        std::fs::write(
            &file,
            r#"{"type":"session","id":"abc","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}
not valid json
{"type":"message","id":"1","parentId":null,"timestamp":"2025-01-01T00:00:01Z","message":{"role":"user","content":"hi","timestamp":1}}
"#,
        )
        .unwrap();
        let entries = load_entries_from_file(&file);
        assert_eq!(entries.len(), 2);
    }

    // --- find_most_recent_session (port of file-operations.test.ts) ---

    #[test]
    fn test_find_most_recent_empty_dir() {
        let dir = TempDir::new().unwrap();
        let result = find_most_recent_session(dir.path(), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_most_recent_ignores_non_jsonl() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file.txt"), "hello").unwrap();
        std::fs::write(dir.path().join("file.json"), "{}").unwrap();
        let result = find_most_recent_session(dir.path(), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_most_recent_ignores_invalid_jsonl() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("invalid.jsonl"), r#"{"type":"message"}"#).unwrap();
        let result = find_most_recent_session(dir.path(), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_most_recent_single_valid() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("session.jsonl");
        std::fs::write(
            &file,
            r#"{"type":"session","id":"abc","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}"#,
        )
        .unwrap();
        let result = find_most_recent_session(dir.path(), None);
        assert_eq!(result, Some(file.to_string_lossy().to_string()));
    }

    #[test]
    fn test_find_most_recent_most_recently_modified() {
        let dir = TempDir::new().unwrap();
        let older = dir.path().join("older.jsonl");
        let newer = dir.path().join("newer.jsonl");
        std::fs::write(
            &older,
            r#"{"type":"session","id":"old","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}"#,
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(
            &newer,
            r#"{"type":"session","id":"new","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}"#,
        )
        .unwrap();
        let result = find_most_recent_session(dir.path(), None);
        assert_eq!(result.unwrap(), newer.to_string_lossy().to_string());
    }

    #[test]
    fn test_find_most_recent_skips_invalid_returns_valid() {
        let dir = TempDir::new().unwrap();
        let invalid = dir.path().join("invalid.jsonl");
        let valid = dir.path().join("valid.jsonl");
        std::fs::write(&invalid, r#"{"type":"not-session"}"#).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(
            &valid,
            r#"{"type":"session","id":"abc","timestamp":"2025-01-01T00:00:00Z","cwd":"/tmp"}"#,
        )
        .unwrap();
        let result = find_most_recent_session(dir.path(), None);
        assert_eq!(result.unwrap(), valid.to_string_lossy().to_string());
    }

    #[test]
    fn test_find_most_recent_filters_by_cwd() {
        let dir = TempDir::new().unwrap();
        let project_a = dir.path().join("project-a");
        let project_b = dir.path().join("project-b");
        std::fs::create_dir_all(&project_a).unwrap();
        std::fs::create_dir_all(&project_b).unwrap();

        let file_a = dir.path().join("a.jsonl");
        let file_b = dir.path().join("b.jsonl");
        std::fs::write(
            &file_a,
            serde_json::to_string(&serde_json::json!({"type":"session","id":"a","timestamp":"2025-01-01T00:00:00Z","cwd":project_a.to_string_lossy()})).unwrap(),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(
            &file_b,
            serde_json::to_string(&serde_json::json!({"type":"session","id":"b","timestamp":"2025-01-01T00:00:00Z","cwd":project_b.to_string_lossy()})).unwrap(),
        )
        .unwrap();

        let result_a = find_most_recent_session(dir.path(), Some(project_a.to_str().unwrap()));
        assert!(result_a.is_some());
        assert!(result_a.unwrap().contains("a.jsonl"));

        let result_b = find_most_recent_session(dir.path(), Some(project_b.to_str().unwrap()));
        assert!(result_b.is_some());
        assert!(result_b.unwrap().contains("b.jsonl"));
    }

    // --- build_session_context additional coverage ---

    #[test]
    fn test_build_context_single_message() {
        let entries = vec![SessionEntry::Message(SessionMessageEntry {
            entry_type: "message".into(),
            id: "1".into(),
            parent_id: None,
            timestamp: "2025-01-01T00:00:00Z".into(),
            message: json!({"role":"user","content":"hello","timestamp":1}),
        })];
        let ctx = build_session_context(&entries, None, None);
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0]["role"], "user");
    }

    #[test]
    fn test_build_context_model_from_assistant_message() {
        let entries = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "".into(),
                message: json!({"role":"user","content":"hello","timestamp":1}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"hi","timestamp":2,"provider":"anthropic","modelId":"claude-test"}),
            }),
        ];
        // build_session_context does not extract model from assistant messages
        let ctx = build_session_context(&entries, None, None);
        assert!(ctx.model.is_none());
    }

    #[test]
    fn test_build_context_model_change_entry_overrides_assistant() {
        let entries = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "".into(),
                message: json!({"role":"user","content":"hello","timestamp":1}),
            }),
            SessionEntry::ModelChange(ModelChangeEntry {
                entry_type: "model_change".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "".into(),
                provider: "openai".into(),
                model_id: "gpt-4".into(),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "3".into(),
                parent_id: Some("2".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"hi","timestamp":2}),
            }),
        ];
        let ctx = build_session_context(&entries, None, None);
        // Model change entry sets model
        assert_eq!(
            ctx.model.as_ref().map(|m| m.provider.as_str()),
            Some("openai")
        );
        assert_eq!(
            ctx.model.as_ref().map(|m| m.model_id.as_str()),
            Some("gpt-4")
        );
    }

    #[test]
    fn test_build_context_multiple_compactions_latest_wins() {
        let entries: Vec<SessionEntry> = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "".into(),
                message: json!({"role":"user","content":"a","timestamp":1}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"b","timestamp":2}),
            }),
            SessionEntry::Compaction(CompactionEntry {
                entry_type: "compaction".into(),
                id: "3".into(),
                parent_id: Some("2".into()),
                timestamp: "".into(),
                summary: "First summary".into(),
                first_kept_entry_id: "1".into(),
                tokens_before: 500,
                details: None,
                from_hook: None,
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "4".into(),
                parent_id: Some("3".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"c","timestamp":3}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "5".into(),
                parent_id: Some("4".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"d","timestamp":4}),
            }),
            SessionEntry::Compaction(CompactionEntry {
                entry_type: "compaction".into(),
                id: "6".into(),
                parent_id: Some("5".into()),
                timestamp: "".into(),
                summary: "Second summary".into(),
                first_kept_entry_id: "4".into(),
                tokens_before: 800,
                details: None,
                from_hook: None,
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "7".into(),
                parent_id: Some("6".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"e","timestamp":5}),
            }),
        ];
        let ctx = build_session_context(&entries, None, None);
        // Should use second compaction (latest): summary + kept(4,5) + after(7) = 4 messages
        assert_eq!(ctx.messages.len(), 4);
        assert_eq!(ctx.messages[0]["role"], "compactionSummary");
        assert!(
            ctx.messages[0]["summary"]
                .as_str()
                .unwrap()
                .contains("Second summary")
        );
        assert_eq!(ctx.messages[1]["content"], "c");
    }

    #[test]
    fn test_build_context_with_branch_summary() {
        let entries: Vec<SessionEntry> = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "".into(),
                message: json!({"role":"user","content":"start","timestamp":1}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"response","timestamp":2}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "3".into(),
                parent_id: Some("2".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"abandoned","timestamp":3}),
            }),
            SessionEntry::BranchSummary(BranchSummaryEntry {
                entry_type: "branch_summary".into(),
                id: "4".into(),
                parent_id: Some("2".into()),
                timestamp: "".into(),
                from_id: "3".into(),
                summary: "Summary of abandoned work".into(),
                details: None,
                from_hook: None,
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "5".into(),
                parent_id: Some("4".into()),
                timestamp: "".into(),
                message: json!({"role":"user","content":"new direction","timestamp":4}),
            }),
        ];
        let ctx = build_session_context(&entries, Some("5"), None);
        // start, response, branch_summary, new direction = 4 messages
        assert_eq!(ctx.messages.len(), 4);
        assert_eq!(ctx.messages[0]["content"], "start");
        assert!(
            ctx.messages[2]["summary"]
                .as_str()
                .unwrap()
                .contains("Summary of abandoned work")
        );
        assert_eq!(ctx.messages[3]["content"], "new direction");
    }

    #[test]
    fn test_build_context_handles_orphaned_entries() {
        let entries: Vec<SessionEntry> = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "".into(),
                message: json!({"role":"user","content":"hello","timestamp":1}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("missing".into()),
                timestamp: "".into(),
                message: json!({"role":"assistant","content":"orphan","timestamp":2}),
            }),
        ];
        let ctx = build_session_context(&entries, Some("2"), None);
        // Only the orphan since parent chain breaks
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0]["role"], "assistant");
    }

    // --- append_compaction field verification ---

    #[test]
    fn test_append_compaction_sets_fields() {
        let mut sm = make_in_memory("/tmp");
        let id1 = sm.append_message(&json!({"role":"user","content":"1","timestamp":1}));
        let _id2 = sm.append_message(&json!({"role":"assistant","content":"2","timestamp":2}));
        let cid = sm.append_compaction("test summary", &id1, 500);
        let raw_entries = sm.get_entries();
        let ce = raw_entries.iter().find(|e| e.id() == cid).unwrap();
        match ce {
            SessionEntry::Compaction(c) => {
                assert_eq!(c.summary, "test summary");
                assert_eq!(c.first_kept_entry_id, id1);
                assert_eq!(c.tokens_before, 500);
            }
            _ => panic!("expected compaction"),
        }
    }

    // --- get_children returns empty for nonexistent ---

    #[test]
    fn test_get_children_returns_empty_for_nonexistent() {
        let sm = make_in_memory("/tmp");
        let children = sm.get_children("nonexistent");
        assert!(children.is_empty());
    }

    // --- fork_from with custom session id (ignored: writes to disk) ---

    #[test]
    #[ignore = "fork_from writes to filesystem; needs integration test setup"]
    fn test_fork_with_custom_id() {
        // Placeholder: fork_from creates real files and is better tested
        // via integration tests. The Rust API accepts NewSessionOptions.
    }

    // --- get_entry returns correct type for each entry kind ---

    #[test]
    fn test_get_entry_type_message() {
        let mut sm = make_in_memory("/tmp");
        let id = sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        let entry = sm.get_entry(&id).unwrap();
        match &entry {
            SessionEntry::Message(m) => {
                assert_eq!(m.message["role"], "user");
                assert_eq!(m.message["content"], "hello");
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn test_get_entry_type_thinking_level() {
        let mut sm = make_in_memory("/tmp");
        let id = sm.append_thinking_level_change("high");
        let entry = sm.get_entry(&id).unwrap();
        match &entry {
            SessionEntry::ThinkingLevelChange(t) => assert_eq!(t.thinking_level, "high"),
            _ => panic!("expected thinking_level_change"),
        }
    }

    #[test]
    fn test_get_entry_type_model_change() {
        let mut sm = make_in_memory("/tmp");
        let id = sm.append_model_change("openai", "gpt-4");
        let entry = sm.get_entry(&id).unwrap();
        match &entry {
            SessionEntry::ModelChange(m) => {
                assert_eq!(m.provider, "openai");
                assert_eq!(m.model_id, "gpt-4");
            }
            _ => panic!("expected model_change"),
        }
    }

    // --- branch_with_summary throws for nonexistent ---

    #[test]
    #[should_panic(expected = "not found")]
    fn test_branch_with_summary_throws_for_nonexistent() {
        let mut sm = make_in_memory("/tmp");
        sm.append_message(&json!({"role":"user","content":"hello","timestamp":1}));
        sm.branch_with_summary(Some("nonexistent"), "summary");
    }

    // --- build_session_context with null/empty leaf_id ---

    #[test]
    fn test_build_context_null_leaf_id_returns_empty() {
        let entries = vec![SessionEntry::Message(SessionMessageEntry {
            entry_type: "message".into(),
            id: "1".into(),
            parent_id: None,
            timestamp: "".into(),
            message: json!({"role":"user","content":"hello","timestamp":1}),
        })];
        let ctx = build_session_context(&entries, Some("null"), None);
        assert!(ctx.messages.is_empty());
        assert_eq!(ctx.thinking_level, "off");
    }

    #[test]
    fn test_build_context_blank_leaf_id_returns_empty() {
        let entries = vec![SessionEntry::Message(SessionMessageEntry {
            entry_type: "message".into(),
            id: "1".into(),
            parent_id: None,
            timestamp: "".into(),
            message: json!({"role":"user","content":"hello","timestamp":1}),
        })];
        let ctx = build_session_context(&entries, Some(""), None);
        assert!(ctx.messages.is_empty());
        assert_eq!(ctx.thinking_level, "off");
    }

    // --- get_session_info filters empty/whitespace name ---

    #[test]
    fn test_get_session_name_filters_whitespace_name() {
        let mut sm = make_in_memory("/tmp");
        sm.append_session_info("   ");
        assert_eq!(sm.get_session_name(), None);
    }

    #[test]
    fn test_get_session_name_filters_empty_name() {
        let mut sm = make_in_memory("/tmp");
        sm.append_session_info("");
        assert_eq!(sm.get_session_name(), None);
    }

    // --- get_default_session_dir_path with special chars ---

    #[test]
    fn test_get_default_session_path_with_special_chars() {
        let path = get_default_session_dir_path("/a/b/c");
        assert!(path.contains("--a-b-c--"));
    }
}
