use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xtask::{PromptLexiconBuildConfig, build_prompt_lexicon, check_prompt_lexicon};

#[test]
fn builds_v1_lexicon_with_priority_aliases_and_sources() {
    let workspace = TestWorkspace::new("lexicon_build_priority");
    workspace.write_json_source(
        "people.json",
        r#"{
            "basic": {
                "Hero_Pose": "category primary",
                "fallback_only": null
            }
        }"#,
    );
    workspace.write_source_file(
        "weighted.csv",
        "hero_pose,80,weighted fallback\nfallback_only,50,weighted only\nfrom_source,7,weighted source\n",
    );
    workspace.write_source_file(
        "explicit.csv",
        "hero_pose,explicit primary\nfrom_source,source primary\n",
    );
    workspace.write_source_file(
        "aliases.csv",
        "tag,category,count,alias\nhero_pose,0,1,\"alias one,Hero Pose,None\"\nfallback_only,0,1,\"weighted only,alias two\"\nfrom_source,0,1,\"source alias\"\n",
    );
    workspace.write_manifest(
        r#"{
            "version": 1,
            "sources": [
                {
                    "id": "alias",
                    "path": "aliases.csv",
                    "parser": "alias_csv",
                    "priority": 30,
                    "alias_only": true,
                    "allow_primary": false
                },
                {
                    "id": "explicit",
                    "path": "explicit.csv",
                    "parser": "simple_csv",
                    "priority": 20,
                    "allow_primary": true
                },
                {
                    "id": "weighted",
                    "path": "weighted.csv",
                    "parser": "weighted_csv",
                    "priority": 10,
                    "allow_primary": true
                }
            ]
        }"#,
    );

    let summary = build_prompt_lexicon(&workspace.config()).unwrap();
    let output = workspace.read_output();

    assert_eq!(summary.total_tags, 3);
    assert_eq!(summary.categorized_tags, 2);
    assert_eq!(summary.uncategorized_tags, 1);
    assert_eq!(summary.matched_weights, 3);
    assert_eq!(summary.total_translations, 9);
    assert_eq!(summary.tags_with_aliases, 3);
    assert_eq!(output["schema"], "atelier-prompt-lexicon");
    assert_eq!(output["version"], 1);
    assert_eq!(output["sources"].as_array().unwrap().len(), 3);
    assert_eq!(output["stats"]["primary_from_category_json"], 1);
    assert_eq!(output["stats"]["primary_from_manifest_sources"], 2);
    assert_eq!(
        translations_for(&output, "hero_pose"),
        vec![
            "category primary",
            "alias one",
            "explicit primary",
            "weighted fallback",
        ]
    );
    assert_eq!(
        translations_for(&output, "fallback_only"),
        vec!["weighted only", "alias two",]
    );
    assert_eq!(
        translations_for(&output, "from_source"),
        vec!["source primary", "source alias", "weighted source",]
    );
}

#[test]
fn supports_reversed_and_github_csv_rows_with_tag_fallback() {
    let workspace = TestWorkspace::new("lexicon_build_parsers");
    workspace.write_source_file("weighted.csv", "stage_light,100,\nno_translation,5,\n");
    workspace.write_source_file("reversed.csv", "Character Name,character_tag\n");
    workspace.write_source_file(
        "github.csv",
        "danbooru_text,danbooru_url,tag,danbooru_translation\n\"wiki\",\"https://example.test\",\"stage_light\",\"Light Label,Stage Alias,stage light,None\"\n",
    );
    workspace.write_manifest(
        r#"{
            "version": 1,
            "sources": [
                {
                    "id": "reversed",
                    "path": "reversed.csv",
                    "parser": "reversed_csv",
                    "priority": 50,
                    "allow_primary": true
                },
                {
                    "id": "github",
                    "path": "github.csv",
                    "parser": "github_csv",
                    "priority": 40,
                    "allow_primary": true
                },
                {
                    "id": "weighted",
                    "path": "weighted.csv",
                    "parser": "weighted_csv",
                    "priority": 10,
                    "allow_primary": true
                }
            ]
        }"#,
    );

    build_prompt_lexicon(&workspace.config()).unwrap();
    let output = workspace.read_output();

    assert_eq!(
        translations_for(&output, "character_tag"),
        vec!["Character Name"]
    );
    assert_eq!(
        translations_for(&output, "stage_light"),
        vec!["Light Label", "Stage Alias",]
    );
    assert_eq!(
        translations_for(&output, "no_translation"),
        vec!["no_translation"]
    );
    assert_eq!(output["stats"]["primary_from_manifest_sources"], 2);
    assert_eq!(output["stats"]["primary_fallback_to_tag"], 1);
}

#[test]
fn category_order_file_controls_catalog_and_subcategory_order() {
    let workspace = TestWorkspace::new("lexicon_category_order");
    workspace.write_json_source(
        "background.json",
        r#"{
            "z_sub": {
                "sky": "sky label"
            },
            "a_sub": {
                "room": "room label"
            }
        }"#,
    );
    workspace.write_json_source(
        "characters.json",
        r#"{
            "pose": {
                "hero_pose": "hero label"
            }
        }"#,
    );
    workspace.write_category_order(
        r#"{
            "version": 1,
            "categories": [
                {
                    "name": "characters",
                    "subcategories": ["pose"]
                },
                {
                    "name": "background",
                    "subcategories": ["z_sub", "a_sub"]
                }
            ]
        }"#,
    );
    workspace.write_manifest(
        r#"{
            "version": 1,
            "sources": []
        }"#,
    );

    build_prompt_lexicon(&workspace.config()).unwrap();
    let output = workspace.read_output();

    let categories = output["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|category| category["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    let subcategories = output["subcategories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|subcategory| subcategory["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(categories, ["characters", "background"]);
    assert_eq!(subcategories, ["pose", "z_sub", "a_sub"]);
}

#[test]
fn parsers_drop_truncated_csv_rows() {
    let workspace = TestWorkspace::new("lexicon_truncated_rows");
    workspace.write_source_file("weighted.csv", "weighted_ok,10,Weighted OK\nweighted_bad\n");
    workspace.write_source_file("simple.csv", "simple_ok,Simple OK\nsimple_bad\n");
    workspace.write_source_file(
        "github.csv",
        "tag,danbooru_translation\ngithub_ok,\"GitHub Label,GitHub Alias\"\ngithub_bad_only_one_column\n",
    );
    workspace.write_source_file(
        "alias.csv",
        "tag,category,count,alias\nalias_ok,0,1,Alias OK\nalias_bad,0\n",
    );
    workspace.write_manifest(
        r#"{
            "version": 1,
            "sources": [
                {
                    "id": "simple",
                    "path": "simple.csv",
                    "parser": "simple_csv",
                    "priority": 40,
                    "allow_primary": true
                },
                {
                    "id": "github",
                    "path": "github.csv",
                    "parser": "github_csv",
                    "priority": 30,
                    "allow_primary": true
                },
                {
                    "id": "alias",
                    "path": "alias.csv",
                    "parser": "alias_csv",
                    "priority": 20,
                    "alias_only": true,
                    "allow_primary": false
                },
                {
                    "id": "weighted",
                    "path": "weighted.csv",
                    "parser": "weighted_csv",
                    "priority": 10,
                    "allow_primary": true
                }
            ]
        }"#,
    );

    build_prompt_lexicon(&workspace.config()).unwrap();
    let output = workspace.read_output();

    assert!(has_tag(&output, "weighted_ok"));
    assert!(has_tag(&output, "simple_ok"));
    assert_eq!(
        translations_for(&output, "github_ok"),
        vec!["GitHub Label", "GitHub Alias"]
    );
    assert!(!has_tag(&output, "weighted_bad"));
    assert!(!has_tag(&output, "simple_bad"));
    assert!(!has_tag(&output, "github_bad_only_one_column"));
    assert!(!has_tag(&output, "alias_bad"));
}

#[test]
fn check_detects_stale_generated_lexicon() {
    let workspace = TestWorkspace::new("lexicon_check_stale");
    workspace.write_source_file("weighted.csv", "hero_pose,80,hero\n");
    workspace.write_manifest(
        r#"{
            "version": 1,
            "sources": [
                {
                    "id": "weighted",
                    "path": "weighted.csv",
                    "parser": "weighted_csv",
                    "priority": 10,
                    "allow_primary": true
                }
            ]
        }"#,
    );
    build_prompt_lexicon(&workspace.config()).unwrap();

    assert!(check_prompt_lexicon(&workspace.config()).is_ok());

    workspace.write_file("assets/prompt-lexicon/generated/lexicon.json", "{}\n");
    let error = check_prompt_lexicon(&workspace.config()).unwrap_err();

    assert!(error.contains("generated prompt lexicon is stale"));
}

fn translations_for(output: &Value, tag: &str) -> Vec<String> {
    let item = output["tags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["tag"] == tag)
        .unwrap();
    let start = usize::try_from(item["translation_start"].as_u64().unwrap()).unwrap();
    let count = usize::try_from(item["translation_count"].as_u64().unwrap()).unwrap();
    output["translations"].as_array().unwrap()[start..start + count]
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect()
}

fn has_tag(output: &Value, tag: &str) -> bool {
    output["tags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["tag"] == tag)
}

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("atelier_xtask_{name}_{unique}"));
        fs::create_dir_all(path.join("assets/prompt-lexicon/sources/json"))
            .expect("test workspace should be created");
        fs::create_dir_all(path.join("assets/prompt-lexicon/generated"))
            .expect("test generated directory should be created");
        Self { path }
    }

    fn config(&self) -> PromptLexiconBuildConfig {
        PromptLexiconBuildConfig::default_for_workspace(&self.path)
    }

    fn write_manifest(&self, contents: &str) {
        self.write_file(
            "assets/prompt-lexicon/sources/translation-sources.json",
            contents,
        );
    }

    fn write_json_source(&self, name: &str, contents: &str) {
        self.write_file(
            format!("assets/prompt-lexicon/sources/json/{name}").as_str(),
            contents,
        );
    }

    fn write_category_order(&self, contents: &str) {
        self.write_file(
            "assets/prompt-lexicon/sources/category-order.json",
            contents,
        );
    }

    fn write_source_file(&self, name: &str, contents: &str) {
        self.write_file(
            format!("assets/prompt-lexicon/sources/{name}").as_str(),
            contents,
        );
    }

    fn read_output(&self) -> Value {
        let output = fs::read_to_string(self.config().output_file)
            .expect("generated lexicon should be readable");
        serde_json::from_str(&output).expect("generated lexicon should be valid JSON")
    }

    fn write_file(&self, relative_path: &str, contents: &str) {
        let path = self.path.join(relative_path);
        fs::create_dir_all(path.parent().expect("test file should have a parent"))
            .expect("test file parent should be created");
        fs::write(path, contents).expect("test file should be written");
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("test workspace should be removed");
    }
}
