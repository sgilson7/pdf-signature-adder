//! The counter's rules. The page draws `View` and decides nothing: whether a
//! button is enabled and what note is shown are both decided here.

use serde::{Deserialize, Serialize};

/// The counter's settings and every label the page shows, read from data.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub title: String,
    pub intro: String,
    pub max: u32,
    pub step: u32,
    pub add_label: String,
    pub remove_label: String,
    pub reset_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct State {
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Add,
    Remove,
    Reset,
}

/// What the page draws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct View {
    pub count: u32,
    pub can_add: bool,
    pub can_remove: bool,
    pub note: String,
}

/// Settings are usable when the step is at least one and fits under the maximum.
pub fn check_settings(s: &Settings) -> Result<(), String> {
    if s.step == 0 {
        return Err("The step must be at least 1.".into());
    }
    if s.step > s.max {
        return Err(format!(
            "The step ({}) is larger than the maximum ({}).",
            s.step, s.max
        ));
    }
    Ok(())
}

pub fn start() -> State {
    State { count: 0 }
}

/// Adding past the maximum stops at the maximum; removing below zero stops at zero.
pub fn apply(s: &Settings, state: State, action: Action) -> State {
    let count = match action {
        Action::Add => (state.count + s.step).min(s.max),
        Action::Remove => state.count.saturating_sub(s.step),
        Action::Reset => 0,
    };
    State { count }
}

pub fn view(s: &Settings, state: State) -> View {
    let note = if state.count == s.max {
        format!("The counter stops at {}.", s.max)
    } else {
        String::new()
    };
    View {
        count: state.count,
        can_add: state.count < s.max,
        can_remove: state.count > 0,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(max: u32, step: u32) -> Settings {
        Settings {
            title: "T".into(),
            intro: "I".into(),
            max,
            step,
            add_label: "+".into(),
            remove_label: "-".into(),
            reset_label: "0".into(),
        }
    }

    // Expected values worked out by hand: max 10, step 3 goes 0, 3, 6, 9, 10.
    #[test]
    fn adding_stops_at_the_maximum() {
        let s = settings(10, 3);
        let mut st = start();
        let mut seen = vec![st.count];
        for _ in 0..5 {
            st = apply(&s, st, Action::Add);
            seen.push(st.count);
        }
        assert_eq!(seen, vec![0, 3, 6, 9, 10, 10]);
    }

    #[test]
    fn removing_stops_at_zero() {
        let s = settings(10, 3);
        let st = apply(&s, State { count: 2 }, Action::Remove);
        assert_eq!(st.count, 0);
        assert_eq!(apply(&s, st, Action::Remove).count, 0);
    }

    #[test]
    fn reset_returns_to_zero() {
        assert_eq!(
            apply(&settings(10, 3), State { count: 7 }, Action::Reset).count,
            0
        );
    }

    #[test]
    fn view_disables_add_only_at_the_maximum() {
        let s = settings(10, 3);
        assert!(view(&s, State { count: 9 }).can_add);
        let at_max = view(&s, State { count: 10 });
        assert!(!at_max.can_add);
        assert_eq!(at_max.note, "The counter stops at 10.");
    }

    #[test]
    fn view_disables_remove_only_at_zero() {
        let s = settings(10, 3);
        assert!(!view(&s, State { count: 0 }).can_remove);
        assert!(view(&s, State { count: 1 }).can_remove);
        assert_eq!(view(&s, State { count: 1 }).note, "");
    }

    #[test]
    fn a_zero_step_is_rejected() {
        assert_eq!(
            check_settings(&settings(10, 0)),
            Err("The step must be at least 1.".into())
        );
    }

    #[test]
    fn a_step_larger_than_the_maximum_is_rejected() {
        assert_eq!(
            check_settings(&settings(2, 3)),
            Err("The step (3) is larger than the maximum (2).".into())
        );
        assert_eq!(check_settings(&settings(3, 3)), Ok(()));
    }
}
