pub(crate) const REDUCED_MOTION_PHASE: u8 = 1;

pub(crate) const fn skeleton_tick_required(is_streaming: bool, reduced_motion: bool) -> bool {
    is_streaming && !reduced_motion
}

pub(crate) const fn rendered_skeleton_phase(phase: u8, reduced_motion: bool) -> u8 {
    if reduced_motion {
        REDUCED_MOTION_PHASE
    } else {
        phase
    }
}

pub(crate) const fn next_skeleton_phase(phase: u8, reduced_motion: bool) -> u8 {
    if reduced_motion {
        phase
    } else {
        (phase + 1) % 4
    }
}

pub(crate) fn accent_opacity(phase: u8, reduced_motion: bool) -> f32 {
    if reduced_motion {
        1.0
    } else {
        0.6 + 0.4 * (f32::from(phase) / 3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{REDUCED_MOTION_PHASE, skeleton_tick_required};
    use crate::state::UiState;
    use crate::{App, Message};

    #[test]
    fn motion_default_state_is_enabled() {
        // Given: a fresh transient UI state.
        let ui = UiState::new(false, false);

        // When: no accessibility preference has been changed.

        // Then: normal motion remains the session default.
        assert!(!ui.reduced_motion);
    }

    #[test]
    fn motion_toggle_transition_freezes_phase_and_queued_tick_is_harmless() {
        // Given: a streaming app currently displaying a non-stable phase.
        let (mut app, _) = App::new();
        app.skeleton_phase = 3;

        // When: reduced motion is enabled and a previously queued tick arrives.
        let _ = app.update(Message::SetReducedMotion(true));
        let _ = app.update(Message::SkeletonTick);

        // Then: the preference is transiently enabled and the visible phase is stable.
        assert!(app.ui.reduced_motion);
        assert_eq!(app.skeleton_phase, REDUCED_MOTION_PHASE);
    }

    #[test]
    fn motion_subscription_policy_only_ticks_while_streaming_with_normal_motion() {
        // Given: streaming and non-streaming states with both motion preferences.

        // When: the subscription policy is evaluated.

        // Then: the periodic tick is scheduled only for a normal-motion stream.
        assert!(skeleton_tick_required(true, false));
        assert!(!skeleton_tick_required(false, false));
        assert!(!skeleton_tick_required(true, true));
        assert!(!skeleton_tick_required(false, true));
    }
}
