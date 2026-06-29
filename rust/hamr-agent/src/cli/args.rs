//! CLI argument parsing and help display.
//!
//! Mirror of `packages/coding-agent/src/cli/args.ts`.

use hamr_ai::types::ModelThinkingLevel;
use std::collections::HashMap;

/// Output mode for the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Text,
    Json,
    Rpc,
}

impl Mode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Mode::Text),
            "json" => Some(Mode::Json),
            "rpc" => Some(Mode::Rpc),
            _ => None,
        }
    }
}

/// A diagnostic message produced during argument parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub diag_type: DiagnosticType,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticType {
    Warning,
    Error,
}

/// Parsed CLI arguments.
///
/// Mirror of the TypeScript `Args` interface.
#[derive(Debug, Clone, Default)]
pub struct Args {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub system_prompt: Option<String>,
    pub append_system_prompt: Vec<String>,
    pub thinking: Option<ModelThinkingLevel>,
    pub r#continue: bool,
    pub resume: bool,
    pub help: bool,
    pub version: bool,
    pub mode: Option<Mode>,
    pub name: Option<String>,
    pub no_session: bool,
    pub session: Option<String>,
    pub session_id: Option<String>,
    pub fork: Option<String>,
    pub session_dir: Option<String>,
    pub models: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub exclude_tools: Option<Vec<String>>,
    pub no_tools: bool,
    pub no_builtin_tools: bool,
    pub extensions: Option<Vec<String>>,
    pub no_extensions: bool,
    pub print: bool,
    pub export: Option<String>,
    pub no_skills: bool,
    pub skills: Option<Vec<String>>,
    pub prompt_templates: Option<Vec<String>>,
    pub no_prompt_templates: bool,
    pub themes: Option<Vec<String>>,
    pub no_themes: bool,
    pub no_context_files: bool,
    pub list_models: Option<String>,
    pub list_models_flag: bool,
    pub offline: bool,
    pub verbose: bool,
    pub project_trust_override: Option<bool>,
    pub messages: Vec<String>,
    pub file_args: Vec<String>,
    /// Unknown flags (potentially extension flags) - map of flag name to value.
    pub unknown_flags: HashMap<String, FlagValue>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Value for unknown CLI flags — either a boolean or a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagValue {
    Bool(bool),
    String(String),
}

/// The valid thinking levels as string slices matching ModelThinkingLevel variants.
pub const VALID_THINKING_LEVELS: &[&str] = &["off", "minimal", "low", "medium", "high", "xhigh"];

/// Check if a string is a valid thinking level.
pub fn is_valid_thinking_level(level: &str) -> Option<ModelThinkingLevel> {
    match level {
        "off" => Some(ModelThinkingLevel::Off),
        "minimal" => Some(ModelThinkingLevel::Minimal),
        "low" => Some(ModelThinkingLevel::Low),
        "medium" => Some(ModelThinkingLevel::Medium),
        "high" => Some(ModelThinkingLevel::High),
        "xhigh" => Some(ModelThinkingLevel::XHigh),
        _ => None,
    }
}

/// Parse command-line arguments into an `Args` struct.
///
/// This mirrors the TypeScript `parseArgs` function exactly.
pub fn parse_args(raw_args: &[String]) -> Args {
    let mut result = Args::default();
    let args: Vec<&str> = raw_args.iter().map(|s| s.as_str()).collect();
    let mut i = 0;

    while i < args.len() {
        let arg = args[i];

        match arg {
            "--help" | "-h" => result.help = true,

            "--version" | "-v" => result.version = true,

            "--mode" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.mode = Mode::from_str(args[i]);
                }
            }

            "--continue" | "-c" => result.r#continue = true,

            "--resume" | "-r" => result.resume = true,

            "--provider" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.provider = Some(args[i].to_string());
                }
            }

            "--model" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.model = Some(args[i].to_string());
                }
            }

            "--api-key" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.api_key = Some(args[i].to_string());
                }
            }

            "--system-prompt" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.system_prompt = Some(args[i].to_string());
                }
            }

            "--append-system-prompt" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.append_system_prompt.push(args[i].to_string());
                }
            }

            "--name" | "-n" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.name = Some(args[i].to_string());
                } else {
                    result.diagnostics.push(Diagnostic {
                        diag_type: DiagnosticType::Error,
                        message: "--name requires a value".to_string(),
                    });
                }
            }

            "--no-session" => result.no_session = true,

            "--session" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.session = Some(args[i].to_string());
                }
            }

            "--session-id" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.session_id = Some(args[i].to_string());
                }
            }

            "--fork" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.fork = Some(args[i].to_string());
                }
            }

            "--session-dir" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.session_dir = Some(args[i].to_string());
                }
            }

            "--models" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.models =
                        Some(args[i].split(',').map(|s| s.trim().to_string()).collect());
                }
            }

            "--no-tools" | "-nt" => result.no_tools = true,

            "--no-builtin-tools" | "-nbt" => result.no_builtin_tools = true,

            "--tools" | "-t" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.tools = Some(
                        args[i]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|name| !name.is_empty())
                            .collect(),
                    );
                }
            }

            "--exclude-tools" | "-xt" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.exclude_tools = Some(
                        args[i]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|name| !name.is_empty())
                            .collect(),
                    );
                }
            }

            "--thinking" => {
                if i + 1 < args.len() {
                    i += 1;
                    let level = args[i];
                    match is_valid_thinking_level(level) {
                        Some(tl) => result.thinking = Some(tl),
                        None => {
                            result.diagnostics.push(Diagnostic {
                                diag_type: DiagnosticType::Warning,
                                message: format!(
                                    "Invalid thinking level \"{}\". Valid values: {}",
                                    level,
                                    VALID_THINKING_LEVELS.join(", ")
                                ),
                            });
                        }
                    }
                }
            }

            "--print" | "-p" => {
                result.print = true;
                // In TS, after --print, the next non-flag arg becomes part of messages
                let next = if i + 1 < args.len() {
                    Some(args[i + 1])
                } else {
                    None
                };
                if let Some(next_val) = next {
                    if !next_val.starts_with('@')
                        && (!next_val.starts_with('-') || next_val.starts_with("---"))
                    {
                        result.messages.push(next_val.to_string());
                        i += 1;
                    }
                }
            }

            "--export" => {
                if i + 1 < args.len() {
                    i += 1;
                    result.export = Some(args[i].to_string());
                }
            }

            "--extension" | "-e" => {
                if i + 1 < args.len() {
                    i += 1;
                    result
                        .extensions
                        .get_or_insert_with(Vec::new)
                        .push(args[i].to_string());
                }
            }

            "--no-extensions" | "-ne" => result.no_extensions = true,

            "--skill" => {
                if i + 1 < args.len() {
                    i += 1;
                    result
                        .skills
                        .get_or_insert_with(Vec::new)
                        .push(args[i].to_string());
                }
            }

            "--prompt-template" => {
                if i + 1 < args.len() {
                    i += 1;
                    result
                        .prompt_templates
                        .get_or_insert_with(Vec::new)
                        .push(args[i].to_string());
                }
            }

            "--theme" => {
                if i + 1 < args.len() {
                    i += 1;
                    result
                        .themes
                        .get_or_insert_with(Vec::new)
                        .push(args[i].to_string());
                }
            }

            "--no-skills" | "-ns" => result.no_skills = true,

            "--no-prompt-templates" | "-np" => result.no_prompt_templates = true,

            "--no-themes" => result.no_themes = true,

            "--no-context-files" | "-nc" => result.no_context_files = true,

            "--list-models" => {
                // Check if next arg is a search pattern (not a flag or file arg)
                if i + 1 < args.len()
                    && !args[i + 1].starts_with('-')
                    && !args[i + 1].starts_with('@')
                {
                    i += 1;
                    result.list_models = Some(args[i].to_string());
                } else {
                    result.list_models_flag = true;
                }
            }

            "--verbose" => result.verbose = true,

            "--approve" | "-a" => result.project_trust_override = Some(true),

            "--no-approve" | "-na" => result.project_trust_override = Some(false),

            "--offline" => result.offline = true,

            // @file args
            arg if arg.starts_with('@') => {
                result.file_args.push(arg[1..].to_string());
            }

            // Unknown --flag or --flag=value
            arg if arg.starts_with("--") => {
                let flag_body = &arg[2..];
                if let Some(eq_idx) = flag_body.find('=') {
                    let name = flag_body[..eq_idx].to_string();
                    let value = flag_body[eq_idx + 1..].to_string();
                    result.unknown_flags.insert(name, FlagValue::String(value));
                } else {
                    let flag_name = flag_body.to_string();
                    let next = if i + 1 < args.len() {
                        Some(args[i + 1])
                    } else {
                        None
                    };
                    if let Some(next_val) = next {
                        if !next_val.starts_with('-') && !next_val.starts_with('@') {
                            result
                                .unknown_flags
                                .insert(flag_name, FlagValue::String(next_val.to_string()));
                            i += 1;
                        } else {
                            result
                                .unknown_flags
                                .insert(flag_name, FlagValue::Bool(true));
                        }
                    } else {
                        result
                            .unknown_flags
                            .insert(flag_name, FlagValue::Bool(true));
                    }
                }
            }

            // Unknown short flag
            arg if arg.starts_with('-') && !arg.starts_with("--") => {
                result.diagnostics.push(Diagnostic {
                    diag_type: DiagnosticType::Error,
                    message: format!("Unknown option: {}", arg),
                });
            }

            // Positional message argument
            _ => {
                result.messages.push(arg.to_string());
            }
        }

        i += 1;
    }

    result
}

/// A CLI flag registered by an extension.
///
/// Mirror of the TypeScript `ExtensionFlag` interface.
#[derive(Debug, Clone)]
pub struct ExtensionFlag {
    pub name: String,
    pub description: Option<String>,
    pub flag_type: ExtensionFlagType,
    pub default: Option<FlagDefault>,
    pub extension_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionFlagType {
    Boolean,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagDefault {
    Bool(bool),
    String(String),
}

/// Print the CLI help text.
///
/// Mirror of the TypeScript `printHelp` function.
pub fn print_help(
    app_name: &str,
    config_dir_name: &str,
    env_agent_dir: &str,
    env_session_dir: &str,
    extension_flags: Option<&[ExtensionFlag]>,
) {
    let extension_flags_text = match extension_flags {
        Some(flags) if !flags.is_empty() => {
            let mut text = String::from("\nExtension CLI Flags:\n");
            for flag in flags {
                let value = match flag.flag_type {
                    ExtensionFlagType::String => " <value>",
                    ExtensionFlagType::Boolean => "",
                };
                let default_desc = format!("Registered by {}", flag.extension_path);
                let description = flag.description.as_deref().unwrap_or(&default_desc);
                text.push_str(&format!(
                    "  --{}{:<width$}{} {}\n",
                    flag.name,
                    value,
                    "",
                    description,
                    width = 30usize.saturating_sub(flag.name.len() + value.len() + 4)
                ));
            }
            text
        }
        _ => String::new(),
    };

    println!(
        "{app_name} - AI coding assistant with read, bash, edit, write tools

Usage:
  {app_name} [options] [@files...] [messages...]

Commands:
  {app_name} install <source> [-l]     Install extension source and add to settings
  {app_name} remove <source> [-l]      Remove extension source from settings
  {app_name} uninstall <source> [-l]   Alias for remove
  {app_name} update [source|self|hamr]  Update hamr and installed extensions
  {app_name} list                      List installed extensions from settings
  {app_name} config                    Open TUI to enable/disable package resources
  {app_name} <command> --help          Show help for install/remove/uninstall/update/list

Options:
  --provider <name>              Provider name (default: relay)
  --model <pattern>              Model pattern or ID (supports \"provider/id\" and optional \":<thinking>\")
  --api-key <key>                API key (defaults to env vars)
  --system-prompt <text>         System prompt (default: coding assistant prompt)
  --append-system-prompt <text>  Append text or file contents to the system prompt (can be used multiple times)
  --mode <mode>                  Output mode: text (default), json, or rpc
  --print, -p                    Non-interactive mode: process prompt and exit
  --continue, -c                 Continue previous session
  --resume, -r                   Select a session to resume
  --session <path|id>            Use specific session file or partial UUID
  --session-id <id>              Use exact project session ID, creating it if missing
  --fork <path|id>               Fork specific session file or partial UUID into a new session
  --session-dir <dir>            Directory for session storage and lookup
  --no-session                   Don't save session (ephemeral)
  --name, -n <name>              Set session display name
  --models <patterns>            Comma-separated model patterns for Ctrl+P cycling
                                 Supports globs (anthropic/*, *sonnet*) and fuzzy matching
  --no-tools, -nt                Disable all tools by default (built-in and extension)
  --no-builtin-tools, -nbt       Disable built-in tools by default but keep extension/custom tools enabled
  --tools, -t <tools>            Comma-separated allowlist of tool names to enable
                                 Applies to built-in, extension, and custom tools
  --exclude-tools, -xt <tools>   Comma-separated denylist of tool names to disable
                                 Applies to built-in, extension, and custom tools
  --thinking <level>             Set thinking level: off, minimal, low, medium, high, xhigh
  --extension, -e <path>         Load an extension file (can be used multiple times)
  --no-extensions, -ne           Disable extension discovery (explicit -e paths still work)
  --skill <path>                 Load a skill file or directory (can be used multiple times)
  --no-skills, -ns               Disable skills discovery and loading
  --prompt-template <path>       Load a prompt template file or directory (can be used multiple times)
  --no-prompt-templates, -np     Disable prompt template discovery and loading
  --theme <path>                 Load a theme file or directory (can be used multiple times)
  --no-themes                    Disable theme discovery and loading
  --no-context-files, -nc        Disable AGENTS.md and CLAUDE.md discovery and loading
  --export <file>                Export session file to HTML and exit
  --list-models [search]         List available models (with optional fuzzy search)
  --verbose                      Force verbose startup (overrides quietStartup setting)
  --approve, -a                  Trust project-local files for this run
  --no-approve, -na              Ignore project-local files for this run
  --offline                      Disable startup network operations (same as PI_OFFLINE=1)
  --help, -h                     Show this help
  --version, -v                  Show version number

Extensions can register additional flags (e.g., --plan from plan-mode extension).{ext_flags}

Examples:
  # Interactive mode
  {app_name}

  # Interactive mode with initial prompt
  {app_name} \"List all .ts files in src/\"

  # Include files in initial message
  {app_name} @prompt.md @image.png \"What color is the sky?\"

  # Non-interactive mode (process and exit)
  {app_name} -p \"List all .ts files in src/\"

  # Multiple messages (interactive)
  {app_name} \"Read package.json\" \"What dependencies do we have?\"

  # Continue previous session
  {app_name} --continue \"What did we discuss?\"

  # Start a named session
  {app_name} --name \"Refactor auth module\"

  # Use different model
  {app_name} --provider openai --model gpt-4o-mini \"Help me refactor this code\"

  # Use model with provider prefix (no --provider needed)
  {app_name} --model openai/gpt-4o \"Help me refactor this code\"

  # Use model with thinking level shorthand
  {app_name} --model sonnet:high \"Solve this complex problem\"

  # Limit model cycling to specific models
  {app_name} --models claude-sonnet,claude-haiku,gpt-4o

  # Limit to a specific provider with glob pattern
  {app_name} --models \"github-copilot/*\"

  # Cycle models with fixed thinking levels
  {app_name} --models sonnet:high,haiku:low

  # Start with a specific thinking level
  {app_name} --thinking high \"Solve this complex problem\"

  # Read-only mode (no file modifications possible)
  {app_name} --tools read,grep,find,ls -p \"Review the code in src/\"

  # Disable one tool while keeping the rest available
  {app_name} --exclude-tools ask_question

  # Export a session file to HTML
  {app_name} --export ~/{config_dir}/agent/sessions/--path--/session.jsonl
  {app_name} --export session.jsonl output.html

Environment Variables:
  ANTHROPIC_API_KEY                - Anthropic Claude API key
  ANTHROPIC_OAUTH_TOKEN            - Anthropic OAuth token (alternative to API key)
  ANT_LING_API_KEY                 - Ant Ling API key
  OPENAI_API_KEY                   - OpenAI GPT API key
  AZURE_OPENAI_API_KEY             - Azure OpenAI API key
  AZURE_OPENAI_BASE_URL            - Azure OpenAI/Cognitive Services base URL (e.g. https://{{resource}}.openai.azure.com)
  AZURE_OPENAI_RESOURCE_NAME       - Azure OpenAI resource name (alternative to base URL)
  AZURE_OPENAI_API_VERSION         - Azure OpenAI API version (default: v1)
  AZURE_OPENAI_DEPLOYMENT_NAME_MAP - Azure OpenAI model=deployment map (comma-separated)
  DEEPSEEK_API_KEY                 - DeepSeek API key
  NVIDIA_API_KEY                   - NVIDIA NIM API key
  GEMINI_API_KEY                   - Google Gemini API key
  GROQ_API_KEY                     - Groq API key
  CEREBRAS_API_KEY                 - Cerebras API key
  XAI_API_KEY                      - xAI Grok API key
  FIREWORKS_API_KEY                - Fireworks API key
  TOGETHER_API_KEY                 - Together AI API key
  OPENROUTER_API_KEY               - OpenRouter API key
  AI_GATEWAY_API_KEY               - Vercel AI Gateway API key
  ZAI_API_KEY                      - ZAI API key
  ZAI_CODING_CN_API_KEY            - ZAI Coding Plan API key (China)
  MISTRAL_API_KEY                  - Mistral API key
  MINIMAX_API_KEY                  - MiniMax API key
  MOONSHOT_API_KEY                 - Moonshot AI API key
  OPENCODE_API_KEY                 - OpenCode Zen/OpenCode Go API key
  KIMI_API_KEY                     - Kimi For Coding API key
  CLOUDFLARE_API_KEY               - Cloudflare API token (Workers AI and AI Gateway)
  CLOUDFLARE_ACCOUNT_ID            - Cloudflare account id (required for both)
  CLOUDFLARE_GATEWAY_ID            - Cloudflare AI Gateway slug (required for AI Gateway)
  XIAOMI_API_KEY                   - Xiaomi MiMo API key (api.xiaomimimo.com billing)
  XIAOMI_TOKEN_PLAN_CN_API_KEY     - Xiaomi MiMo Token Plan API key (China region)
  XIAOMI_TOKEN_PLAN_AMS_API_KEY    - Xiaomi MiMo Token Plan API key (Amsterdam region)
  XIAOMI_TOKEN_PLAN_SGP_API_KEY    - Xiaomi MiMo Token Plan API key (Singapore region)
  AWS_PROFILE                      - AWS profile for Amazon Bedrock
  AWS_ACCESS_KEY_ID                - AWS access key for Amazon Bedrock
  AWS_SECRET_ACCESS_KEY            - AWS secret key for Amazon Bedrock
  AWS_BEARER_TOKEN_BEDROCK         - Bedrock API key (bearer token)
  AWS_REGION                       - AWS region for Amazon Bedrock (e.g., us-east-1)
  {env_agent:<pad$} - Config directory (default: ~/{config_dir}/agent)
  {env_session:<pad$} - Session storage directory (overridden by --session-dir)
  HAMR_PACKAGE_DIR                 - Override package directory (for Nix/Guix store paths)
  HAMR_OFFLINE                     - Disable startup network operations when set to 1/true/yes
  HAMR_TELEMETRY                   - Override install telemetry when set to 1/true/yes or 0/false/no
  HAMR_SHARE_VIEWER_URL            - Base URL for /share command (default: https://hamr.dev/session/)

Built-in Tool Names:
  read   - Read file contents
  bash   - Execute bash commands
  edit   - Edit files with find/replace
  write  - Write files (creates/overwrites)
  grep   - Search file contents (read-only, off by default)
  find   - Find files by glob pattern (read-only, off by default)
  ls     - List directory contents (read-only, off by default)
",
        app_name = app_name,
        ext_flags = extension_flags_text,
        config_dir = config_dir_name,
        env_agent = env_agent_dir,
        env_session = env_session_dir,
        pad = 32,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_empty() {
        let args = parse_args(&[]);
        assert!(!args.help);
        assert!(!args.version);
        assert!(args.messages.is_empty());
        assert!(args.file_args.is_empty());
        assert!(args.unknown_flags.is_empty());
        assert!(args.diagnostics.is_empty());
    }

    #[test]
    fn test_parse_args_help_short() {
        let args = parse_args(&["-h".to_string()]);
        assert!(args.help);
    }

    #[test]
    fn test_parse_args_help_long() {
        let args = parse_args(&["--help".to_string()]);
        assert!(args.help);
    }

    #[test]
    fn test_parse_args_version_short() {
        let args = parse_args(&["-v".to_string()]);
        assert!(args.version);
    }

    #[test]
    fn test_parse_args_version_long() {
        let args = parse_args(&["--version".to_string()]);
        assert!(args.version);
    }

    #[test]
    fn test_parse_args_mode() {
        let args = parse_args(&["--mode".to_string(), "json".to_string()]);
        assert_eq!(args.mode, Some(Mode::Json));
    }

    #[test]
    fn test_parse_args_mode_invalid() {
        let args = parse_args(&["--mode".to_string(), "invalid".to_string()]);
        assert_eq!(args.mode, None);
    }

    #[test]
    fn test_parse_args_continue_short() {
        let args = parse_args(&["-c".to_string()]);
        assert!(args.r#continue);
    }

    #[test]
    fn test_parse_args_continue_long() {
        let args = parse_args(&["--continue".to_string()]);
        assert!(args.r#continue);
    }

    #[test]
    fn test_parse_args_resume_short() {
        let args = parse_args(&["-r".to_string()]);
        assert!(args.resume);
    }

    #[test]
    fn test_parse_args_resume_long() {
        let args = parse_args(&["--resume".to_string()]);
        assert!(args.resume);
    }

    #[test]
    fn test_parse_args_provider() {
        let args = parse_args(&["--provider".to_string(), "openai".to_string()]);
        assert_eq!(args.provider, Some("openai".to_string()));
    }

    #[test]
    fn test_parse_args_model() {
        let args = parse_args(&["--model".to_string(), "gpt-4o".to_string()]);
        assert_eq!(args.model, Some("gpt-4o".to_string()));
    }

    #[test]
    fn test_parse_args_api_key() {
        let args = parse_args(&["--api-key".to_string(), "sk-test".to_string()]);
        assert_eq!(args.api_key, Some("sk-test".to_string()));
    }

    #[test]
    fn test_parse_args_system_prompt() {
        let args = parse_args(&["--system-prompt".to_string(), "You are helpful".to_string()]);
        assert_eq!(args.system_prompt, Some("You are helpful".to_string()));
    }

    #[test]
    fn test_parse_args_append_system_prompt() {
        let args = parse_args(&[
            "--append-system-prompt".to_string(),
            "extra".to_string(),
            "--append-system-prompt".to_string(),
            "more".to_string(),
        ]);
        assert_eq!(args.append_system_prompt, vec!["extra", "more"]);
    }

    #[test]
    fn test_parse_args_name() {
        let args = parse_args(&["--name".to_string(), "test-session".to_string()]);
        assert_eq!(args.name, Some("test-session".to_string()));
    }

    #[test]
    fn test_parse_args_name_short() {
        let args = parse_args(&["-n".to_string(), "test-session".to_string()]);
        assert_eq!(args.name, Some("test-session".to_string()));
    }

    #[test]
    fn test_parse_args_name_missing_value() {
        let args = parse_args(&["--name".to_string()]);
        assert_eq!(args.name, None);
        assert_eq!(args.diagnostics.len(), 1);
        assert_eq!(args.diagnostics[0].diag_type, DiagnosticType::Error);
        assert_eq!(args.diagnostics[0].message, "--name requires a value");
    }

    #[test]
    fn test_parse_args_no_session() {
        let args = parse_args(&["--no-session".to_string()]);
        assert!(args.no_session);
    }

    #[test]
    fn test_parse_args_session() {
        let args = parse_args(&["--session".to_string(), "abc123".to_string()]);
        assert_eq!(args.session, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_args_session_id() {
        let args = parse_args(&["--session-id".to_string(), "proj-id".to_string()]);
        assert_eq!(args.session_id, Some("proj-id".to_string()));
    }

    #[test]
    fn test_parse_args_fork() {
        let args = parse_args(&["--fork".to_string(), "abc123".to_string()]);
        assert_eq!(args.fork, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_args_session_dir() {
        let args = parse_args(&["--session-dir".to_string(), "/tmp/sessions".to_string()]);
        assert_eq!(args.session_dir, Some("/tmp/sessions".to_string()));
    }

    #[test]
    fn test_parse_args_models() {
        let args = parse_args(&["--models".to_string(), "sonnet,haiku".to_string()]);
        assert_eq!(
            args.models,
            Some(vec!["sonnet".to_string(), "haiku".to_string()])
        );
    }

    #[test]
    fn test_parse_args_no_tools_short() {
        let args = parse_args(&["-nt".to_string()]);
        assert!(args.no_tools);
    }

    #[test]
    fn test_parse_args_no_tools_long() {
        let args = parse_args(&["--no-tools".to_string()]);
        assert!(args.no_tools);
    }

    #[test]
    fn test_parse_args_no_builtin_tools_short() {
        let args = parse_args(&["-nbt".to_string()]);
        assert!(args.no_builtin_tools);
    }

    #[test]
    fn test_parse_args_no_builtin_tools_long() {
        let args = parse_args(&["--no-builtin-tools".to_string()]);
        assert!(args.no_builtin_tools);
    }

    #[test]
    fn test_parse_args_tools_allowlist() {
        let args = parse_args(&["--tools".to_string(), "read,edit".to_string()]);
        assert_eq!(
            args.tools,
            Some(vec!["read".to_string(), "edit".to_string()])
        );
    }

    #[test]
    fn test_parse_args_tools_short() {
        let args = parse_args(&["-t".to_string(), "read,edit".to_string()]);
        assert_eq!(
            args.tools,
            Some(vec!["read".to_string(), "edit".to_string()])
        );
    }

    #[test]
    fn test_parse_args_exclude_tools() {
        let args = parse_args(&["--exclude-tools".to_string(), "bash".to_string()]);
        assert_eq!(args.exclude_tools, Some(vec!["bash".to_string()]));
    }

    #[test]
    fn test_parse_args_exclude_tools_short() {
        let args = parse_args(&["-xt".to_string(), "bash".to_string()]);
        assert_eq!(args.exclude_tools, Some(vec!["bash".to_string()]));
    }

    #[test]
    fn test_parse_args_thinking_valid() {
        let args = parse_args(&["--thinking".to_string(), "high".to_string()]);
        assert_eq!(args.thinking, Some(ModelThinkingLevel::High));
    }

    #[test]
    fn test_parse_args_thinking_off() {
        let args = parse_args(&["--thinking".to_string(), "off".to_string()]);
        assert_eq!(args.thinking, Some(ModelThinkingLevel::Off));
    }

    #[test]
    fn test_parse_args_thinking_invalid() {
        let args = parse_args(&["--thinking".to_string(), "superhigh".to_string()]);
        assert_eq!(args.thinking, None);
        assert_eq!(args.diagnostics.len(), 1);
        assert_eq!(args.diagnostics[0].diag_type, DiagnosticType::Warning);
        assert!(
            args.diagnostics[0]
                .message
                .contains("Invalid thinking level")
        );
    }

    #[test]
    fn test_parse_args_print_short() {
        let args = parse_args(&["-p".to_string()]);
        assert!(args.print);
    }

    #[test]
    fn test_parse_args_print_long() {
        let args = parse_args(&["--print".to_string()]);
        assert!(args.print);
    }

    #[test]
    fn test_parse_args_print_with_message() {
        let args = parse_args(&["-p".to_string(), "hello".to_string()]);
        assert!(args.print);
        assert_eq!(args.messages, vec!["hello"]);
    }

    #[test]
    fn test_parse_args_export() {
        let args = parse_args(&["--export".to_string(), "output.html".to_string()]);
        assert_eq!(args.export, Some("output.html".to_string()));
    }

    #[test]
    fn test_parse_args_extension() {
        let args = parse_args(&["--extension".to_string(), "ext.js".to_string()]);
        assert_eq!(args.extensions, Some(vec!["ext.js".to_string()]));
    }

    #[test]
    fn test_parse_args_extension_short() {
        let args = parse_args(&["-e".to_string(), "ext.js".to_string()]);
        assert_eq!(args.extensions, Some(vec!["ext.js".to_string()]));
    }

    #[test]
    fn test_parse_args_extension_multiple() {
        let args = parse_args(&[
            "-e".to_string(),
            "a.js".to_string(),
            "-e".to_string(),
            "b.js".to_string(),
        ]);
        assert_eq!(
            args.extensions,
            Some(vec!["a.js".to_string(), "b.js".to_string()])
        );
    }

    #[test]
    fn test_parse_args_no_extensions() {
        let args = parse_args(&["--no-extensions".to_string()]);
        assert!(args.no_extensions);
    }

    #[test]
    fn test_parse_args_mode_rpc() {
        let args = parse_args(&["--mode".to_string(), "rpc".to_string()]);
        assert_eq!(args.mode, Some(Mode::Rpc));
    }

    #[test]
    fn test_parse_args_skill_single() {
        let args = parse_args(&["--skill".to_string(), "./skill-dir".to_string()]);
        assert_eq!(args.skills, Some(vec!["./skill-dir".to_string()]));
    }

    #[test]
    fn test_parse_args_skill_multiple() {
        let args = parse_args(&[
            "--skill".to_string(),
            "./skill-a".to_string(),
            "--skill".to_string(),
            "./skill-b".to_string(),
        ]);
        assert_eq!(
            args.skills,
            Some(vec!["./skill-a".to_string(), "./skill-b".to_string()])
        );
    }

    #[test]
    fn test_parse_args_prompt_template_single() {
        let args = parse_args(&["--prompt-template".to_string(), "./prompts".to_string()]);
        assert_eq!(args.prompt_templates, Some(vec!["./prompts".to_string()]));
    }

    #[test]
    fn test_parse_args_prompt_template_multiple() {
        let args = parse_args(&[
            "--prompt-template".to_string(),
            "./one".to_string(),
            "--prompt-template".to_string(),
            "./two".to_string(),
        ]);
        assert_eq!(
            args.prompt_templates,
            Some(vec!["./one".to_string(), "./two".to_string()])
        );
    }

    #[test]
    fn test_parse_args_theme_single() {
        let args = parse_args(&["--theme".to_string(), "./theme.json".to_string()]);
        assert_eq!(args.themes, Some(vec!["./theme.json".to_string()]));
    }

    #[test]
    fn test_parse_args_theme_multiple() {
        let args = parse_args(&[
            "--theme".to_string(),
            "./dark.json".to_string(),
            "--theme".to_string(),
            "./light.json".to_string(),
        ]);
        assert_eq!(
            args.themes,
            Some(vec!["./dark.json".to_string(), "./light.json".to_string()])
        );
    }

    #[test]
    fn test_parse_args_no_skills() {
        let args = parse_args(&["--no-skills".to_string()]);
        assert!(args.no_skills);
    }

    #[test]
    fn test_parse_args_no_prompt_templates() {
        let args = parse_args(&["--no-prompt-templates".to_string()]);
        assert!(args.no_prompt_templates);
    }

    #[test]
    fn test_parse_args_no_themes() {
        let args = parse_args(&["--no-themes".to_string()]);
        assert!(args.no_themes);
    }

    #[test]
    fn test_parse_args_no_context_files() {
        let args = parse_args(&["--no-context-files".to_string()]);
        assert!(args.no_context_files);
    }

    #[test]
    fn test_parse_args_no_context_files_short() {
        let args = parse_args(&["-nc".to_string()]);
        assert!(args.no_context_files);
    }

    #[test]
    fn test_parse_args_approve() {
        let args = parse_args(&["--approve".to_string()]);
        assert_eq!(args.project_trust_override, Some(true));
    }

    #[test]
    fn test_parse_args_approve_short() {
        let args = parse_args(&["-a".to_string()]);
        assert_eq!(args.project_trust_override, Some(true));
    }

    #[test]
    fn test_parse_args_no_approve() {
        let args = parse_args(&["--no-approve".to_string()]);
        assert_eq!(args.project_trust_override, Some(false));
    }

    #[test]
    fn test_parse_args_no_approve_short() {
        let args = parse_args(&["-na".to_string()]);
        assert_eq!(args.project_trust_override, Some(false));
    }

    #[test]
    fn test_parse_args_verbose() {
        let args = parse_args(&["--verbose".to_string()]);
        assert!(args.verbose);
    }

    #[test]
    fn test_parse_args_offline() {
        let args = parse_args(&["--offline".to_string()]);
        assert!(args.offline);
    }

    #[test]
    fn test_parse_args_messages_plain_text() {
        let args = parse_args(&["hello".to_string(), "world".to_string()]);
        assert_eq!(args.messages, vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_args_file_args() {
        let args = parse_args(&["@README.md".to_string(), "@src/main.ts".to_string()]);
        assert_eq!(args.file_args, vec!["README.md", "src/main.ts"]);
    }

    #[test]
    fn test_parse_args_mixed_messages_and_files() {
        let args = parse_args(&[
            "@file.txt".to_string(),
            "explain this".to_string(),
            "@image.png".to_string(),
        ]);
        assert_eq!(args.file_args, vec!["file.txt", "image.png"]);
        assert_eq!(args.messages, vec!["explain this"]);
    }

    #[test]
    fn test_parse_args_unknown_flag_string() {
        let args = parse_args(&["--unknown-flag".to_string(), "message".to_string()]);
        assert_eq!(args.messages.len(), 0);
        assert_eq!(
            args.unknown_flags.get("unknown-flag"),
            Some(&FlagValue::String("message".to_string()))
        );
    }

    #[test]
    fn test_parse_args_unknown_flag_boolean() {
        let args = parse_args(&["--unknown-flag".to_string()]);
        assert_eq!(
            args.unknown_flags.get("unknown-flag"),
            Some(&FlagValue::Bool(true))
        );
    }

    #[test]
    fn test_parse_args_unknown_flag_equals() {
        let args = parse_args(&["--unknown-flag=value".to_string()]);
        assert_eq!(
            args.unknown_flags.get("unknown-flag"),
            Some(&FlagValue::String("value".to_string()))
        );
    }

    #[test]
    fn test_parse_args_no_tools_with_explicit_tools() {
        let args = parse_args(&[
            "--no-tools".to_string(),
            "--tools".to_string(),
            "read,bash".to_string(),
        ]);
        assert!(args.no_tools);
        assert_eq!(
            args.tools,
            Some(vec!["read".to_string(), "bash".to_string()])
        );
    }

    #[test]
    fn test_parse_args_no_builtin_tools_with_explicit_tools() {
        let args = parse_args(&[
            "--no-builtin-tools".to_string(),
            "--tools".to_string(),
            "read,bash".to_string(),
        ]);
        assert!(args.no_builtin_tools);
        assert_eq!(
            args.tools,
            Some(vec!["read".to_string(), "bash".to_string()])
        );
    }

    #[test]
    fn test_parse_args_version_takes_precedence() {
        let args = parse_args(&[
            "--version".to_string(),
            "--help".to_string(),
            "some message".to_string(),
        ]);
        assert!(args.version);
        assert!(args.help);
        assert!(args.messages.contains(&"some message".to_string()));
    }

    #[test]
    fn test_parse_args_complex_combination() {
        let args = parse_args(&[
            "--provider".to_string(),
            "anthropic".to_string(),
            "--model".to_string(),
            "claude-sonnet".to_string(),
            "--print".to_string(),
            "--thinking".to_string(),
            "high".to_string(),
            "@prompt.md".to_string(),
            "Do the task".to_string(),
        ]);
        assert_eq!(args.provider, Some("anthropic".to_string()));
        assert_eq!(args.model, Some("claude-sonnet".to_string()));
        assert!(args.print);
        assert_eq!(args.thinking, Some(ModelThinkingLevel::High));
        assert_eq!(args.file_args, vec!["prompt.md"]);
        assert_eq!(args.messages, vec!["Do the task"]);
    }

    #[test]
    fn test_parse_args_name_with_other_flags() {
        let args = parse_args(&[
            "--name".to_string(),
            "named-run".to_string(),
            "--print".to_string(),
            "--model".to_string(),
            "gpt-4o".to_string(),
            "hello".to_string(),
        ]);
        assert_eq!(args.name, Some("named-run".to_string()));
        assert!(args.print);
        assert_eq!(args.model, Some("gpt-4o".to_string()));
        assert_eq!(args.messages, vec!["hello"]);
    }

    #[test]
    fn test_parse_args_name_preserves_empty() {
        let args = parse_args(&["--name".to_string(), "".to_string()]);
        assert_eq!(args.name, Some("".to_string()));
    }

    #[test]
    fn test_parse_args_no_extensions_with_explicit_e() {
        let args = parse_args(&[
            "--no-extensions".to_string(),
            "-e".to_string(),
            "foo.ts".to_string(),
            "-e".to_string(),
            "bar.ts".to_string(),
        ]);
        assert!(args.no_extensions);
        assert_eq!(
            args.extensions,
            Some(vec!["foo.ts".to_string(), "bar.ts".to_string()])
        );
    }

    #[test]
    fn test_parse_args_print_with_yaml_frontmatter() {
        let prompt = "---\ntitle: hello\n---\nSay hi.";
        let args = parse_args(&["-p".to_string(), prompt.to_string()]);
        assert!(args.print);
        assert_eq!(args.messages, vec![prompt]);
        assert!(args.unknown_flags.is_empty());
    }

    #[test]
    fn test_parse_args_print_does_not_consume_options() {
        let args = parse_args(&[
            "-p".to_string(),
            "--provider".to_string(),
            "openai".to_string(),
            "Say hi.".to_string(),
        ]);
        assert!(args.print);
        assert_eq!(args.provider, Some("openai".to_string()));
        assert_eq!(args.messages, vec!["Say hi."]);
    }

    #[test]
    fn test_parse_args_no_skills_short() {
        let args = parse_args(&["-ns".to_string()]);
        assert!(args.no_skills);
    }

    #[test]
    fn test_parse_args_no_prompt_templates_short() {
        let args = parse_args(&["-np".to_string()]);
        assert!(args.no_prompt_templates);
    }

    #[test]
    fn test_parse_args_mode_text() {
        let args = parse_args(&["--mode".to_string(), "text".to_string()]);
        assert_eq!(args.mode, Some(Mode::Text));
    }

    #[test]
    fn test_parse_args_models_filtered_empty_entries() {
        let args = parse_args(&["--models".to_string(), "sonnet,,haiku".to_string()]);
        assert_eq!(
            args.models,
            Some(vec![
                "sonnet".to_string(),
                "".to_string(),
                "haiku".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_args_unknown_short_flag_diagnostic() {
        let args = parse_args(&["-x".to_string()]);
        assert_eq!(args.diagnostics.len(), 1);
        assert_eq!(args.diagnostics[0].diag_type, DiagnosticType::Error);
        assert!(args.diagnostics[0].message.contains("Unknown option: -x"));
    }
}
