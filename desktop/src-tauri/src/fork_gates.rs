//! Fork configuration gates.
//!
//! The Lunar Park fork disables some upstream boot-time behaviors by
//! default; each gate here re-enables one via an explicit env opt-in. The
//! upstream code paths stay intact — gates only control whether they run.

/// Whether boot-time voice (STT/TTS) model downloads are enabled.
///
/// Fork default is disabled; only an explicit `1` or `true` (from
/// `BUZZ_VOICE_MODEL_AUTODOWNLOAD`) restores the upstream fetch-at-boot
/// behavior. Models still download on-demand when transcription is used.
pub fn voice_model_autodownload_enabled(value: Option<&str>) -> bool {
    matches!(value.map(str::trim), Some("1") | Some("true"))
}

#[cfg(test)]
mod voice_model_gate_tests {
    use super::voice_model_autodownload_enabled;

    #[test]
    fn autodownload_is_disabled_by_default() {
        assert!(!voice_model_autodownload_enabled(None));
        assert!(!voice_model_autodownload_enabled(Some("")));
        assert!(!voice_model_autodownload_enabled(Some("0")));
        assert!(!voice_model_autodownload_enabled(Some("false")));
    }

    #[test]
    fn autodownload_enables_on_explicit_opt_in() {
        assert!(voice_model_autodownload_enabled(Some("1")));
        assert!(voice_model_autodownload_enabled(Some("true")));
        assert!(voice_model_autodownload_enabled(Some(" 1 ")));
    }
}
