//! PowerToys 설정 파일 읽기(읽기만). `settings.json`의 `enabled`에서 켜진 모듈만 보고,
//! 각 모듈 `settings.json` 안의 단축키 객체 `{win, ctrl, alt, shift, code}`를 모두 찾는다(모듈마다 키 이름이 달라 일반 탐색).

use ink_domain::shortcuts::{combo_from_vk, LiveBinding};
use serde_json::Value;
use std::path::Path;

/// 모듈 폴더 이름 → 화면에 쓰는 이름.
pub fn module_label(module: &str) -> String {
    let ko = match module {
        "AlwaysOnTop" => "항상 위",
        "ColorPicker" => "색 선택",
        "CropAndLock" => "자르기·잠그기",
        "Measure Tool" => "화면 자",
        "TextExtractor" => "텍스트 추출",
        "AdvancedPaste" => "고급 붙여넣기",
        "Workspaces" => "작업 공간",
        "CmdPal" => "명령 팔레트",
        "Shortcut Guide" => "단축키 안내",
        "MouseJump" => "마우스 점프",
        "MouseHighlighter" => "마우스 강조",
        "MousePointerCrosshairs" => "마우스 십자선",
        _ => module,
    };
    format!("PowerToys {ko}")
}

/// 단축키처럼 생긴 객체를 모두 찾는다. code 0(비움)은 건너뛴다.
fn collect(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            let is_hotkey = map.get("code").and_then(Value::as_u64).is_some() && ["win", "ctrl", "alt", "shift"].iter().any(|k| map.get(*k).is_some_and(Value::is_boolean));
            if is_hotkey {
                let b = |k: &str| map.get(k).and_then(Value::as_bool).unwrap_or(false);
                let code = map["code"].as_u64().unwrap_or(0) as u32;
                if code != 0 {
                    if let Some(c) = combo_from_vk(b("ctrl"), b("alt"), b("shift"), b("win"), code) {
                        out.push(c);
                    }
                }
                return;
            }
            for child in map.values() {
                collect(child, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| collect(i, out)),
        _ => {}
    }
}

/// 순수 함수: 일반 설정 + (모듈 이름, 모듈 설정) → 지금 잡혀 있는 키.
pub fn bindings_from(general: &Value, modules: &[(String, Value)]) -> Vec<LiveBinding> {
    let enabled = |m: &str| general.get("enabled").and_then(|e| e.get(m)).and_then(Value::as_bool).unwrap_or(false);
    let mut out = Vec::new();
    for (name, settings) in modules {
        if !enabled(name) {
            continue;
        }
        let mut combos = Vec::new();
        collect(settings, &mut combos);
        combos.sort();
        combos.dedup();
        for combo in combos {
            out.push(LiveBinding { owner: module_label(name), combo, what: "단축키".into() });
        }
    }
    out
}

fn read_json(path: &Path) -> Option<Value> {
    let bytes = std::fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    serde_json::from_slice(bytes).ok()
}

/// 설정 폴더(`%LOCALAPPDATA%\Microsoft\PowerToys`)에서 읽는다. 일반 설정을 못 읽으면 None.
pub fn read_bindings(dir: &Path) -> Option<Vec<LiveBinding>> {
    let general = read_json(&dir.join("settings.json"))?;
    let names: Vec<String> = general.get("enabled")?.as_object()?.iter().filter(|(_, v)| v.as_bool() == Some(true)).map(|(k, _)| k.clone()).collect();
    let modules: Vec<(String, Value)> = names.into_iter().filter_map(|n| read_json(&dir.join(&n).join("settings.json")).map(|v| (n, v))).collect();
    Some(bindings_from(&general, &modules))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn 켜진_모듈의_단축키만_읽는다() {
        let general = json!({ "enabled": { "AlwaysOnTop": true, "ColorPicker": false, "PowerToys Run": true } });
        let aot = json!({ "properties": { "hotkey": { "value": { "win": false, "ctrl": false, "alt": true, "shift": true, "code": 68, "key": "" } }, "frame-thickness": { "value": 15 } } });
        let picker = json!({ "properties": { "ActivationShortcut": { "win": true, "ctrl": false, "alt": false, "shift": true, "code": 67 } } });
        let run = json!({ "properties": { "open_powerlauncher": { "win": false, "ctrl": false, "alt": true, "shift": false, "code": 32 }, "empty": { "win": false, "ctrl": false, "alt": false, "shift": false, "code": 0 } } });
        let modules = vec![("AlwaysOnTop".to_string(), aot), ("ColorPicker".to_string(), picker), ("PowerToys Run".to_string(), run)];
        let b = bindings_from(&general, &modules);
        assert_eq!(b.len(), 2, "{b:?}");
        assert_eq!(b[0], LiveBinding { owner: "PowerToys 항상 위".into(), combo: "Alt+Shift+D".into(), what: "단축키".into() });
        assert_eq!(b[1].combo, "Alt+Space");
    }

    #[test]
    fn 이_pc의_powertoys_설정을_읽어도_넘어지지_않는다() {
        if let Some(d) = std::env::var_os("LOCALAPPDATA") {
            let _ = read_bindings(&Path::new(&d).join("Microsoft").join("PowerToys"));
        }
    }
}
