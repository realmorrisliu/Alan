use std::process::Command;

use tempfile::TempDir;

fn write_skill(root: &std::path::Path, name: &str, body: &str) {
    let skill_dir = root.join(name);
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), body).unwrap();
}

#[test]
fn skills_packages_reports_mounts_exports_and_unavailable_skills() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let workspace = temp.path().join("workspace");
    let global_agent_root = home.join(".alan/agent");
    let workspace_agent_root = workspace.join(".alan/agent");
    let workspace_skills_root = workspace_agent_root.join("skills");

    std::fs::create_dir_all(&global_agent_root).unwrap();
    std::fs::create_dir_all(&workspace_skills_root).unwrap();

    std::fs::write(
        global_agent_root.join("agent.toml"),
        r#"
[[package_mounts]]
package = "skill:release-checklist"
mode = "explicit_only"
"#,
    )
    .unwrap();

    std::fs::write(
        workspace_agent_root.join("agent.toml"),
        r#"
[[package_mounts]]
package = "skill:tool-heavy"
mode = "discoverable"
"#,
    )
    .unwrap();

    write_skill(
        &workspace_skills_root,
        "release-checklist",
        r#"---
name: Release Checklist
description: Release workflow
---

Body
"#,
    );

    write_skill(
        &workspace_skills_root,
        "tool-heavy",
        r#"---
name: Tool Heavy
description: Needs extra tools
capabilities:
  required_tools: ["missing_tool"]
---

Body
"#,
    );
    std::fs::create_dir_all(workspace_skills_root.join("tool-heavy/scripts")).unwrap();
    std::fs::create_dir_all(workspace_skills_root.join("tool-heavy/agents/reviewer")).unwrap();
    std::fs::write(
        workspace_skills_root.join("tool-heavy/agents/reviewer/agent.toml"),
        "openai_responses_model = \"gpt-5.4\"\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args([
            "skills",
            "packages",
            "--workspace",
            workspace.to_str().unwrap(),
        ])
        .env("HOME", &home)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[repo] skill:release-checklist (explicit_only)"));
    assert!(stdout.contains("[repo] skill:tool-heavy (discoverable)"));
    assert!(stdout.contains("exports: child_agents=1, resources=scripts"));
    assert!(stdout.contains(
        "skills: $tool-heavy [delegate: reviewer] [unavailable: missing dependencies: tool:missing_tool]"
    ));
}
