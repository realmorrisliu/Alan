use alan_skill_tools::{SkillEvalRunOptions, run_eval_manifest};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn shared_skill_tooling_binary_regenerates_benchmark_and_review_bundle() {
    let temp = TempDir::new().unwrap();
    let package_root = temp.path().join("skill-package");
    let output_dir = temp.path().join("eval-run");

    std::fs::create_dir_all(package_root.join("evals")).unwrap();
    std::fs::write(
        package_root.join("SKILL.md"),
        r#"---
name: skill-package
description: Evaluate a skill package
---

Body
"#,
    )
    .unwrap();
    std::fs::write(
        package_root.join("evals/evals.json"),
        r#"{
  "version": 1,
  "cases": [
    {"id": "trigger", "type": "trigger", "input": "use $skill-package", "expected": true}
  ]
}"#,
    )
    .unwrap();

    run_eval_manifest(&SkillEvalRunOptions {
        package_root: package_root.clone(),
        manifest_path: package_root.join("evals/evals.json"),
        output_dir: Some(output_dir.clone()),
    })
    .unwrap();

    std::fs::remove_file(output_dir.join("benchmark.json")).unwrap();
    std::fs::remove_file(output_dir.join("review/index.html")).unwrap();

    let aggregate = Command::new(env!("CARGO_BIN_EXE_alan-skill-tools"))
        .args(["aggregate-benchmark", output_dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(aggregate.status.success(), "{aggregate:?}");
    assert!(output_dir.join("benchmark.json").is_file());

    let review = Command::new(env!("CARGO_BIN_EXE_alan-skill-tools"))
        .args(["generate-review", output_dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(review.status.success(), "{review:?}");
    assert!(output_dir.join("review/index.html").is_file());
    let review_html = std::fs::read_to_string(output_dir.join("review/index.html")).unwrap();
    assert!(review_html.contains("../benchmark.json"));
    assert!(review_html.contains("../run.json"));
    assert!(review_html.contains("../cases/trigger/case.json"));
}
