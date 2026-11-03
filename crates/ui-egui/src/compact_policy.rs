//! Runtime presentation policy, deliberately independent of the session and desktop dock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preference {
    #[default]
    Auto,
    Compact,
    Desktop,
}

impl Preference {
    pub fn parse(value: &str) -> Self {
        match value {
            "compact" => Self::Compact,
            "desktop" => Self::Desktop,
            _ => Self::Auto,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Compact => "compact",
            Self::Desktop => "desktop",
        }
    }
}

/// Resize is presentation only. A gesture owns its current projection until it finishes.
pub fn next_compact(current: bool, preference: Preference, width: f32, height: f32, busy: bool) -> bool {
    if busy || !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return current;
    }
    match preference {
        Preference::Compact => true,
        Preference::Desktop => false,
        Preference::Auto if current => !(width >= 960.0 && height >= 640.0),
        Preference::Auto => width < 900.0 || height < 600.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_and_short_landscape_enter_and_hysteresis_prevents_thrashing() {
        assert!(next_compact(false, Preference::Auto, 390.0, 700.0, false));
        assert!(next_compact(false, Preference::Auto, 1000.0, 390.0, false));
        assert!(next_compact(true, Preference::Auto, 940.0, 900.0, false));
        assert!(next_compact(true, Preference::Auto, 1200.0, 620.0, false));
        assert!(!next_compact(true, Preference::Auto, 960.0, 640.0, false));
        assert!(!next_compact(false, Preference::Auto, 900.0, 600.0, false));
    }

    #[test]
    fn explicit_choices_and_resize_wait_for_gesture_completion() {
        assert!(!next_compact(false, Preference::Compact, 390.0, 700.0, true));
        assert!(next_compact(true, Preference::Desktop, 1400.0, 900.0, true));
        assert!(next_compact(false, Preference::Compact, 1400.0, 900.0, false));
        assert!(!next_compact(true, Preference::Desktop, 390.0, 700.0, false));
        assert_eq!(Preference::parse("untrusted"), Preference::Auto);
        assert_eq!(Preference::parse("compact").id(), "compact");
    }

    #[test]
    fn invalid_dimensions_do_not_replace_a_valid_projection() {
        assert!(next_compact(true, Preference::Auto, f32::NAN, 900.0, false));
        assert!(!next_compact(false, Preference::Auto, 390.0, -1.0, false));
    }
}
