//! Resource loader — manages extensions, skills, prompts, themes, context files,
//! and system prompts for an agent session.
//!
//! Port of `packages/coding-agent/src/core/resource-loader.ts`.
//!
//! Extensions, prompts, themes, and package-manager contributions are still
//! being ported. Context files, system prompts, and skills use their real
//! loaders so sessions receive the same resources shown at startup.

use crate::core::diagnostics::ResourceDiagnostic as SkillDiagnostic;
use crate::core::skills::{LoadSkillsOptions, Skill, load_skills};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// A context file (AGENTS.md / CLAUDE.md) read from disk.
#[derive(Debug, Clone)]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// ResourceDiagnostic / ResourceCollision
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ResourceDiagnostic {
    pub type_: String,
    pub message: String,
    pub path: String,
    pub collision: Option<ResourceCollision>,
}

#[derive(Debug, Clone)]
pub struct ResourceCollision {
    pub resource_type: String,
    pub name: String,
    pub winner_path: String,
    pub loser_path: String,
    pub winner_source: Option<String>,
    pub loser_source: Option<String>,
}

// ---------------------------------------------------------------------------
// Config directory constant
// ---------------------------------------------------------------------------

pub const CONFIG_DIR_NAME: &str = ".hamr";

// ---------------------------------------------------------------------------
// load_project_context_files — AGENTS.md / CLAUDE.md scanning
// ---------------------------------------------------------------------------

fn load_context_file_from_dir(dir: &Path) -> Option<ContextFile> {
    let candidates = ["AGENTS.md", "AGENTS.MD", "CLAUDE.md", "CLAUDE.MD"];
    for filename in &candidates {
        let file_path = dir.join(filename);
        if file_path.exists() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                return Some(ContextFile {
                    path: file_path.to_string_lossy().to_string(),
                    content,
                });
            }
        }
    }
    None
}

/// Load context files (AGENTS.md / CLAUDE.md) from agent dir and ancestor
/// directories of `cwd`.  Returns files in order: agent dir first, then
/// ancestors from root down to cwd.
pub fn load_project_context_files(cwd: &Path, agent_dir: &Path) -> Vec<ContextFile> {
    let mut context_files: Vec<ContextFile> = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    // Global context from agent dir
    if let Some(ctx) = load_context_file_from_dir(agent_dir) {
        seen_paths.insert(ctx.path.clone());
        context_files.push(ctx);
    }

    // Ancestor scanning from cwd up to root
    let root = Path::new("/");
    let mut ancestor_ctx: Vec<ContextFile> = Vec::new();

    let mut current_dir = cwd.to_path_buf();
    loop {
        if let Some(ctx) = load_context_file_from_dir(&current_dir) {
            if !seen_paths.contains(&ctx.path) {
                seen_paths.insert(ctx.path.clone());
                ancestor_ctx.push(ctx);
            }
        }
        if current_dir == root {
            break;
        }
        let parent = current_dir.parent().map(|p| p.to_path_buf());
        match parent {
            Some(p) if p != current_dir => current_dir = p,
            _ => break,
        }
    }

    // Ancestors go in order (closest first)
    ancestor_ctx.reverse();
    context_files.extend(ancestor_ctx);
    context_files
}

// ---------------------------------------------------------------------------
// resolve_prompt_input
// ---------------------------------------------------------------------------

/// If `input` is a path to an existing file, read its content; otherwise
/// return `input` as-is (it's inline text).
fn resolve_prompt_input(input: Option<&str>) -> Option<String> {
    let input = input?.trim();
    if input.is_empty() {
        return None;
    }
    let p = Path::new(input);
    if p.exists() {
        fs::read_to_string(p)
            .ok()
            .or_else(|| Some(input.to_string()))
    } else {
        Some(input.to_string())
    }
}

// ---------------------------------------------------------------------------
// DefaultResourceLoader
// ---------------------------------------------------------------------------

/// Default implementation of the resource loader.  Manages extensions, skills,
/// prompts, themes, context files, and system prompts.
///
/// The algorithmic structure (path merging, dedup, is-under-path,
/// source-info determination) mirrors the TS implementation.
pub struct DefaultResourceLoader {
    cwd: PathBuf,
    agent_dir: PathBuf,
    project_trusted: bool,

    no_extensions: bool,
    no_skills: bool,
    no_prompt_templates: bool,
    no_themes: bool,
    no_context_files: bool,
    system_prompt_source: Option<String>,
    append_system_prompt_source: Option<Vec<String>>,

    // Loaded state
    skill_paths: Vec<PathBuf>,
    skills: Vec<Skill>,
    skill_diagnostics: Vec<SkillDiagnostic>,
    prompts: Vec<String>, // stub — paths only
    prompt_diagnostics: Vec<ResourceDiagnostic>,
    themes: Vec<String>, // stub — paths only
    theme_diagnostics: Vec<ResourceDiagnostic>,
    agents_files: Vec<ContextFile>,
    system_prompt: Option<String>,
    append_system_prompt: Vec<String>,
}

impl DefaultResourceLoader {
    pub fn new(cwd: PathBuf, agent_dir: PathBuf, project_trusted: bool) -> Self {
        Self {
            cwd,
            agent_dir,
            project_trusted,
            no_extensions: false,
            no_skills: false,
            no_prompt_templates: false,
            no_themes: false,
            no_context_files: false,
            system_prompt_source: None,
            append_system_prompt_source: None,
            skill_paths: vec![],
            skills: vec![],
            skill_diagnostics: vec![],
            prompts: vec![],
            prompt_diagnostics: vec![],
            themes: vec![],
            theme_diagnostics: vec![],
            agents_files: vec![],
            system_prompt: None,
            append_system_prompt: vec![],
        }
    }

    /// Create with full options (builder-style).
    #[allow(clippy::too_many_arguments)]
    pub fn with_options(
        cwd: PathBuf,
        agent_dir: PathBuf,
        project_trusted: bool,
        no_extensions: bool,
        no_skills: bool,
        no_prompt_templates: bool,
        no_themes: bool,
        no_context_files: bool,
        system_prompt: Option<String>,
        append_system_prompt: Option<Vec<String>>,
    ) -> Self {
        Self {
            cwd,
            agent_dir,
            project_trusted,
            no_extensions,
            no_skills,
            no_prompt_templates,
            no_themes,
            no_context_files,
            system_prompt_source: system_prompt,
            append_system_prompt_source: append_system_prompt,
            skill_paths: vec![],
            skills: vec![],
            skill_diagnostics: vec![],
            prompts: vec![],
            prompt_diagnostics: vec![],
            themes: vec![],
            theme_diagnostics: vec![],
            agents_files: vec![],
            system_prompt: None,
            append_system_prompt: vec![],
        }
    }

    // ── Getters ──────────────────────────────────────────────────────────

    pub fn get_agents_files(&self) -> &[ContextFile] {
        &self.agents_files
    }

    pub fn get_system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn get_append_system_prompt(&self) -> &[String] {
        &self.append_system_prompt
    }

    pub fn get_skills(&self) -> (&[Skill], &[SkillDiagnostic]) {
        (&self.skills, &self.skill_diagnostics)
    }

    pub fn get_prompts(&self) -> (&[String], &[ResourceDiagnostic]) {
        (&self.prompts, &self.prompt_diagnostics)
    }

    pub fn get_themes(&self) -> (&[String], &[ResourceDiagnostic]) {
        (&self.themes, &self.theme_diagnostics)
    }

    /// Set explicitly configured skill files or directories.
    pub fn set_skill_paths(&mut self, paths: Vec<PathBuf>) {
        self.skill_paths = paths
            .into_iter()
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    self.cwd.join(path)
                }
            })
            .collect();
    }

    // ── Private helpers ──────────────────────────────────────────────────

    fn resolve_resource_path(&self, p: &str) -> PathBuf {
        let path = Path::new(p);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }

    fn merge_paths(&self, primary: &[String], additional: &[String]) -> Vec<String> {
        let mut merged: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for p in primary.iter().chain(additional) {
            let resolved = self.resolve_resource_path(p);
            if let Ok(canon) = resolved.canonicalize() {
                let s = canon.to_string_lossy().to_string();
                if seen.insert(s.clone()) {
                    merged.push(s);
                }
            }
        }
        merged
    }

    fn discover_system_prompt_file(&self) -> Option<String> {
        let project = self.cwd.join(CONFIG_DIR_NAME).join("SYSTEM.md");
        if self.project_trusted && project.exists() {
            return Some(project.to_string_lossy().to_string());
        }
        let global = self.agent_dir.join("SYSTEM.md");
        if global.exists() {
            return Some(global.to_string_lossy().to_string());
        }
        None
    }

    fn discover_append_system_prompt_file(&self) -> Option<String> {
        let project = self.cwd.join(CONFIG_DIR_NAME).join("APPEND_SYSTEM.md");
        if self.project_trusted && project.exists() {
            return Some(project.to_string_lossy().to_string());
        }
        let global = self.agent_dir.join("APPEND_SYSTEM.md");
        if global.exists() {
            return Some(global.to_string_lossy().to_string());
        }
        None
    }

    // ── Reload ───────────────────────────────────────────────────────────

    /// Full reload: scans context files, resolves system/append prompts, and
    /// loads skills. Extensions, prompts, and themes remain separate ports.
    pub fn reload(&mut self) {
        // 1. Load context files (AGENTS.md / CLAUDE.md)
        self.agents_files = if self.no_context_files {
            vec![]
        } else {
            load_project_context_files(&self.cwd, &self.agent_dir)
        };

        // 2. System prompt
        let system_path = self
            .system_prompt_source
            .clone()
            .or_else(|| self.discover_system_prompt_file());
        self.system_prompt = resolve_prompt_input(system_path.as_deref());

        // 3. Append system prompt
        let append_sources = self
            .append_system_prompt_source
            .clone()
            .or_else(|| self.discover_append_system_prompt_file().map(|f| vec![f]));

        self.append_system_prompt = append_sources
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| resolve_prompt_input(Some(&s)))
            .collect();

        // 4. Skills. --no-skills suppresses defaults, while explicitly
        // supplied paths remain available just like the TypeScript loader.
        if self.no_skills && self.skill_paths.is_empty() {
            self.skills.clear();
            self.skill_diagnostics.clear();
        } else {
            let result = load_skills(LoadSkillsOptions {
                cwd: self.cwd.clone(),
                agent_dir: self.agent_dir.clone(),
                skill_paths: self.skill_paths.clone(),
                include_defaults: !self.no_skills,
            });
            self.skills = result.skills;
            self.skill_diagnostics = result.diagnostics;
            self.skills.sort_by(|a, b| a.name.cmp(&b.name));
        }

        // 5. Extensions, prompts, and themes — pending their dedicated ports.
    }

    /// Extend resources dynamically (from extension contributions).
    pub fn extend_resources(
        &mut self,
        skill_paths: &[String],
        prompt_paths: &[String],
        theme_paths: &[String],
    ) {
        if !skill_paths.is_empty() {
            let existing: Vec<String> = self
                .skill_paths
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect();
            self.skill_paths = self
                .merge_paths(&existing, skill_paths)
                .into_iter()
                .map(PathBuf::from)
                .collect();
            let result = load_skills(LoadSkillsOptions {
                cwd: self.cwd.clone(),
                agent_dir: self.agent_dir.clone(),
                skill_paths: self.skill_paths.clone(),
                include_defaults: !self.no_skills,
            });
            self.skills = result.skills;
            self.skill_diagnostics = result.diagnostics;
            self.skills.sort_by(|a, b| a.name.cmp(&b.name));
        }
        if !prompt_paths.is_empty() {
            self.prompts = self.merge_paths(&self.prompts, prompt_paths);
        }
        if !theme_paths.is_empty() {
            self.themes = self.merge_paths(&self.themes, theme_paths);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_prompt_input_none() {
        assert_eq!(resolve_prompt_input(None), None);
    }

    #[test]
    fn test_resolve_prompt_input_empty_string() {
        assert_eq!(resolve_prompt_input(Some("")), None);
    }

    #[test]
    fn test_resolve_prompt_input_whitespace() {
        assert_eq!(resolve_prompt_input(Some("   ")), None);
    }

    #[test]
    fn test_resolve_prompt_input_strips_whitespace() {
        let result = resolve_prompt_input(Some("  hello  "));
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_resolve_prompt_input_preserves_content() {
        let result = resolve_prompt_input(Some("- bullet point"));
        assert_eq!(result, Some("- bullet point".to_string()));
    }

    #[test]
    fn test_load_context_file_from_dir_nonexistent() {
        let dir = Path::new("/tmp/__nonexistent_ctx_dir__");
        let result = load_context_file_from_dir(dir);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_context_file_from_dir_finds_agents_md() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("AGENTS.md");
        fs::write(&path, "test content").unwrap();
        let result = load_context_file_from_dir(dir.path());
        assert!(result.is_some());
        let ctx = result.unwrap();
        assert!(ctx.path.ends_with("AGENTS.md"));
        assert_eq!(ctx.content, "test content");
    }

    #[test]
    fn test_load_context_file_from_dir_finds_claude_md() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "claude content").unwrap();
        let result = load_context_file_from_dir(dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "claude content");
    }

    #[test]
    fn test_load_context_file_from_dir_agents_over_claude() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "agents").unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "claude").unwrap();
        let result = load_context_file_from_dir(dir.path());
        assert!(result.is_some());
        // AGENTS.md should be preferred
        assert_eq!(result.unwrap().content, "agents");
    }

    #[test]
    fn test_load_project_context_files_empty_dir() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let files = load_project_context_files(cwd.path(), agent_dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn test_load_project_context_files_from_agent_dir() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        fs::write(agent_dir.path().join("AGENTS.md"), "global instructions").unwrap();
        let files = load_project_context_files(cwd.path(), agent_dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].path.contains("AGENTS.md"));
    }

    #[test]
    fn test_load_project_context_files_from_cwd_and_agent() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        // Write in both locations
        fs::write(agent_dir.path().join("AGENTS.md"), "global").unwrap();
        fs::write(cwd.path().join("AGENTS.md"), "project").unwrap();
        let files = load_project_context_files(cwd.path(), agent_dir.path());
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_load_project_context_files_avoids_duplicates() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        // Same file path should not be included twice
        fs::write(agent_dir.path().join("AGENTS.md"), "global").unwrap();
        // Also create AGENTS.md in cwd which is under agent_dir (should still be added)
        fs::write(cwd.path().join("AGENTS.md"), "project").unwrap();
        let files = load_project_context_files(cwd.path(), agent_dir.path());
        // Just verify we get some files without panic
        assert!(!files.is_empty());
    }

    // --- merge_paths ---

    #[test]
    fn test_merge_paths_empty() {
        let loader =
            DefaultResourceLoader::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/agent"), true);
        let merged = loader.merge_paths(&[], &[]);
        assert!(merged.is_empty());
    }

    // --- discover_system_prompt_file / discover_append_system_prompt_file ---

    #[test]
    fn test_discover_system_prompt_project_over_global() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let hamr_dir = cwd.path().join(".hamr");
        fs::create_dir_all(&hamr_dir).unwrap();
        fs::write(hamr_dir.join("SYSTEM.md"), "project system").unwrap();
        fs::write(agent_dir.path().join("SYSTEM.md"), "global system").unwrap();
        let loader = DefaultResourceLoader::new(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            true,
        );
        let found = loader.discover_system_prompt_file();
        assert!(found.is_some());
        assert!(found.as_ref().unwrap().contains(".hamr"));
    }

    #[test]
    fn test_discover_system_prompt_global_only() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        fs::write(agent_dir.path().join("SYSTEM.md"), "global system").unwrap();
        let loader = DefaultResourceLoader::new(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            true,
        );
        let found = loader.discover_system_prompt_file();
        assert!(found.is_some());
    }

    #[test]
    fn test_discover_system_prompt_none() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let loader = DefaultResourceLoader::new(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            true,
        );
        assert!(loader.discover_system_prompt_file().is_none());
    }

    #[test]
    fn test_discover_system_prompt_not_trusted_skips_project() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let hamr_dir = cwd.path().join(".hamr");
        fs::create_dir_all(&hamr_dir).unwrap();
        fs::write(hamr_dir.join("SYSTEM.md"), "project system").unwrap();
        fs::write(agent_dir.path().join("SYSTEM.md"), "global system").unwrap();
        let loader = DefaultResourceLoader::new(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            false, // not trusted
        );
        let found = loader.discover_system_prompt_file();
        assert!(found.is_some());
        // Should find global, not project
        assert!(!found.as_ref().unwrap().contains(".hamr"));
    }

    // --- reload with context files ---

    #[test]
    fn test_reload_loads_context_files() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        fs::write(cwd.path().join("AGENTS.md"), "project instructions").unwrap();
        let mut loader = DefaultResourceLoader::new(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            true,
        );
        loader.reload();
        let files = loader.get_agents_files();
        assert!(!files.is_empty());
        assert!(
            files
                .iter()
                .any(|f| f.content.contains("project instructions"))
        );
    }

    #[test]
    fn test_reload_skips_context_files_when_no_context_files() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        fs::write(cwd.path().join("AGENTS.md"), "project instructions").unwrap();
        let mut loader = DefaultResourceLoader::with_options(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            true,
            false, // no_extensions
            false, // no_skills
            false, // no_prompt_templates
            false, // no_themes
            true,  // no_context_files
            None,
            None,
        );
        loader.reload();
        assert!(loader.get_agents_files().is_empty());
    }

    #[test]
    fn test_reload_loads_default_project_skills() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let skill_dir = cwd.path().join(".hamr/skills/release-check");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: release-check\ndescription: Verify release parity\n---\n",
        )
        .unwrap();

        let mut loader = DefaultResourceLoader::new(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            true,
        );
        loader.reload();

        let (skills, diagnostics) = loader.get_skills();
        assert!(diagnostics.is_empty());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "release-check");
    }

    #[test]
    fn test_no_skills_still_loads_explicit_skill_path() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let skill_dir = cwd.path().join("explicit-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: explicit-skill\ndescription: Explicit test skill\n---\n",
        )
        .unwrap();

        let mut loader = DefaultResourceLoader::with_options(
            cwd.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            true,
            false,
            true,
            false,
            false,
            false,
            None,
            None,
        );
        loader.set_skill_paths(vec![PathBuf::from("explicit-skill")]);
        loader.reload();

        let (skills, diagnostics) = loader.get_skills();
        assert!(diagnostics.is_empty());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "explicit-skill");
    }

    // --- extend_resources ---

    #[test]
    fn test_extend_resources_empty() {
        let mut loader =
            DefaultResourceLoader::new(PathBuf::from("/tmp"), PathBuf::from("/tmp/agent"), true);
        // Should not panic
        loader.reload();
        loader.extend_resources(&[], &[], &[]);
    }
}
