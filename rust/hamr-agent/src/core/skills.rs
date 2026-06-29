//! Skill loading, validation, and prompt formatting.
//!
//! Port of `packages/coding-agent/src/core/skills.ts`.
//!
//! Skills are markdown files with YAML frontmatter discovered from
//! directories on disk. Each skill has a name, description, and optional
//! disable-model-invocation flag.

use crate::core::diagnostics::{
    DiagnosticType, ResourceCollision, ResourceDiagnostic, ResourceType,
};
use crate::core::source_info::{
    SourceInfo, SourceScope, SyntheticSourceInfoOptions, create_synthetic_source_info,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Max name length per spec.
const MAX_NAME_LENGTH: usize = 64;

/// Max description length per spec.
const MAX_DESCRIPTION_LENGTH: usize = 1024;

/// File names treated as ignore rules.
const IGNORE_FILE_NAMES: &[&str] = &[".gitignore", ".ignore", ".fdignore"];

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Parsed YAML frontmatter from a skill markdown file.
#[derive(Debug, Clone, Default)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub disable_model_invocation: Option<bool>,
}

/// A loaded skill.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub file_path: PathBuf,
    pub base_dir: PathBuf,
    pub source_info: SourceInfo,
    pub disable_model_invocation: bool,
}

/// Result of loading skills.
#[derive(Debug, Clone)]
pub struct LoadSkillsResult {
    pub skills: Vec<Skill>,
    pub diagnostics: Vec<ResourceDiagnostic>,
}

/// Options for [`load_skills_from_dir`].
#[derive(Debug, Clone)]
pub struct LoadSkillsFromDirOptions {
    /// Directory to scan for skills.
    pub dir: PathBuf,
    /// Source identifier for these skills.
    pub source: String,
}

/// Options for [`load_skills`].
#[derive(Debug, Clone)]
pub struct LoadSkillsOptions {
    /// Working directory for project-local skills.
    pub cwd: PathBuf,
    /// Agent config directory for global skills.
    pub agent_dir: PathBuf,
    /// Explicit skill paths (files or directories).
    pub skill_paths: Vec<PathBuf>,
    /// Include default skills directories.
    pub include_defaults: bool,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_name(name: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if name.len() > MAX_NAME_LENGTH {
        errors.push(format!(
            "name exceeds {MAX_NAME_LENGTH} characters ({})",
            name.len()
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
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
        None | Some("") => {
            errors.push("description is required".to_string());
        }
        Some(d) if d.trim().is_empty() => {
            errors.push("description is required".to_string());
        }
        Some(d) if d.len() > MAX_DESCRIPTION_LENGTH => {
            errors.push(format!(
                "description exceeds {MAX_DESCRIPTION_LENGTH} characters ({})",
                d.len()
            ));
        }
        _ => {}
    }

    errors
}

// ---------------------------------------------------------------------------
// Source info helper
// ---------------------------------------------------------------------------

fn make_diagnostic(
    diagnostic_type: DiagnosticType,
    message: String,
    path: &Path,
) -> ResourceDiagnostic {
    ResourceDiagnostic {
        diagnostic_type,
        message,
        path: Some(path.to_string_lossy().to_string()),
        collision: None,
    }
}

fn make_collision_diagnostic(
    name: &str,
    winner_path: &Path,
    loser_path: &Path,
) -> ResourceDiagnostic {
    ResourceDiagnostic {
        diagnostic_type: DiagnosticType::Collision,
        message: format!("name \"{name}\" collision"),
        path: Some(loser_path.to_string_lossy().to_string()),
        collision: Some(ResourceCollision {
            resource_type: ResourceType::Skill,
            name: name.to_string(),
            winner_path: winner_path.to_string_lossy().to_string(),
            loser_path: loser_path.to_string_lossy().to_string(),
            winner_source: None,
            loser_source: None,
        }),
    }
}

fn create_skill_source_info(file_path: &Path, base_dir: &Path, source: &str) -> SourceInfo {
    let options = match source {
        "user" => SyntheticSourceInfoOptions {
            source: "local".to_string(),
            scope: Some(SourceScope::User),
            origin: None,
            base_dir: Some(base_dir.to_string_lossy().to_string()),
        },
        "project" => SyntheticSourceInfoOptions {
            source: "local".to_string(),
            scope: Some(SourceScope::Project),
            origin: None,
            base_dir: Some(base_dir.to_string_lossy().to_string()),
        },
        "path" => SyntheticSourceInfoOptions {
            source: "local".to_string(),
            scope: None,
            origin: None,
            base_dir: Some(base_dir.to_string_lossy().to_string()),
        },
        _ => SyntheticSourceInfoOptions {
            source: source.to_string(),
            scope: None,
            origin: None,
            base_dir: Some(base_dir.to_string_lossy().to_string()),
        },
    };
    create_synthetic_source_info(&file_path.to_string_lossy(), options)
}

// ---------------------------------------------------------------------------
// Frontmatter parsing
// ---------------------------------------------------------------------------

fn parse_frontmatter(content: &str) -> Option<SkillFrontmatter> {
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }

    let after_first = &content[3..];
    let end_idx = after_first.find("---")?;
    let yaml_str = &after_first[..end_idx];

    let value: serde_yaml::Value = serde_yaml::from_str(yaml_str).ok()?;
    let mapping = value.as_mapping()?;

    let name = mapping
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let description = mapping
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // YAML allows either `disable-model-invocation` or `disableModelInvocation` (serde_yaml handles both)
    let disable_model_invocation = mapping
        .get("disable-model-invocation")
        .or_else(|| mapping.get("disableModelInvocation"))
        .and_then(|v| v.as_bool());

    Some(SkillFrontmatter {
        name,
        description,
        disable_model_invocation,
    })
}

// ---------------------------------------------------------------------------
// Skill loading from directory
// ---------------------------------------------------------------------------

pub fn load_skills_from_dir(options: LoadSkillsFromDirOptions) -> LoadSkillsResult {
    let LoadSkillsFromDirOptions { dir, source } = options;
    load_skills_from_dir_internal(&dir, &source, true)
}

fn load_skills_from_dir_internal(
    dir: &Path,
    source: &str,
    include_root_files: bool,
) -> LoadSkillsResult {
    let mut skills = Vec::new();
    let mut diagnostics = Vec::new();

    if !dir.exists() {
        return LoadSkillsResult {
            skills,
            diagnostics,
        };
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            return LoadSkillsResult {
                skills,
                diagnostics,
            };
        }
    };

    // First pass: look for SKILL.md in this directory
    for entry in entries.flatten() {
        if entry.file_name() == "SKILL.md" {
            let file_path = entry.path();
            if file_path.is_file() {
                let result = load_skill_from_file(&file_path, source);
                if let Some(skill) = result.skill {
                    skills.push(skill);
                }
                diagnostics.extend(result.diagnostics);
                return LoadSkillsResult {
                    skills,
                    diagnostics,
                };
            }
        }
    }

    // Second pass: collect files and recurse into dirs
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            return LoadSkillsResult {
                skills,
                diagnostics,
            };
        }
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') {
            continue;
        }
        if name_str == "node_modules" {
            continue;
        }

        let file_path = entry.path();
        let file_type = entry.file_type().ok();

        if file_type.as_ref().map_or(false, |ft| ft.is_dir()) {
            let sub_result = load_skills_from_dir_internal(&file_path, source, false);
            skills.extend(sub_result.skills);
            diagnostics.extend(sub_result.diagnostics);
            continue;
        }

        if !include_root_files {
            continue;
        }

        if !name_str.ends_with(".md") {
            continue;
        }

        let result = load_skill_from_file(&file_path, source);
        if let Some(skill) = result.skill {
            skills.push(skill);
        }
        diagnostics.extend(result.diagnostics);
    }

    LoadSkillsResult {
        skills,
        diagnostics,
    }
}

struct SkillFileResult {
    skill: Option<Skill>,
    diagnostics: Vec<ResourceDiagnostic>,
}

fn load_skill_from_file(file_path: &Path, source: &str) -> SkillFileResult {
    let mut diagnostics = Vec::new();

    let raw_content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            diagnostics.push(make_diagnostic(
                DiagnosticType::Warning,
                format!("failed to read skill file: {e}"),
                file_path,
            ));
            return SkillFileResult {
                skill: None,
                diagnostics,
            };
        }
    };

    let frontmatter = parse_frontmatter(&raw_content).unwrap_or_default();
    let skill_dir = file_path.parent().unwrap_or(Path::new("."));
    let parent_dir_name = skill_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Validate description
    let desc_errors = validate_description(frontmatter.description.as_deref());
    for error in &desc_errors {
        diagnostics.push(make_diagnostic(
            DiagnosticType::Warning,
            error.clone(),
            file_path,
        ));
    }

    let name = frontmatter.name.unwrap_or(parent_dir_name);

    let name_errors = validate_name(&name);
    for error in &name_errors {
        diagnostics.push(make_diagnostic(
            DiagnosticType::Warning,
            error.clone(),
            file_path,
        ));
    }

    let description = match frontmatter.description {
        Some(ref d) if !d.trim().is_empty() => d.clone(),
        _ => {
            return SkillFileResult {
                skill: None,
                diagnostics,
            };
        }
    };

    SkillFileResult {
        skill: Some(Skill {
            name,
            description,
            file_path: file_path.to_path_buf(),
            base_dir: skill_dir.to_path_buf(),
            source_info: create_skill_source_info(file_path, skill_dir, source),
            disable_model_invocation: frontmatter.disable_model_invocation.unwrap_or(false),
        }),
        diagnostics,
    }
}

// ---------------------------------------------------------------------------
// Prompt formatting
// ---------------------------------------------------------------------------

pub fn format_skills_for_prompt(skills: &[Skill]) -> String {
    let visible_skills: Vec<&Skill> = skills
        .iter()
        .filter(|s| !s.disable_model_invocation)
        .collect();

    if visible_skills.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        String::new(),
        String::new(),
        "Use skills only when the user's task clearly requires one. Do not load skills for greetings or general chat.".to_string(),
        "When needed, read the matching skill's SKILL.md first. Resolve relative paths from that skill's directory.".to_string(),
        String::new(),
        "<available_skills>".to_string(),
    ];

    for skill in &visible_skills {
        lines.push("  <skill>".to_string());
        lines.push(format!("    <name>{}</name>", escape_xml(&skill.name)));
        lines.push(format!(
            "    <description>{}</description>",
            escape_xml(&skill.description)
        ));
        lines.push(format!(
            "    <location>{}</location>",
            escape_xml(&skill.file_path.to_string_lossy())
        ));
        lines.push("  </skill>".to_string());
    }

    lines.push("</available_skills>".to_string());
    lines.join("\n")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ---------------------------------------------------------------------------
// Top-level loader
// ---------------------------------------------------------------------------

pub fn load_skills(options: LoadSkillsOptions) -> LoadSkillsResult {
    let LoadSkillsOptions {
        cwd,
        agent_dir,
        skill_paths,
        include_defaults,
    } = options;

    let mut skill_map: HashMap<String, Skill> = HashMap::new();
    let mut real_path_set: HashSet<PathBuf> = HashSet::new();
    let mut all_diagnostics: Vec<ResourceDiagnostic> = Vec::new();
    let mut collision_diagnostics: Vec<ResourceDiagnostic> = Vec::new();

    fn add_skills(
        skill_map: &mut HashMap<String, Skill>,
        real_path_set: &mut HashSet<PathBuf>,
        all_diagnostics: &mut Vec<ResourceDiagnostic>,
        collision_diagnostics: &mut Vec<ResourceDiagnostic>,
        result: LoadSkillsResult,
    ) {
        all_diagnostics.extend(result.diagnostics);
        for skill in result.skills {
            let real_path =
                std::fs::canonicalize(&skill.file_path).unwrap_or_else(|_| skill.file_path.clone());

            if real_path_set.contains(&real_path) {
                continue;
            }

            if let Some(existing) = skill_map.get(&skill.name) {
                collision_diagnostics.push(make_collision_diagnostic(
                    &skill.name,
                    &existing.file_path,
                    &skill.file_path,
                ));
            } else {
                skill_map.insert(skill.name.clone(), skill);
                real_path_set.insert(real_path);
            }
        }
    }

    if include_defaults {
        add_skills(
            &mut skill_map,
            &mut real_path_set,
            &mut all_diagnostics,
            &mut collision_diagnostics,
            load_skills_from_dir_internal(&agent_dir.join("skills"), "user", true),
        );
        add_skills(
            &mut skill_map,
            &mut real_path_set,
            &mut all_diagnostics,
            &mut collision_diagnostics,
            load_skills_from_dir_internal(&cwd.join(".hamr").join("skills"), "project", true),
        );
    }

    let user_skills_dir = agent_dir.join("skills");
    let project_skills_dir = cwd.join(".hamr").join("skills");

    fn is_under_path(target: &Path, root: &Path) -> bool {
        if target == root {
            return true;
        }
        target.starts_with(root)
    }

    let get_source = |resolved_path: &Path| -> &str {
        if is_under_path(resolved_path, &user_skills_dir) {
            "user"
        } else if is_under_path(resolved_path, &project_skills_dir) {
            "project"
        } else {
            "temporary"
        }
    };

    for raw_path in skill_paths {
        if !raw_path.exists() {
            all_diagnostics.push(make_diagnostic(
                DiagnosticType::Warning,
                "skill path does not exist".to_string(),
                &raw_path,
            ));
            continue;
        }

        let source = get_source(&raw_path);

        if raw_path.is_dir() {
            add_skills(
                &mut skill_map,
                &mut real_path_set,
                &mut all_diagnostics,
                &mut collision_diagnostics,
                load_skills_from_dir_internal(&raw_path, source, true),
            );
        } else if raw_path.extension().map_or(false, |e| e == "md") {
            let result = load_skill_from_file(&raw_path, source);
            if let Some(skill) = result.skill {
                add_skills(
                    &mut skill_map,
                    &mut real_path_set,
                    &mut all_diagnostics,
                    &mut collision_diagnostics,
                    LoadSkillsResult {
                        skills: vec![skill],
                        diagnostics: result.diagnostics,
                    },
                );
            } else {
                all_diagnostics.extend(result.diagnostics);
            }
        } else {
            all_diagnostics.push(make_diagnostic(
                DiagnosticType::Warning,
                "skill path is not a markdown file".to_string(),
                &raw_path,
            ));
        }
    }

    all_diagnostics.extend(collision_diagnostics);

    LoadSkillsResult {
        skills: skill_map.into_values().collect(),
        diagnostics: all_diagnostics,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("my-skill").is_empty());
        assert!(validate_name("test123").is_empty());
        assert!(validate_name("a").is_empty());
    }

    #[test]
    fn test_validate_name_invalid_chars() {
        let errors = validate_name("My_Skill");
        assert!(errors.iter().any(|e| e.contains("invalid characters")));
    }

    #[test]
    fn test_validate_name_starts_with_hyphen() {
        let errors = validate_name("-bad");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("start or end with a hyphen"))
        );
    }

    #[test]
    fn test_validate_name_consecutive_hyphens() {
        let errors = validate_name("bad--skill");
        assert!(errors.iter().any(|e| e.contains("consecutive hyphens")));
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(65);
        let errors = validate_name(&long_name);
        assert!(errors.iter().any(|e| e.contains("exceeds 64")));
    }

    #[test]
    fn test_validate_description_required() {
        let errors = validate_description(None);
        assert!(errors.iter().any(|e| e.contains("required")));
    }

    #[test]
    fn test_validate_description_empty() {
        let errors = validate_description(Some(""));
        assert!(errors.iter().any(|e| e.contains("required")));
    }

    #[test]
    fn test_validate_description_valid() {
        let errors = validate_description(Some("A valid description"));
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_description_too_long() {
        let long_desc = "a".repeat(1025);
        let errors = validate_description(Some(&long_desc));
        assert!(errors.iter().any(|e| e.contains("exceeds 1024")));
    }

    #[test]
    fn test_format_skills_empty() {
        let result = format_skills_for_prompt(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_skills_xml() {
        let skill = Skill {
            name: "test-skill".to_string(),
            description: "A test skill.".to_string(),
            file_path: PathBuf::from("/path/to/skill/SKILL.md"),
            base_dir: PathBuf::from("/path/to/skill"),
            source_info: create_synthetic_source_info(
                "/path/to/skill/SKILL.md",
                SyntheticSourceInfoOptions {
                    source: "test".to_string(),
                    scope: None,
                    origin: None,
                    base_dir: None,
                },
            ),
            disable_model_invocation: false,
        };

        let result = format_skills_for_prompt(&[skill]);
        assert!(result.contains("<available_skills>"));
        assert!(result.contains("</available_skills>"));
        assert!(result.contains("<skill>"));
        assert!(result.contains("<name>test-skill</name>"));
    }

    #[test]
    fn test_format_skills_escapes_xml() {
        let skill = Skill {
            name: "test-skill".to_string(),
            description: "A \"special\" <skill> & more.".to_string(),
            file_path: PathBuf::from("/path/SKILL.md"),
            base_dir: PathBuf::from("/path"),
            source_info: create_synthetic_source_info(
                "/path/SKILL.md",
                SyntheticSourceInfoOptions {
                    source: "test".to_string(),
                    scope: None,
                    origin: None,
                    base_dir: None,
                },
            ),
            disable_model_invocation: false,
        };

        let result = format_skills_for_prompt(&[skill]);
        assert!(result.contains("&quot;special&quot;"));
        assert!(result.contains("&lt;skill&gt;"));
        assert!(result.contains("&amp;"));
    }

    #[test]
    fn test_format_skills_excludes_disabled() {
        let create = |name: &str, disabled: bool| -> Skill {
            Skill {
                name: name.to_string(),
                description: "desc".to_string(),
                file_path: PathBuf::from(format!("/{name}/SKILL.md")),
                base_dir: PathBuf::from(format!("/{name}")),
                source_info: create_synthetic_source_info(
                    &format!("/{name}/SKILL.md"),
                    SyntheticSourceInfoOptions {
                        source: "test".to_string(),
                        scope: None,
                        origin: None,
                        base_dir: None,
                    },
                ),
                disable_model_invocation: disabled,
            }
        };

        let skills = vec![create("visible", false), create("hidden", true)];
        let result = format_skills_for_prompt(&skills);
        assert!(result.contains("visible"));
        assert!(!result.contains("hidden"));
    }

    #[test]
    fn test_parse_frontmatter_basic() {
        let content = "---\nname: test\ndescription: A test\n---\n# Content";
        let fm = parse_frontmatter(content).unwrap();
        assert_eq!(fm.name.as_deref(), Some("test"));
        assert_eq!(fm.description.as_deref(), Some("A test"));
    }

    #[test]
    fn test_parse_frontmatter_no_content() {
        let content = "no frontmatter here";
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_frontmatter_disable_model() {
        let content =
            "---\nname: test\ndescription: A test\ndisable-model-invocation: true\n---\n# Content";
        let fm = parse_frontmatter(content).unwrap();
        assert_eq!(fm.disable_model_invocation, Some(true));
    }
}
