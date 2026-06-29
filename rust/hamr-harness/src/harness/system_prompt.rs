//! Port of `packages/agent/src/harness/system-prompt.ts`.

use crate::harness::types::Skill;

pub fn format_skills_for_system_prompt(skills: &[Skill]) -> String {
    let visible_skills: Vec<&Skill> = skills
        .iter()
        .filter(|skill| !skill.disable_model_invocation)
        .collect();
    if visible_skills.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        "The following skills provide specialized instructions for specific tasks.".to_string(),
        "Read the full skill file when the task matches its description.".to_string(),
        "When a skill file references a relative path, resolve it against the skill directory (parent of SKILL.md / dirname of the path) and use that absolute path in tool commands.".to_string(),
        String::new(),
        "<available_skills>".to_string(),
    ];

    for skill in visible_skills {
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

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::format_skills_for_system_prompt;
    use crate::harness::types::Skill;

    #[test]
    fn formats_visible_skills_and_escapes_xml() {
        let output = format_skills_for_system_prompt(&[
            Skill {
                name: "visible".to_string(),
                description: "Use <this> & that".to_string(),
                content: "body".to_string(),
                file_path: "/skills/visible/SKILL.md".to_string(),
                disable_model_invocation: false,
            },
            Skill {
                name: "hidden".to_string(),
                description: "Hidden".to_string(),
                content: "body".to_string(),
                file_path: "/skills/hidden/SKILL.md".to_string(),
                disable_model_invocation: true,
            },
        ]);

        assert!(output.contains("<name>visible</name>"));
        assert!(output.contains("&lt;this&gt; &amp; that"));
        assert!(!output.contains("hidden"));
    }

    #[test]
    fn returns_empty_string_when_no_skills_are_visible() {
        let output = format_skills_for_system_prompt(&[Skill {
            name: "hidden".to_string(),
            description: "Hidden".to_string(),
            content: "body".to_string(),
            file_path: "/skills/hidden/SKILL.md".to_string(),
            disable_model_invocation: true,
        }]);

        assert!(output.is_empty());
    }
}
