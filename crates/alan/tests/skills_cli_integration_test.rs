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
[[skill_overrides]]
skill = "release-checklist"
allow_implicit_invocation = false
"#,
    )
    .unwrap();

    std::fs::write(
        workspace_agent_root.join("agent.toml"),
        r#"
[[skill_overrides]]
skill = "tool-heavy"
enabled = true
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
    assert!(stdout.contains("[repo] skill:release-checklist"));
    assert!(stdout.contains("[repo] skill:tool-heavy"));
    assert!(stdout.contains("exports: child_agents=1, resources=scripts"));
    assert!(stdout.contains("skills: $release-checklist [implicit: false]"));
    assert!(stdout.contains(
        "skills: $tool-heavy [delegate: reviewer] [unavailable: missing dependencies: tool:missing_tool]"
    ));
}

#[test]
fn skills_init_inline_scaffolds_and_validates_package() {
    let temp = TempDir::new().unwrap();
    let package_root = temp.path().join("doc-review");

    let init = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args([
            "skills",
            "init",
            package_root.to_str().unwrap(),
            "--template",
            "inline",
            "--name",
            "Doc Review",
            "--description",
            "Review documentation when asked.",
            "--short-description",
            "Review documentation",
        ])
        .output()
        .unwrap();

    assert!(init.status.success(), "{init:?}");
    assert!(package_root.join("SKILL.md").is_file());
    assert!(package_root.join("agents/openai.yaml").is_file());

    let validate = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args(["skills", "validate", package_root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(validate.status.success(), "{validate:?}");
    let stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(stdout.contains("status: valid"));
    assert!(stdout.contains("execution: inline(no_child_agent_exports)"));
}

#[test]
fn skills_init_delegate_scaffolds_a_delegated_package() {
    let temp = TempDir::new().unwrap();
    let package_root = temp.path().join("repo-review");

    let init = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args([
            "skills",
            "init",
            package_root.to_str().unwrap(),
            "--template",
            "delegate",
            "--name",
            "Repo Review",
            "--description",
            "Review repositories when asked.",
            "--short-description",
            "Review repositories",
        ])
        .output()
        .unwrap();

    assert!(init.status.success(), "{init:?}");
    assert!(package_root.join("agents/openai.yaml").is_file());
    assert!(
        package_root
            .join("agents/repo-review/persona/ROLE.md")
            .is_file()
    );

    let validate = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args(["skills", "validate", package_root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(validate.status.success(), "{validate:?}");
    let stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(stdout.contains(
        "execution: delegate(target=repo-review, source=same_name_skill_and_child_agent)"
    ));
}

#[test]
fn skills_validate_fails_for_ambiguous_delegate_shape() {
    let temp = TempDir::new().unwrap();
    let package_root = temp.path().join("skill-creator");
    write_skill(
        temp.path(),
        "skill-creator",
        r#"---
name: Skill Creator
description: Create skills
---

Body
"#,
    );
    std::fs::create_dir_all(package_root.join("agents/creator/persona")).unwrap();
    std::fs::create_dir_all(package_root.join("agents/grader/persona")).unwrap();

    let validate = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args(["skills", "validate", package_root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!validate.status.success(), "{validate:?}");
    let stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(stdout.contains("status: invalid"));
    assert!(stdout.contains("execution: unresolved(ambiguous_package_shape)"));
}

#[test]
fn skills_eval_runs_package_local_hook() {
    let temp = TempDir::new().unwrap();
    let package_root = temp.path().join("lint-check");
    write_skill(
        temp.path(),
        "lint-check",
        r#"---
name: Lint Check
description: Run lint checks
---

Body
"#,
    );
    std::fs::create_dir_all(package_root.join("scripts")).unwrap();
    std::fs::write(
        package_root.join("scripts/eval.sh"),
        "#!/bin/sh\necho \"eval hook ok\"\n",
    )
    .unwrap();

    let eval = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args(["skills", "eval", package_root.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(eval.status.success(), "{eval:?}");
    let stdout = String::from_utf8_lossy(&eval.stdout);
    assert!(stdout.contains("status: passed"));
    assert!(stdout.contains("eval hook ok"));
}

#[test]
fn skills_eval_runs_structured_manifest_and_writes_artifacts() {
    let temp = TempDir::new().unwrap();
    let package_root = temp.path().join("skill-creator-eval");
    let output_dir = temp.path().join("eval-output");
    write_skill(
        temp.path(),
        "skill-creator-eval",
        r#"---
name: Skill Creator Eval
description: Evaluate skill packages
---

Body
"#,
    );
    std::fs::create_dir_all(package_root.join("scripts")).unwrap();
    std::fs::create_dir_all(package_root.join("evals")).unwrap();
    std::fs::create_dir_all(package_root.join("eval-viewer")).unwrap();
    std::fs::create_dir_all(package_root.join("agents")).unwrap();
    std::fs::write(
        package_root.join("scripts/candidate.sh"),
        "#!/bin/sh\nprintf '{\"passed\":true,\"variant\":\"candidate\"}'\n",
    )
    .unwrap();
    std::fs::write(
        package_root.join("scripts/baseline.sh"),
        "#!/bin/sh\nprintf '{\"passed\":false,\"variant\":\"baseline\"}'\n",
    )
    .unwrap();
    std::fs::write(
        package_root.join("scripts/grader.sh"),
        "#!/bin/sh\nprintf '{\"passed\":true,\"score\":1}'\n",
    )
    .unwrap();
    std::fs::write(
        package_root.join("scripts/analyzer.sh"),
        "#!/bin/sh\nprintf '{\"passed\":true,\"notes\":[\"candidate kept the skill-specific workflow\"]}'\n",
    )
    .unwrap();
    std::fs::write(
        package_root.join("scripts/comparator.sh"),
        "#!/bin/sh\nprintf '{\"passed\":true,\"delta\":\"candidate is more explicit than baseline\"}'\n",
    )
    .unwrap();
    std::fs::write(package_root.join("agents/grader.md"), "# grader\n").unwrap();
    std::fs::write(package_root.join("agents/analyzer.md"), "# analyzer\n").unwrap();
    std::fs::write(package_root.join("agents/comparator.md"), "# comparator\n").unwrap();
    std::fs::write(
        package_root.join("eval-viewer/viewer.html"),
        "<!doctype html><title>viewer</title>",
    )
    .unwrap();
    std::fs::write(
        package_root.join("evals/evals.json"),
        r#"{
  "version": 1,
  "suite": "skill-creator-eval",
  "review": {"viewer": "eval-viewer"},
  "cases": [
    {
      "id": "trigger-create",
      "type": "trigger",
      "input": "please use $skill-creator-eval to create skill package",
      "expected": true
    },
    {
      "id": "compare-guidance",
      "type": "command",
      "prompt": "Compare candidate and baseline outputs",
      "command": ["sh", "scripts/candidate.sh"],
      "comparison": {
        "mode": "with_without_skill",
        "baseline_command": ["sh", "scripts/baseline.sh"]
      },
      "grading": {
        "command": ["sh", "scripts/grader.sh"],
        "prompt_file": "agents/grader.md"
      },
      "analyzer": {
        "command": ["sh", "scripts/analyzer.sh"],
        "prompt_file": "agents/analyzer.md"
      },
      "comparator": {
        "command": ["sh", "scripts/comparator.sh"],
        "prompt_file": "agents/comparator.md"
      }
    }
  ]
}"#,
    )
    .unwrap();

    let eval = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args([
            "skills",
            "eval",
            package_root.to_str().unwrap(),
            "--output-dir",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(eval.status.success(), "{eval:?}");
    let stdout = String::from_utf8_lossy(&eval.stdout);
    assert!(stdout.contains("status: passed"));
    assert!(stdout.contains("manifest:"));
    assert!(stdout.contains("cases: 2 total, 2 passed, 0 failed"));
    assert!(stdout.contains("compare-guidance [command] [passed]"));
    assert!(output_dir.join("run.json").is_file());
    assert!(output_dir.join("benchmark.json").is_file());
    assert!(output_dir.join("review/index.html").is_file());
    assert!(output_dir.join("review/viewer/viewer.html").is_file());
    assert!(output_dir.join("cases/trigger-create/case.json").is_file());
    assert!(
        output_dir
            .join("cases/compare-guidance/with_skill.json")
            .is_file()
    );
    assert!(
        output_dir
            .join("cases/compare-guidance/without_skill.json")
            .is_file()
    );
    assert!(
        output_dir
            .join("cases/compare-guidance/grading.json")
            .is_file()
    );
    assert!(
        output_dir
            .join("cases/compare-guidance/analyzer.json")
            .is_file()
    );
    assert!(
        output_dir
            .join("cases/compare-guidance/comparator.json")
            .is_file()
    );
}
