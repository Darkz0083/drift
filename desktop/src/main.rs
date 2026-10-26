// Prevents an extra console window on Windows in release.
#![cfg_attr(
    any(not(debug_assertions), feature = "webext-qa"),
    windows_subsystem = "windows"
)]

fn main() -> std::process::ExitCode {
    #[cfg(all(target_os = "macos", feature = "work-product"))]
    drift_app::work_commands::private_guard_main();
    #[cfg(all(target_os = "macos", feature = "work-integration-qa", debug_assertions))]
    if let Some((_, stage)) = [
        (
            "--probe-interactive-safari-government",
            drift_engine::LivenessStage::Safari,
        ),
        (
            "--probe-interactive-scripts-government",
            drift_engine::LivenessStage::BrowseScripts,
        ),
        (
            "--probe-interactive-gate-government",
            drift_engine::LivenessStage::DocumentGate,
        ),
    ]
    .into_iter()
    .find(|(flag, _)| std::env::args_os().skip(1).eq([*flag]))
    {
        return if drift_engine::run_interactive_liveness_probe(
            drift_engine::LivenessSite::Government,
            stage,
        )
        .is_ok()
        {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    #[cfg(all(target_os = "macos", feature = "work-integration-qa", debug_assertions))]
    if std::env::args_os()
        .skip(1)
        .eq(["--probe-interactive-government"])
    {
        return if drift_engine::run_interactive_government_probe().is_ok() {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    #[cfg(all(target_os = "macos", feature = "work-integration-qa", debug_assertions))]
    if let Some((_, stage)) = [
        (
            "--probe-bare-government",
            drift_engine::LivenessStage::Bare,
        ),
        (
            "--probe-safari-government",
            drift_engine::LivenessStage::Safari,
        ),
        (
            "--probe-scripts-government",
            drift_engine::LivenessStage::BrowseScripts,
        ),
        (
            "--probe-gate-government",
            drift_engine::LivenessStage::DocumentGate,
        ),
    ]
    .into_iter()
    .find(|(flag, _)| std::env::args_os().skip(1).eq([*flag]))
    {
        return if drift_engine::run_liveness_probe(
            drift_engine::LivenessSite::Government,
            stage,
        )
        .is_ok()
        {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    #[cfg(all(target_os = "macos", feature = "work-integration-qa", debug_assertions))]
    if std::env::args_os()
        .skip(1)
        .eq(["--check-provider-keychain"])
    {
        use std::io::Write as _;
        let started = std::time::Instant::now();
        let available = drift_agentic::load_macos_development_openai_credential().is_ok();
        let _ = writeln!(
            std::io::stderr(),
            "keychain_check available={available} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        return if available {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    drift_desktop_lib::run();
    std::process::ExitCode::SUCCESS
}
