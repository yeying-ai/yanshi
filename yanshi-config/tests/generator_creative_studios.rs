//! Validation and graph-construction tests for multi-paradigm Creative Studio
//! generators and their companion `SKILL.md` bundles.

use std::path::PathBuf;

use yanshi_config::config::schema::{
  UnknownParameterMode, WorkflowValidationOptions, validate_flow_definition_with_options,
};
use yanshi_config::executor::{build_flow_from_definition, parse_workflow_definition};
use yanshi_skills::SkillLoader;

fn workspace_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .expect("workspace root")
    .to_path_buf()
}

fn assert_strict_valid_and_builds(generator_file: &str) {
  let root = workspace_root();
  let prev = std::env::current_dir().expect("cwd");
  std::env::set_current_dir(&root).expect("set workspace root");
  let yaml_path = root.join("examples/generators").join(generator_file);
  let raw = std::fs::read_to_string(&yaml_path)
    .unwrap_or_else(|e| panic!("failed to read {}: {e}", yaml_path.display()));
  let def = parse_workflow_definition(&raw)
    .unwrap_or_else(|e| panic!("failed to parse {generator_file}: {e}"));
  let report = validate_flow_definition_with_options(
    &def,
    WorkflowValidationOptions {
      unknown_parameters: UnknownParameterMode::Error,
    },
  );
  assert!(
    report.issues.is_empty(),
    "{generator_file} strict issues: {:?}",
    report.issues
  );
  assert!(
    report.warnings.is_empty(),
    "{generator_file} strict warnings: {:?}",
    report.warnings
  );
  build_flow_from_definition(&def, Some("mock-model"))
    .unwrap_or_else(|e| panic!("failed to build Flow for {generator_file}: {e}"));
  let _ = std::env::set_current_dir(prev);
}

fn assert_skill_valid(rel_dir: &str) {
  let dir = workspace_root().join(rel_dir);
  let manifest =
    SkillLoader::load(&dir).unwrap_or_else(|e| panic!("failed to load skill {rel_dir}: {e}"));
  let warnings = SkillLoader::validate(&manifest, &dir)
    .unwrap_or_else(|e| panic!("skill validation failed for {rel_dir}: {e}"));
  assert!(
    warnings.is_empty(),
    "unexpected skill warnings in {rel_dir}: {warnings:?}"
  );
}

#[test]
fn episodic_drama_studio_and_crew_skills_validate_strictly() {
  for skill in [
    "examples/skills/drama-crew/showrunner",
    "examples/skills/drama-crew/casting-director",
    "examples/skills/drama-crew/episode-writer",
  ] {
    assert_skill_valid(skill);
  }
  assert_strict_valid_and_builds("episodic_drama_studio.yml");
}

#[test]
fn debate_podcast_show_and_panel_skills_validate_strictly() {
  for skill in [
    "examples/skills/debate-panel/pro-advocate",
    "examples/skills/debate-panel/con-challenger",
    "examples/skills/debate-panel/roundtable-moderator",
  ] {
    assert_skill_valid(skill);
  }
  assert_strict_valid_and_builds("debate_podcast_show.yml");
}
