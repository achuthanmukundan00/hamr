//! Port of `packages/coding-agent/src/core/system_prompt.ts`.
//!
//! System prompt construction and project context loading.

/// A skill entry — mirrors `Skill` in `packages/coding-agent/src/core/skills.ts`.
/// Only the fields needed by `format_skills_for_prompt` are defined here.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub file_path: String,
    pub disable_model_invocation: bool,
}

/// Options for building the system prompt.
#[derive(Debug, Clone, Default)]
pub struct BuildSystemPromptOptions {
    /// Custom system prompt (replaces default).
    pub custom_prompt: Option<String>,
    /// Tools to include in prompt. Default: ["read", "bash", "edit", "write"]
    pub selected_tools: Option<Vec<String>>,
    /// Optional one-line tool snippets keyed by tool name.
    pub tool_snippets: Option<std::collections::HashMap<String, String>>,
    /// Additional guideline bullets appended to the default system prompt guidelines.
    pub prompt_guidelines: Option<Vec<String>>,
    /// Text to append to system prompt.
    pub append_system_prompt: Option<String>,
    /// Working directory.
    pub cwd: String,
    /// Pre-loaded context files.
    pub context_files: Option<Vec<ContextFile>>,
    /// Pre-loaded skills.
    pub skills: Option<Vec<Skill>>,
}

/// A context file entry with path and content.
#[derive(Debug, Clone)]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Path helpers (mirrors config.ts getReadmePath / getDocsPath / getExamplesPath)
// ---------------------------------------------------------------------------

fn get_readme_path() -> String {
    std::env::var("HAMR_README_PATH")
        .or_else(|_| std::env::var("PI_README_PATH"))
        .unwrap_or_else(|_| "/opt/homebrew/lib/node_modules/@skaft/hamr/README.md".to_string())
}

fn get_docs_path() -> String {
    std::env::var("HAMR_DOCS_PATH")
        .or_else(|_| std::env::var("PI_DOCS_PATH"))
        .unwrap_or_else(|_| "/opt/homebrew/lib/node_modules/@skaft/hamr/docs".to_string())
}

fn get_examples_path() -> String {
    std::env::var("HAMR_EXAMPLES_PATH")
        .or_else(|_| std::env::var("PI_EXAMPLES_PATH"))
        .unwrap_or_else(|_| "/opt/homebrew/lib/node_modules/@skaft/hamr/examples".to_string())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Format skills for inclusion in a system prompt.
/// Uses XML format per Agent Skills standard.
/// Skills with disable_model_invocation=true are excluded.
pub fn format_skills_for_prompt(skills: &[Skill]) -> String {
    let visible: Vec<&Skill> = skills
        .iter()
        .filter(|s| !s.disable_model_invocation)
        .collect();

    if visible.is_empty() {
        return String::new();
    }

    let mut lines: Vec<String> = vec![
        String::new(),
        String::new(),
        "Use skills only when the user's task clearly requires one. Do not load skills for greetings or general chat.".to_string(),
        "When needed, read the matching skill's SKILL.md first. Resolve relative paths from that skill's directory.".to_string(),
        String::new(),
        "<available_skills>".to_string(),
    ];

    for skill in &visible {
        lines.push("  <skill>".to_string());
        lines.push(format!("    <name>{}</name>", escape_xml(&skill.name)));
        lines.push(format!(
            "    <description>{}</description>",
            escape_xml(&skill.description)
        ));
        lines.push(format!(
            "    <location>{}</location>",
            escape_xml(&skill.file_path)
        ));
        lines.push("  </skill>".to_string());
    }

    lines.push("</available_skills>".to_string());

    lines.join("\n")
}

/// Build the system prompt with tools, guidelines, and context.
pub fn build_system_prompt(options: &BuildSystemPromptOptions) -> String {
    let resolved_cwd = &options.cwd;
    let prompt_cwd = resolved_cwd.replace('\\', "/");

    let now = chrono::Local::now();
    let date = now.format("%Y-%m-%d").to_string();

    let append_section = options
        .append_system_prompt
        .as_ref()
        .map(|a| format!("\n\n{}", a))
        .unwrap_or_default();

    let context_files = options.context_files.as_deref().unwrap_or(&[]);
    let skills = options.skills.as_deref().unwrap_or(&[]);

    if let Some(ref custom_prompt) = options.custom_prompt {
        let mut prompt = custom_prompt.clone();

        if !append_section.is_empty() {
            prompt.push_str(&append_section);
        }

        // Append project context files
        if !context_files.is_empty() {
            prompt.push_str("\n\n<project_context>\n\n");
            prompt.push_str("Project-specific instructions and guidelines:\n\n");
            for cf in context_files {
                prompt.push_str(&format!(
                    "<project_instructions path=\"{}\">\n{}\n</project_instructions>\n\n",
                    cf.path, cf.content
                ));
            }
            prompt.push_str("</project_context>\n");
        }

        // Append skills index
        if !skills.is_empty() {
            prompt.push_str(&format_skills_for_prompt(skills));
        }

        // Add date and working directory last
        prompt.push_str(&format!("\nCurrent date: {}", date));
        prompt.push_str(&format!("\nCurrent working directory: {}", prompt_cwd));

        return prompt;
    }

    // Default prompt path
    let readme_path = get_readme_path();
    let docs_path = get_docs_path();
    let examples_path = get_examples_path();

    let tools = options
        .selected_tools
        .as_ref()
        .map(|t| t.clone())
        .unwrap_or_else(|| vec!["read".into(), "bash".into(), "edit".into(), "write".into()]);

    let tool_snippets = options.tool_snippets.as_ref();

    let visible_tools: Vec<&String> = tools
        .iter()
        .filter(|name| {
            tool_snippets
                .map(|ts| ts.contains_key(name.as_str()))
                .unwrap_or(false)
        })
        .collect();

    let tools_list = if visible_tools.is_empty() {
        "(none)".to_string()
    } else {
        visible_tools
            .iter()
            .map(|name| {
                let snippet = tool_snippets
                    .and_then(|ts| ts.get(name.as_str()))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                format!("- {}: {}", name, snippet)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Build guidelines based on which tools are actually available
    let mut guidelines_list: Vec<String> = Vec::new();
    let mut guidelines_set: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut add_guideline = |guideline: &str| {
        if guidelines_set.insert(guideline.to_string()) {
            guidelines_list.push(guideline.to_string());
        }
    };

    let has_bash = tools.iter().any(|t| t == "bash");
    let has_grep = tools.iter().any(|t| t == "grep");
    let has_find = tools.iter().any(|t| t == "find");
    let has_ls = tools.iter().any(|t| t == "ls");
    let has_write = tools.iter().any(|t| t == "write");
    let has_edit = tools.iter().any(|t| t == "edit");
    let has_mutation = has_write || has_edit || has_bash;

    // File exploration guidelines
    if has_bash && !has_grep && !has_find && !has_ls {
        add_guideline("Use bash for file operations like ls, rg, find");
    }

    if let Some(ref prompt_guidelines) = options.prompt_guidelines {
        for guideline in prompt_guidelines {
            let normalized = guideline.trim();
            if !normalized.is_empty() {
                add_guideline(normalized);
            }
        }
    }

    // Always include these
    add_guideline("Show file paths clearly.");
    add_guideline("Skill or template fits task? Discover and load it via `ls`/`read`.");
    add_guideline(
        "Use bundled skills and extensions first, only make your own if they don't exist",
    );

    // Tool-call discipline
    add_guideline("Emit only the tool call when calling a tool. No prose alongside it.");

    // Syntax gold
    if has_mutation {
        add_guideline("Finish the task, then stop and summarize.");
        add_guideline("Make real changes with write/edit. Don't just describe them.");
        if has_edit {
            add_guideline("Read a file before you edit it. edit must match exactly.");
        }
    }
    add_guideline(
        "[IMPORTANT]: Think smart, do brilliant things. Speak short. Be honest. Less syllables better than more. But always tell all relevant information.",
    );

    let guidelines = guidelines_list
        .iter()
        .map(|g| format!("- {}", g))
        .collect::<Vec<_>>()
        .join("\n");

    let mut prompt = format!(
        "You are inside the hamr harness. Help user, stay present to user requests, use *tools* to fulfill them.\n\
\n\
Available tools:\n\
{}\n\
\n\
Projects may add custom tools too.\n\
\n\
Rules:\n\
{}\n\
\n\
hamr docs (*read* only when asked about hamr — SDK, extensions, themes, skills, TUI):\n\
- Main docs: {}\n\
- More docs: {}\n\
- Examples: {} (extensions, custom tools, SDK)\n\
- Resolve docs/... under More docs and examples/... under Examples, not the cwd\n\
- Topics: extensions (docs/extensions.md, examples/extensions/), themes (docs/themes.md), skills (docs/skills.md), prompt templates (docs/prompt-templates.md), TUI (docs/tui.md), keybindings (docs/keybindings.md), SDK (docs/sdk.md), custom providers (docs/custom-provider.md), models (docs/models.md), packages (docs/packages.md)\n\
- *read* full hamr .md files; follow .md cross-references before coding",
        tools_list, guidelines, readme_path, docs_path, examples_path
    );

    if !append_section.is_empty() {
        prompt.push_str(&append_section);
    }

    // Append project context files
    if !context_files.is_empty() {
        prompt.push_str("\n\n<project_context>\n\n");
        prompt.push_str("Project-specific instructions and guidelines:\n\n");
        for cf in context_files {
            prompt.push_str(&format!(
                "<project_instructions path=\"{}\">\n{}\n</project_instructions>\n\n",
                cf.path, cf.content
            ));
        }
        prompt.push_str("</project_context>\n");
    }

    // Append skills index
    if !skills.is_empty() {
        prompt.push_str(&format_skills_for_prompt(skills));
    }

    // Add date and working directory last
    prompt.push_str(&format!("\nCurrent date: {}", date));
    prompt.push_str(&format!("\nCurrent working directory: {}", prompt_cwd));

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_empty_tools_shows_none() {
        let options = BuildSystemPromptOptions {
            selected_tools: Some(vec![]),
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);
        assert!(prompt.contains("Available tools:\n(none)"));
    }

    #[test]
    fn test_empty_tools_shows_file_paths_guideline() {
        let options = BuildSystemPromptOptions {
            selected_tools: Some(vec![]),
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);
        assert!(prompt.contains("Show file paths clearly"));
    }

    #[test]
    fn test_default_tools_with_snippets() {
        let mut snippets = HashMap::new();
        snippets.insert("read".to_string(), "Read file contents".to_string());
        snippets.insert("bash".to_string(), "Execute bash commands".to_string());
        snippets.insert("edit".to_string(), "Make surgical edits".to_string());
        snippets.insert("write".to_string(), "Create or overwrite files".to_string());

        let options = BuildSystemPromptOptions {
            tool_snippets: Some(snippets),
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);
        assert!(prompt.contains("- read:"));
        assert!(prompt.contains("- bash:"));
        assert!(prompt.contains("- edit:"));
        assert!(prompt.contains("- write:"));
    }

    #[test]
    fn test_default_tools_resolve_paths_guideline() {
        let options = BuildSystemPromptOptions {
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);
        assert!(prompt.contains(
            "- Resolve docs/... under More docs and examples/... under Examples, not the cwd"
        ));
    }

    #[test]
    fn test_custom_tool_snippet_included() {
        let mut snippets = HashMap::new();
        snippets.insert(
            "dynamic_tool".to_string(),
            "Run dynamic test behavior".to_string(),
        );

        let options = BuildSystemPromptOptions {
            selected_tools: Some(vec!["read".into(), "dynamic_tool".into()]),
            tool_snippets: Some(snippets),
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);
        assert!(prompt.contains("- dynamic_tool: Run dynamic test behavior"));
    }

    #[test]
    fn test_custom_tool_omitted_without_snippet() {
        let options = BuildSystemPromptOptions {
            selected_tools: Some(vec!["read".into(), "dynamic_tool".into()]),
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);
        assert!(!prompt.contains("dynamic_tool"));
    }

    #[test]
    fn test_prompt_guidelines_appended() {
        let options = BuildSystemPromptOptions {
            selected_tools: Some(vec!["read".into(), "dynamic_tool".into()]),
            prompt_guidelines: Some(vec!["Use dynamic_tool for project summaries.".into()]),
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);
        assert!(prompt.contains("- Use dynamic_tool for project summaries."));
    }

    #[test]
    fn test_prompt_guidelines_deduplicated_and_trimmed() {
        let options = BuildSystemPromptOptions {
            selected_tools: Some(vec!["read".into(), "dynamic_tool".into()]),
            prompt_guidelines: Some(vec![
                "Use dynamic_tool for summaries.".into(),
                "  Use dynamic_tool for summaries.  ".into(),
                "   ".into(),
            ]),
            cwd: "/tmp/test".to_string(),
            ..Default::default()
        };
        let prompt = build_system_prompt(&options);

        // Count exactly the number of occurrences
        let count = prompt
            .match_indices("- Use dynamic_tool for summaries.")
            .count();
        assert_eq!(count, 1);
    }
}
