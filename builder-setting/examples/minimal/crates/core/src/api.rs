//! The one door into the core. Every function takes strings and returns a JSON
//! string: `{"ok": ...}` on success or `{"error": "..."}` with a message the
//! page can show as it is. The shim forwards these calls and nothing else.

use serde::Serialize;

use crate::counter::{self, Action, Settings, State};

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: String) -> String {
    serde_json::json!({ "error": message }).to_string()
}

fn settings(json: &str) -> Result<Settings, String> {
    let s: Settings =
        serde_json::from_str(json).map_err(|e| format!("The settings file could not be read: {e}"))?;
    counter::check_settings(&s)?;
    Ok(s)
}

#[derive(Serialize)]
struct Screen<'a> {
    settings: &'a Settings,
    state: State,
    view: counter::View,
}

/// Checks the settings and returns the first screen.
pub fn start(settings_json: &str) -> String {
    match settings(settings_json) {
        Ok(s) => {
            let state = counter::start();
            ok(&Screen {
                view: counter::view(&s, state),
                settings: &s,
                state,
            })
        }
        Err(e) => error(e),
    }
}

/// Applies `action` ("add", "remove" or "reset") to the state the page holds.
pub fn act(settings_json: &str, state_json: &str, action: &str) -> String {
    let run = || -> Result<String, String> {
        let s = settings(settings_json)?;
        let state: State =
            serde_json::from_str(state_json).map_err(|e| format!("The state could not be read: {e}"))?;
        let action: Action = serde_json::from_value(serde_json::Value::String(action.to_string()))
            .map_err(|_| format!("Unknown action \"{action}\"."))?;
        let next = counter::apply(&s, state, action);
        Ok(ok(&Screen {
            view: counter::view(&s, next),
            settings: &s,
            state: next,
        }))
    };
    run().unwrap_or_else(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS: &str =
        r#"{"title":"T","intro":"I","max":2,"step":1,"add_label":"+","remove_label":"-","reset_label":"0"}"#;

    #[test]
    fn start_returns_count_zero_with_remove_disabled() {
        let v: serde_json::Value = serde_json::from_str(&start(SETTINGS)).unwrap();
        assert_eq!(v["ok"]["view"]["count"], 0);
        assert_eq!(v["ok"]["view"]["can_remove"], false);
        assert_eq!(v["ok"]["view"]["can_add"], true);
    }

    #[test]
    fn act_adds_and_reports_the_stop() {
        let v: serde_json::Value = serde_json::from_str(&act(SETTINGS, r#"{"count":1}"#, "add")).unwrap();
        assert_eq!(v["ok"]["state"]["count"], 2);
        assert_eq!(v["ok"]["view"]["note"], "The counter stops at 2.");
    }

    #[test]
    fn an_unknown_action_is_an_error_message() {
        assert_eq!(
            act(SETTINGS, r#"{"count":1}"#, "double"),
            r#"{"error":"Unknown action \"double\"."}"#
        );
    }

    #[test]
    fn bad_settings_are_an_error_message() {
        let bad = SETTINGS.replace(r#""step":1"#, r#""step":0"#);
        assert_eq!(start(&bad), r#"{"error":"The step must be at least 1."}"#);
    }
}
