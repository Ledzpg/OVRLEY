//! Every font a bundled template names must come with the app.
//!
//! A bare family name such as "Bebas Neue" is looked up among the user's system fonts, so a
//! template that names one fails to load on a PC without it ("font family is unavailable").
//! Bundled fonts are addressed by "<Family>.<extension>" ids from the `fonts` folder.

mod common;

use ovrley_core::fonts::font_catalog;
use serde_json::Value;
use std::collections::BTreeSet;

fn collect_font_ids(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let names_a_font = key.contains("font") && !key.contains("size") && !key.contains("weight");
                match child {
                    Value::String(id) if names_a_font => {
                        found.insert(id.clone());
                    }
                    _ => collect_font_ids(child, found),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect_font_ids(item, found)),
        _ => {}
    }
}

#[test]
fn bundled_templates_only_use_bundled_fonts() {
    let root = common::test_config::repo_git_root();
    let catalog = font_catalog(&[root.join("fonts")]).unwrap();
    let bundled: BTreeSet<_> = catalog.recommended_fonts.iter().map(|font| font.id.clone()).collect();
    assert!(bundled.contains("Bebas Neue.ttf"), "Bebas Neue ships in fonts/, got {bundled:?}");

    let mut checked = 0;
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(root.join("templates")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let template: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let mut ids = BTreeSet::new();
        collect_font_ids(&template, &mut ids);
        for id in ids.difference(&bundled) {
            offenders.push(format!("{} uses {id:?}", path.file_name().unwrap().to_string_lossy()));
        }
        checked += 1;
    }
    assert!(offenders.is_empty(), "templates naming fonts that are not bundled: {offenders:?}");
    assert!(checked >= 7, "expected the bundled templates, found {checked}");
}
