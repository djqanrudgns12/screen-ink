//! 단독 앱 = 플러그인 조립. 같은 플러그인을 Tidy에도 붙인다(03 §6).

use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

pub fn run() {
    // 진단할 때만 SCREEN_INK_LOG=debug — 우리 크레이트만 자세히(강조 사슬·V 시간 등)
    let ours = if std::env::var("SCREEN_INK_LOG").as_deref() == Ok("debug") { log::LevelFilter::Debug } else { log::LevelFilter::Info };
    let builder = tauri::Builder::default()
        // 두 번째 실행은 새로 뜨지 않고 설정 창을 앞으로(05 §11-1)
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| tauri_plugin_screen_ink::open_settings(app)))
        // 로그: 사건마다 한 줄, 창 제목·파일 이름·사용자 이름은 쓰지 않는다(05 §9). 파일당 2MB.
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([Target::new(TargetKind::LogDir { file_name: Some("screen-ink".into()) }), Target::new(TargetKind::Stdout)])
                .level(log::LevelFilter::Info)
                .level_for("ink_app", ours)
                .level_for("tauri_plugin_screen_ink", ours)
                .level_for("ink_platform_windows", ours)
                .level_for("ink_platform_macos", ours)
                .max_file_size(2_000_000)
                .rotation_strategy(RotationStrategy::KeepOne)
                .build(),
        );
    tauri_plugin_screen_ink::install(builder)
        .run(tauri::generate_context!())
        .expect("screen-ink를 시작하지 못했어요");
}
