//! Port of `packages/agent/src/harness/prompt-templates.ts`.

use crate::harness::types::{ExecutionEnv, FileInfo, PromptTemplate};
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptTemplateDiagnosticCode {
    FileInfoFailed,
    ListFailed,
    ReadFailed,
    ParseFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptTemplateDiagnostic {
    pub r#type: String,
    pub code: PromptTemplateDiagnosticCode,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcedPromptTemplate<TSource, TPromptTemplate = PromptTemplate> {
    pub prompt_template: TPromptTemplate,
    pub source: TSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcedPromptTemplateDiagnostic<TSource> {
    pub diagnostic: PromptTemplateDiagnostic,
    pub source: TSource,
}

pub async fn load_prompt_templates<E: ExecutionEnv>(
    env: &E,
    paths: &[String],
) -> (Vec<PromptTemplate>, Vec<PromptTemplateDiagnostic>) {
    let mut prompt_templates = Vec::new();
    let mut diagnostics = Vec::new();

    for path in paths {
        let info = match env.file_info(path, None).await {
            Ok(info) => info,
            Err(error) => {
                if error.code != crate::harness::types::FileErrorCode::NotFound {
                    diagnostics.push(PromptTemplateDiagnostic {
                        r#type: "warning".to_string(),
                        code: PromptTemplateDiagnosticCode::FileInfoFailed,
                        message: error.message,
                        path: path.clone(),
                    });
                }
                continue;
            }
        };

        match resolve_kind(env, &info, &mut diagnostics).await {
            Some("directory") => {
                let (mut templates, mut dir_diagnostics) =
                    load_templates_from_dir(env, &info.path).await;
                prompt_templates.append(&mut templates);
                diagnostics.append(&mut dir_diagnostics);
            }
            Some("file") if info.name.ends_with(".md") => {
                let (template, mut file_diagnostics) =
                    load_template_from_file(env, &info.path).await;
                if let Some(template) = template {
                    prompt_templates.push(template);
                }
                diagnostics.append(&mut file_diagnostics);
            }
            _ => {}
        }
    }

    (prompt_templates, diagnostics)
}

pub async fn load_sourced_prompt_templates<E, TSource, TPromptTemplate, F>(
    env: &E,
    inputs: &[(String, TSource)],
    map_prompt_template: Option<F>,
) -> (
    Vec<SourcedPromptTemplate<TSource, TPromptTemplate>>,
    Vec<SourcedPromptTemplateDiagnostic<TSource>>,
)
where
    E: ExecutionEnv,
    TSource: Clone,
    TPromptTemplate: From<PromptTemplate>,
    F: Fn(PromptTemplate, &TSource) -> TPromptTemplate,
{
    let mut prompt_templates = Vec::new();
    let mut diagnostics = Vec::new();

    for (path, source) in inputs {
        let (loaded_templates, loaded_diagnostics) =
            load_prompt_templates(env, std::slice::from_ref(path)).await;
        for prompt_template in loaded_templates {
            let prompt_template = if let Some(map_prompt_template) = &map_prompt_template {
                map_prompt_template(prompt_template, source)
            } else {
                prompt_template.into()
            };
            prompt_templates.push(SourcedPromptTemplate {
                prompt_template,
                source: source.clone(),
            });
        }
        for diagnostic in loaded_diagnostics {
            diagnostics.push(SourcedPromptTemplateDiagnostic {
                diagnostic,
                source: source.clone(),
            });
        }
    }

    (prompt_templates, diagnostics)
}

async fn load_templates_from_dir<E: ExecutionEnv>(
    env: &E,
    dir: &str,
) -> (Vec<PromptTemplate>, Vec<PromptTemplateDiagnostic>) {
    let mut prompt_templates = Vec::new();
    let mut diagnostics = Vec::new();

    let entries = match env.list_dir(dir, None).await {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(PromptTemplateDiagnostic {
                r#type: "warning".to_string(),
                code: PromptTemplateDiagnosticCode::ListFailed,
                message: error.message,
                path: dir.to_string(),
            });
            return (prompt_templates, diagnostics);
        }
    };

    let mut entries = entries;
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    for entry in entries {
        match resolve_kind(env, &entry, &mut diagnostics).await {
            Some("file") if entry.name.ends_with(".md") => {
                let (template, mut file_diagnostics) =
                    load_template_from_file(env, &entry.path).await;
                if let Some(template) = template {
                    prompt_templates.push(template);
                }
                diagnostics.append(&mut file_diagnostics);
            }
            _ => {}
        }
    }

    (prompt_templates, diagnostics)
}

async fn load_template_from_file<E: ExecutionEnv>(
    env: &E,
    file_path: &str,
) -> (Option<PromptTemplate>, Vec<PromptTemplateDiagnostic>) {
    let mut diagnostics = Vec::new();
    let raw_content = match env.read_text_file(file_path, None).await {
        Ok(content) => content,
        Err(error) => {
            diagnostics.push(PromptTemplateDiagnostic {
                r#type: "warning".to_string(),
                code: PromptTemplateDiagnosticCode::ReadFailed,
                message: error.message,
                path: file_path.to_string(),
            });
            return (None, diagnostics);
        }
    };

    let (frontmatter, body) = match parse_frontmatter(&raw_content) {
        Ok(parsed) => parsed,
        Err(error) => {
            diagnostics.push(PromptTemplateDiagnostic {
                r#type: "warning".to_string(),
                code: PromptTemplateDiagnosticCode::ParseFailed,
                message: error,
                path: file_path.to_string(),
            });
            return (None, diagnostics);
        }
    };

    let first_line = body.lines().find(|line| !line.trim().is_empty());
    let mut description = frontmatter
        .get("description")
        .and_then(yaml_value_as_string)
        .unwrap_or_default();
    if description.is_empty() {
        if let Some(first_line) = first_line {
            description = if first_line.len() > 60 {
                format!("{}...", &first_line[..60])
            } else {
                first_line.to_string()
            };
        }
    }

    (
        Some(PromptTemplate {
            name: basename_env_path(file_path)
                .trim_end_matches(".md")
                .to_string(),
            description: if description.is_empty() {
                None
            } else {
                Some(description)
            },
            content: body,
        }),
        diagnostics,
    )
}

async fn resolve_kind<E: ExecutionEnv>(
    env: &E,
    info: &FileInfo,
    diagnostics: &mut Vec<PromptTemplateDiagnostic>,
) -> Option<&'static str> {
    match info.kind {
        crate::harness::types::FileKind::File => return Some("file"),
        crate::harness::types::FileKind::Directory => return Some("directory"),
        crate::harness::types::FileKind::Symlink => {}
    }

    let canonical_path = match env.canonical_path(&info.path, None).await {
        Ok(path) => path,
        Err(error) => {
            if error.code != crate::harness::types::FileErrorCode::NotFound {
                diagnostics.push(PromptTemplateDiagnostic {
                    r#type: "warning".to_string(),
                    code: PromptTemplateDiagnosticCode::FileInfoFailed,
                    message: error.message,
                    path: info.path.clone(),
                });
            }
            return None;
        }
    };

    let target = match env.file_info(&canonical_path, None).await {
        Ok(target) => target,
        Err(error) => {
            if error.code != crate::harness::types::FileErrorCode::NotFound {
                diagnostics.push(PromptTemplateDiagnostic {
                    r#type: "warning".to_string(),
                    code: PromptTemplateDiagnosticCode::FileInfoFailed,
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

fn yaml_value_as_string(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(value) => Some(value.clone()),
        _ => None,
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

pub fn parse_command_args(args_string: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;

    for character in args_string.chars() {
        if let Some(quote) = in_quote {
            if character == quote {
                in_quote = None;
            } else {
                current.push(character);
            }
        } else if character == '"' || character == '\'' {
            in_quote = Some(character);
        } else if character == ' ' || character == '\t' {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

pub fn substitute_args(content: &str, args: &[String]) -> String {
    let mut result = content.to_string();

    for index in (1..=args.len()).rev() {
        result = result.replace(&format!("${index}"), &args[index - 1]);
    }

    loop {
        let Some(start) = result.find("${@:") else {
            break;
        };
        let Some(end) = result[start..].find('}') else {
            break;
        };
        let end = start + end;
        let spec = &result[start + 4..end];
        let replacement = if let Some((start_str, length_str)) = spec.split_once(':') {
            let start_index = start_str.parse::<usize>().unwrap_or(1).saturating_sub(1);
            let length = length_str.parse::<usize>().unwrap_or(0);
            args.iter()
                .skip(start_index)
                .take(length)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            let start_index = spec.parse::<usize>().unwrap_or(1).saturating_sub(1);
            args.iter()
                .skip(start_index)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        };

        result.replace_range(start..=end, &replacement);
    }

    let all_args = args.join(" ");
    result = result.replace("$ARGUMENTS", &all_args);
    result = result.replace("$@", &all_args);
    result
}

pub fn format_prompt_template_invocation(
    template: &PromptTemplate,
    args: Option<&[String]>,
) -> String {
    substitute_args(&template.content, args.unwrap_or(&[]))
}

#[cfg(test)]
mod tests {
    use super::{
        PromptTemplate, format_prompt_template_invocation, load_prompt_templates,
        load_sourced_prompt_templates,
    };
    use crate::harness::env::nodejs::NodeExecutionEnv;
    use crate::harness::types::FileSystem;

    #[tokio::test]
    async fn loads_templates_from_directories() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.create_dir("a/nested", true, None).await.unwrap();
        env.create_dir("b", true, None).await.unwrap();
        env.write_file(
            "a/one.md",
            b"---\ndescription: One template\n---\nHello $1",
            None,
        )
        .await
        .unwrap();
        env.write_file("a/nested/ignored.md", b"Ignored", None)
            .await
            .unwrap();
        env.write_file("b/two.md", b"First line description\nBody", None)
            .await
            .unwrap();

        let (prompt_templates, diagnostics) =
            load_prompt_templates(&env, &["a".to_string(), "b".to_string()]).await;

        assert!(diagnostics.is_empty());
        assert_eq!(
            prompt_templates,
            vec![
                PromptTemplate {
                    name: "one".to_string(),
                    description: Some("One template".to_string()),
                    content: "Hello $1".to_string(),
                },
                PromptTemplate {
                    name: "two".to_string(),
                    description: Some("First line description".to_string()),
                    content: "First line description\nBody".to_string(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn preserves_source_info() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.create_dir("prompts", true, None).await.unwrap();
        env.write_file(
            "prompts/example.md",
            b"---\ndescription: Example\n---\nExample body",
            None,
        )
        .await
        .unwrap();

        let (prompt_templates, diagnostics) =
            load_sourced_prompt_templates::<_, _, PromptTemplate, _>(
                &env,
                &[("prompts".to_string(), "project".to_string())],
                None::<fn(PromptTemplate, &String) -> PromptTemplate>,
            )
            .await;

        assert!(diagnostics.is_empty());
        assert_eq!(prompt_templates.len(), 1);
        assert_eq!(prompt_templates[0].source, "project");
        assert_eq!(prompt_templates[0].prompt_template.name, "example");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn loads_symlinked_files() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        env.write_file(
            "target.md",
            b"---\ndescription: Target\n---\nTarget body",
            None,
        )
        .await
        .unwrap();
        symlink(root.path().join("target.md"), root.path().join("link.md")).unwrap();

        let (prompt_templates, diagnostics) =
            load_prompt_templates(&env, &["target.md".to_string(), "link.md".to_string()]).await;

        assert!(diagnostics.is_empty());
        assert_eq!(prompt_templates.len(), 2);
        assert_eq!(prompt_templates[1].name, "link");
    }

    #[test]
    fn formats_prompt_invocations() {
        let content = "$1 ${@:2} $ARGUMENTS".to_string();
        let template = PromptTemplate {
            name: "one".to_string(),
            description: None,
            content,
        };
        let output = format_prompt_template_invocation(
            &template,
            Some(&["hello world".to_string(), "test".to_string()]),
        );
        assert_eq!(output, "hello world test hello world test");
    }
}
