//! Port of `packages/agent/src/harness/skills.ts`.

use crate::harness::types::{ExecutionEnv, FileErrorCode, FileInfo, Skill};
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const MAX_NAME_LENGTH: usize = 64;
const MAX_DESCRIPTION_LENGTH: usize = 1024;
const IGNORE_FILE_NAMES: [&str; 3] = [".gitignore", ".ignore", ".fdignore"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillDiagnosticCode {
    FileInfoFailed,
    ListFailed,
    ReadFailed,
    ParseFailed,
    InvalidMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDiagnostic {
    pub r#type: String,
    pub code: SkillDiagnosticCode,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcedSkill<TSource, TSkill = Skill> {
    pub skill: TSkill,
    pub source: TSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcedSkillDiagnostic<TSource> {
    pub diagnostic: SkillDiagnostic,
    pub source: TSource,
}

#[derive(Debug, Clone)]
struct IgnoreRule {
    pattern: String,
    negated: bool,
}

#[derive(Debug, Default, Clone)]
struct IgnoreMatcher {
    rules: Vec<IgnoreRule>,
}

impl IgnoreMatcher {
    fn add<I>(&mut self, patterns: I)
    where
        I: IntoIterator<Item = String>,
    {
        for pattern in patterns {
            let (negated, pattern) = if let Some(pattern) = pattern.strip_prefix('!') {
                (true, pattern.to_string())
            } else {
                (false, pattern)
            };
            self.rules.push(IgnoreRule { pattern, negated });
        }
    }

    fn ignores(&self, path: &str) -> bool {
        let normalized = path.trim_start_matches('/').trim_end_matches('/');
        let is_dir = path.ends_with('/');
        let mut ignored = false;

        for rule in &self.rules {
            if rule.matches(normalized, is_dir) {
                ignored = !rule.negated;
            }
        }

        ignored
    }
}

impl IgnoreRule {
    fn matches(&self, path: &str, is_dir: bool) -> bool {
        let pattern = self.pattern.trim_end_matches('/');
        let directory_only = self.pattern.ends_with('/');
        if directory_only && !is_dir && !path.starts_with(&format!("{pattern}/")) {
            return false;
        }

        if pattern.contains('/') {
            if let Ok(glob) = glob::Pattern::new(pattern) {
                if glob.matches(path) {
                    return true;
                }
            }
            path == pattern || path.starts_with(&format!("{pattern}/"))
        } else {
            path.split('/').any(|segment| {
                glob::Pattern::new(pattern)
                    .map(|glob| glob.matches(segment))
                    .unwrap_or(false)
            })
        }
    }
}

pub fn format_skill_invocation(skill: &Skill, additional_instructions: Option<&str>) -> String {
    let skill_block = format!(
        "<skill name=\"{}\" location=\"{}\">\nReferences are relative to {}.\n\n{}\n</skill>",
        skill.name,
        skill.file_path,
        dirname_env_path(&skill.file_path),
        skill.content
    );

    if let Some(additional_instructions) = additional_instructions {
        format!("{skill_block}\n\n{additional_instructions}")
    } else {
        skill_block
    }
}

pub async fn load_skills<E: ExecutionEnv>(
    env: &E,
    dirs: &[String],
) -> (Vec<Skill>, Vec<SkillDiagnostic>) {
    let mut skills = Vec::new();
    let mut diagnostics = Vec::new();

    for dir in dirs {
        let root_info = match env.file_info(dir, None).await {
            Ok(info) => info,
            Err(error) => {
                if error.code != FileErrorCode::NotFound {
                    diagnostics.push(SkillDiagnostic {
                        r#type: "warning".to_string(),
                        code: SkillDiagnosticCode::FileInfoFailed,
                        message: error.message,
                        path: dir.clone(),
                    });
                }
                continue;
            }
        };

        if resolve_kind(env, &root_info, &mut diagnostics).await != Some("directory") {
            continue;
        }

        let (mut loaded_skills, mut loaded_diagnostics) = load_skills_from_dir_internal(
            env,
            &root_info.path,
            true,
            IgnoreMatcher::default(),
            &root_info.path,
        )
        .await;
        skills.append(&mut loaded_skills);
        diagnostics.append(&mut loaded_diagnostics);
    }

    (skills, diagnostics)
}

pub async fn load_sourced_skills<E, TSource, TSkill, F>(
    env: &E,
    inputs: &[(String, TSource)],
    map_skill: Option<F>,
) -> (
    Vec<SourcedSkill<TSource, TSkill>>,
    Vec<SourcedSkillDiagnostic<TSource>>,
)
where
    E: ExecutionEnv,
    TSource: Clone,
    TSkill: From<Skill>,
    F: Fn(Skill, &TSource) -> TSkill,
{
    let mut skills = Vec::new();
    let mut diagnostics = Vec::new();

    for (path, source) in inputs {
        let (loaded_skills, loaded_diagnostics) =
            load_skills(env, std::slice::from_ref(path)).await;
        for skill in loaded_skills {
            let skill = if let Some(map_skill) = &map_skill {
                map_skill(skill, source)
            } else {
                skill.into()
            };
            skills.push(SourcedSkill {
                skill,
                source: source.clone(),
            });
        }
        for diagnostic in loaded_diagnostics {
            diagnostics.push(SourcedSkillDiagnostic {
                diagnostic,
                source: source.clone(),
            });
        }
    }

    (skills, diagnostics)
}

fn load_skills_from_dir_internal<'a, E: ExecutionEnv + 'a>(
    env: &'a E,
    dir: &str,
    include_root_files: bool,
    mut ignore_matcher: IgnoreMatcher,
    root_dir: &'a str,
) -> Pin<Box<dyn Future<Output = (Vec<Skill>, Vec<SkillDiagnostic>)> + 'a>> {
    let dir = dir.to_string();
    Box::pin(async move {
        let mut skills = Vec::new();
        let mut diagnostics = Vec::new();

        let dir_info = match env.file_info(&dir, None).await {
            Ok(info) => info,
            Err(error) => {
                if error.code != FileErrorCode::NotFound {
                    diagnostics.push(SkillDiagnostic {
                        r#type: "warning".to_string(),
                        code: SkillDiagnosticCode::FileInfoFailed,
                        message: error.message,
                        path: dir.clone(),
                    });
                }
                return (skills, diagnostics);
            }
        };
        if resolve_kind(env, &dir_info, &mut diagnostics).await != Some("directory") {
            return (skills, diagnostics);
        }

        add_ignore_rules(env, &mut ignore_matcher, &dir, root_dir, &mut diagnostics).await;

        let entries = match env.list_dir(&dir, None).await {
            Ok(entries) => entries,
            Err(error) => {
                diagnostics.push(SkillDiagnostic {
                    r#type: "warning".to_string(),
                    code: SkillDiagnosticCode::ListFailed,
                    message: error.message,
                    path: dir.clone(),
                });
                return (skills, diagnostics);
            }
        };

        for entry in &entries {
            if entry.name != "SKILL.md" {
                continue;
            }

            let kind = resolve_kind(env, entry, &mut diagnostics).await;
            if kind != Some("file") {
                continue;
            }

            let rel_path = relative_env_path(root_dir, &entry.path);
            if ignore_matcher.ignores(&rel_path) {
                continue;
            }

            let (skill, mut file_diagnostics) = load_skill_from_file(env, &entry.path).await;
            if let Some(skill) = skill {
                skills.push(skill);
            }
            diagnostics.append(&mut file_diagnostics);
            return (skills, diagnostics);
        }

        let mut entries = entries;
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        for entry in entries {
            if entry.name.starts_with('.') || entry.name == "node_modules" {
                continue;
            }

            let kind = match resolve_kind(env, &entry, &mut diagnostics).await {
                Some(kind) => kind,
                None => continue,
            };
            let rel_path = relative_env_path(root_dir, &entry.path);
            let ignore_path = if kind == "directory" {
                format!("{rel_path}/")
            } else {
                rel_path.clone()
            };
            if ignore_matcher.ignores(&ignore_path) {
                continue;
            }

            if kind == "directory" {
                let (mut nested_skills, mut nested_diagnostics) = load_skills_from_dir_internal(
                    env,
                    &entry.path,
                    false,
                    ignore_matcher.clone(),
                    root_dir,
                )
                .await;
                skills.append(&mut nested_skills);
                diagnostics.append(&mut nested_diagnostics);
                continue;
            }

            if kind != "file" || !include_root_files || !entry.name.ends_with(".md") {
                continue;
            }

            let (skill, mut file_diagnostics) = load_skill_from_file(env, &entry.path).await;
            if let Some(skill) = skill {
                skills.push(skill);
            }
            diagnostics.append(&mut file_diagnostics);
        }

        (skills, diagnostics)
    })
}

async fn add_ignore_rules<E: ExecutionEnv>(
    env: &E,
    ignore_matcher: &mut IgnoreMatcher,
    dir: &str,
    root_dir: &str,
    diagnostics: &mut Vec<SkillDiagnostic>,
) {
    let relative_dir = relative_env_path(root_dir, dir);
    let prefix = if relative_dir.is_empty() {
        String::new()
    } else {
        format!("{relative_dir}/")
    };

    for file_name in IGNORE_FILE_NAMES {
        let ignore_path = join_env_path(dir, file_name);
        let info = match env.file_info(&ignore_path, None).await {
            Ok(info) => info,
            Err(error) => {
                if error.code != FileErrorCode::NotFound {
                    diagnostics.push(SkillDiagnostic {
                        r#type: "warning".to_string(),
                        code: SkillDiagnosticCode::FileInfoFailed,
                        message: error.message,
                        path: ignore_path.clone(),
                    });
                }
                continue;
            }
        };
        if info.kind != crate::harness::types::FileKind::File {
            continue;
        }

        let content = match env.read_text_file(&ignore_path, None).await {
            Ok(content) => content,
            Err(error) => {
                diagnostics.push(SkillDiagnostic {
                    r#type: "warning".to_string(),
                    code: SkillDiagnosticCode::ReadFailed,
                    message: error.message,
                    path: ignore_path.clone(),
                });
                continue;
            }
        };

        let patterns = content
            .split('\n')
            .filter_map(|line| prefix_ignore_pattern(line, &prefix))
            .collect::<Vec<_>>();
        ignore_matcher.add(patterns);
    }
}

fn prefix_ignore_pattern(line: &str, prefix: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('#') && !trimmed.starts_with("\\#") {
        return None;
    }

    let mut pattern = line.to_string();
    let mut negated = false;
    if let Some(rest) = pattern.strip_prefix('!') {
        negated = true;
        pattern = rest.to_string();
    } else if let Some(rest) = pattern.strip_prefix("\\!") {
        pattern = rest.to_string();
    }

    if let Some(rest) = pattern.strip_prefix('/') {
        pattern = rest.to_string();
    }
    let prefixed = if prefix.is_empty() {
        pattern
    } else {
        format!("{prefix}{pattern}")
    };
    if negated {
        Some(format!("!{prefixed}"))
    } else {
        Some(prefixed)
    }
}

async fn load_skill_from_file<E: ExecutionEnv>(
    env: &E,
    file_path: &str,
) -> (Option<Skill>, Vec<SkillDiagnostic>) {
    let mut diagnostics = Vec::new();
    let raw_content = match env.read_text_file(file_path, None).await {
        Ok(content) => content,
        Err(error) => {
            diagnostics.push(SkillDiagnostic {
                r#type: "warning".to_string(),
                code: SkillDiagnosticCode::ReadFailed,
                message: error.message,
                path: file_path.to_string(),
            });
            return (None, diagnostics);
        }
    };

    let (frontmatter, body) = match parse_frontmatter(&raw_content) {
        Ok(parsed) => parsed,
        Err(error) => {
            diagnostics.push(SkillDiagnostic {
                r#type: "warning".to_string(),
                code: SkillDiagnosticCode::ParseFailed,
                message: error,
                path: file_path.to_string(),
            });
            return (None, diagnostics);
        }
    };

    let skill_dir = dirname_env_path(file_path);
    let parent_dir_name = basename_env_path(&skill_dir);
    let description = frontmatter
        .get("description")
        .and_then(yaml_string)
        .map(|value| value.to_string());

    for error in validate_description(description.as_deref()) {
        diagnostics.push(SkillDiagnostic {
            r#type: "warning".to_string(),
            code: SkillDiagnosticCode::InvalidMetadata,
            message: error,
            path: file_path.to_string(),
        });
    }

    let name = frontmatter
        .get("name")
        .and_then(yaml_string)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| parent_dir_name.clone());
    for error in validate_name(&name, &parent_dir_name) {
        diagnostics.push(SkillDiagnostic {
            r#type: "warning".to_string(),
            code: SkillDiagnosticCode::InvalidMetadata,
            message: error,
            path: file_path.to_string(),
        });
    }

    let Some(description) = description.filter(|description| !description.trim().is_empty()) else {
        return (None, diagnostics);
    };

    (
        Some(Skill {
            name,
            description: description.to_string(),
            content: body,
            file_path: file_path.to_string(),
            disable_model_invocation: matches!(
                frontmatter.get("disable-model-invocation"),
                Some(YamlValue::Bool(true))
            ),
        }),
        diagnostics,
    )
}

fn validate_name(name: &str, parent_dir_name: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if name != parent_dir_name {
        errors.push(format!(
            "name \"{name}\" does not match parent directory \"{parent_dir_name}\""
        ));
    }
    if name.len() > MAX_NAME_LENGTH {
        errors.push(format!(
            "name exceeds {MAX_NAME_LENGTH} characters ({})",
            name.len()
        ));
    }
    if !name.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    }) {
        errors.push(
            "name contains invalid characters (must be lowercase a-z, 0-9, hyphens only)"
                .to_string(),
        );
    }
    if name.starts_with('-') || name.ends_with('-') {
        errors.push("name must not start or end with a hyphen".to_string());
    }
    if name.contains("--") {
        errors.push("name must not contain consecutive hyphens".to_string());
    }
    errors
}

fn validate_description(description: Option<&str>) -> Vec<String> {
    let mut errors = Vec::new();
    match description {
        None => errors.push("description is required".to_string()),
        Some(description) if description.trim().is_empty() => {
            errors.push("description is required".to_string())
        }
        Some(description) if description.len() > MAX_DESCRIPTION_LENGTH => errors.push(format!(
            "description exceeds {MAX_DESCRIPTION_LENGTH} characters ({})",
            description.len()
        )),
        Some(_) => {}
    }
    errors
}

async fn resolve_kind<E: ExecutionEnv>(
    env: &E,
    info: &FileInfo,
    diagnostics: &mut Vec<SkillDiagnostic>,
) -> Option<&'static str> {
    match info.kind {
        crate::harness::types::FileKind::File => return Some("file"),
        crate::harness::types::FileKind::Directory => return Some("directory"),
        crate::harness::types::FileKind::Symlink => {}
    }

    let canonical_path = match env.canonical_path(&info.path, None).await {
        Ok(path) => path,
        Err(error) => {
            if error.code != FileErrorCode::NotFound {
                diagnostics.push(SkillDiagnostic {
                    r#type: "warning".to_string(),
                    code: SkillDiagnosticCode::FileInfoFailed,
                    message: error.message,
                    path: info.path.clone(),
                });
            }
            return None;
        }
    };
    let target = match env.file_info(&canonical_path, None).await {
        Ok(info) => info,
        Err(error) => {
            if error.code != FileErrorCode::NotFound {
                diagnostics.push(SkillDiagnostic {
                    r#type: "warning".to_string(),
                    code: SkillDiagnosticCode::FileInfoFailed,
                    message: error.message,
                    path: info.path.clone(),
                });
            }
            return None;
        }
    };

    match target.kind {
        crate::harness::types::FileKind::File => Some("file"),
        crate::harness::types::FileKind::Directory => Some("directory"),
        crate::harness::types::FileKind::Symlink => None,
    }
}

fn parse_frontmatter(content: &str) -> Result<(HashMap<String, YamlValue>, String), String> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    if !normalized.starts_with("---") {
        return Ok((HashMap::new(), normalized));
    }

    let Some(end_index) = normalized[3..].find("\n---") else {
        return Ok((HashMap::new(), normalized));
    };
    let end_index = end_index + 3;
    let yaml_string = &normalized[4..end_index];
    let body = normalized[end_index + 4..].trim().to_string();

    let yaml = serde_yaml::from_str::<YamlValue>(yaml_string).map_err(|error| error.to_string())?;
    let mut frontmatter = HashMap::new();
    if let YamlValue::Mapping(mapping) = yaml {
        for (key, value) in mapping {
            if let YamlValue::String(key) = key {
                frontmatter.insert(key, value);
            }
        }
    }

    Ok((frontmatter, body))
}

fn yaml_string(value: &YamlValue) -> Option<&str> {
    match value {
        YamlValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn join_env_path(base: &str, child: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        child.trim_start_matches('/')
    )
}

fn dirname_env_path(path: &str) -> String {
    let normalized = path.trim_end_matches('/');
    match normalized.rfind('/') {
        Some(index) if index > 0 => normalized[..index].to_string(),
        _ => "/".to_string(),
    }
}

fn basename_env_path(path: &str) -> String {
    let normalized = path.trim_end_matches('/');
    normalized
        .rsplit('/')
        .next()
        .unwrap_or(normalized)
        .to_string()
}

fn relative_env_path(root: &str, path: &str) -> String {
    let normalized_root = root.trim_end_matches('/');
    let normalized_path = path.trim_end_matches('/');
    if normalized_path == normalized_root {
        return String::new();
    }
    if let Some(rest) = normalized_path.strip_prefix(&format!("{normalized_root}/")) {
        rest.to_string()
    } else {
        normalized_path.trim_start_matches('/').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{format_skill_invocation, load_skills, load_sourced_skills};
    use crate::harness::env::nodejs::NodeExecutionEnv;
    use crate::harness::types::FileSystem;
    use crate::harness::types::Skill;

    #[tokio::test]
    async fn loads_skill_md_files() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.create_dir(".agents/skills/example", true, None)
            .await
            .unwrap();
        env.write_file(
            ".agents/skills/example/SKILL.md",
            b"---\nname: example\ndescription: Example skill\ndisable-model-invocation: true\n---\nUse this skill.\n",
            None,
        )
        .await
        .unwrap();

        let (skills, diagnostics) = load_skills(&env, &[".agents/skills".to_string()]).await;

        assert!(diagnostics.is_empty());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "example");
        assert!(skills[0].disable_model_invocation);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn loads_skills_from_symlinked_directories() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.create_dir("actual/example", true, None).await.unwrap();
        env.write_file(
            "actual/example/SKILL.md",
            b"---\nname: example\ndescription: Example skill\n---\nUse this skill.",
            None,
        )
        .await
        .unwrap();
        symlink(root.path().join("actual"), root.path().join("skills-link")).unwrap();

        let (skills, _) = load_skills(&env, &["skills-link".to_string()]).await;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "example");
        assert!(
            skills[0]
                .file_path
                .ends_with("skills-link/example/SKILL.md")
        );
    }

    #[tokio::test]
    async fn preserves_source_info() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.create_dir("user/example", true, None).await.unwrap();
        env.write_file(
            "user/example/SKILL.md",
            b"---\nname: example\ndescription: Example skill\n---\nUse this skill.",
            None,
        )
        .await
        .unwrap();

        let (skills, diagnostics) = load_sourced_skills::<_, _, Skill, _>(
            &env,
            &[("user".to_string(), "user".to_string())],
            None::<fn(Skill, &String) -> Skill>,
        )
        .await;

        assert!(diagnostics.is_empty());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].source, "user");
        assert_eq!(skills[0].skill.name, "example");
    }

    #[tokio::test]
    async fn reports_invalid_metadata_with_source() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.create_dir("user/broken", true, None).await.unwrap();
        env.write_file(
            "user/broken/SKILL.md",
            b"---\nname: broken\n---\nMissing description.",
            None,
        )
        .await
        .unwrap();

        let (skills, diagnostics) = load_sourced_skills::<_, _, Skill, _>(
            &env,
            &[("user".to_string(), "user".to_string())],
            None::<fn(Skill, &String) -> Skill>,
        )
        .await;

        assert!(skills.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].source, "user");
        assert_eq!(
            diagnostics[0].diagnostic.code,
            super::SkillDiagnosticCode::InvalidMetadata
        );
    }

    #[tokio::test]
    async fn loads_direct_markdown_children_only_from_root() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.create_dir("skills/nested", true, None).await.unwrap();
        env.write_file(
            "skills/root.md",
            b"---\ndescription: Root skill\n---\nRoot content",
            None,
        )
        .await
        .unwrap();
        env.write_file(
            "skills/nested/ignored.md",
            b"---\ndescription: Ignored\n---\nIgnored content",
            None,
        )
        .await
        .unwrap();

        let (skills, _) = load_skills(&env, &["skills".to_string()]).await;
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "skills");
        assert_eq!(skills[0].content, "Root content");
    }

    #[test]
    fn formats_skill_invocation() {
        let skill = Skill {
            name: "inspect".to_string(),
            description: "Inspect things".to_string(),
            content: "Use inspection tools.".to_string(),
            file_path: "/project/.pi/skills/inspect/SKILL.md".to_string(),
            disable_model_invocation: false,
        };

        let output = format_skill_invocation(&skill, Some("Check errors."));
        assert_eq!(
            output,
            "<skill name=\"inspect\" location=\"/project/.pi/skills/inspect/SKILL.md\">\nReferences are relative to /project/.pi/skills/inspect.\n\nUse inspection tools.\n</skill>\n\nCheck errors."
        );
    }
}
