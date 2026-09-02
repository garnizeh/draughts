//! The canned fallback — infallible by construction.
//!
//! This adapter is what makes commentary *optional* rather than merely
//! unreliable. Every `(CommentaryEvent, Tone)` pair returns a non-empty line;
//! a missing pair is a startup failure, not a runtime blank (§20.7).

use async_trait::async_trait;

use super::breaker::CircuitState;
use super::{
    Commentary, CommentaryContext, CommentaryEvent, FaceAdapter, FaceError, FallbackReason, Tone,
};

/// Lines are indexed by `(event, tone)` and chosen by ply, so that a game does
/// not repeat the same line twice in a row without needing an RNG here.
pub struct CannedFaceAdapter;

impl CannedFaceAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Produce a line. Cannot fail, and must not.
    #[must_use]
    pub fn canned(
        &self,
        context: &CommentaryContext,
        reason: FallbackReason,
        circuit_state: CircuitState,
    ) -> Commentary {
        let lines = lines_for(context.event, context.tone);
        let text = lines[context.ply as usize % lines.len()].to_string();

        Commentary {
            token_count: text.split_whitespace().count() as u32,
            text,
            provider: "canned",
            fallback_used: true,
            fallback_reason: Some(reason),
            circuit_state,
            latency_ms: 0,
        }
    }
}

impl Default for CannedFaceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaceAdapter for CannedFaceAdapter {
    fn name(&self) -> &'static str {
        "canned"
    }

    async fn generate_commentary(
        &self,
        context: &CommentaryContext,
    ) -> Result<Commentary, FaceError> {
        Ok(self.canned(context, FallbackReason::Disabled, CircuitState::Closed))
    }

    fn is_available(&self) -> bool {
        true
    }
}

/// The line table.
///
/// Deliberately unglamorous and deliberately complete: coverage matters more
/// than wit, because an uncovered pair renders a blank commentary pane.
///
/// Nothing here names a square, a move, or a piece — the same constraint the
/// generated path is held to (§7.7).
fn lines_for(event: CommentaryEvent, tone: Tone) -> &'static [&'static str] {
    use CommentaryEvent as E;
    use Tone as T;

    match (event, tone) {
        (E::GameStart, T::Neutral) => &["Board's set. Let's begin.", "New game. Good luck."],
        (E::GameStart, T::Playful) => &["Twelve each. Try to keep them.", "Here we go again."],
        (E::GameStart, T::Sarcastic) => &["This should be brief.", "I've cleared my afternoon."],
        (E::GameStart, T::Competitive) => &["Let's see what you've got.", "Begin."],

        (E::HumanMove, T::Neutral) => &["Noted.", "Your move is in."],
        (E::HumanMove, T::Playful) => &["Bold.", "Interesting choice."],
        (E::HumanMove, T::Sarcastic) => &["Sure. Why not.", "That's certainly a move."],
        (E::HumanMove, T::Competitive) => &["Seen it.", "Not enough."],

        (E::CpuMove, T::Neutral) => &["Done.", "My turn taken."],
        (E::CpuMove, T::Playful) => &["There.", "Your problem now."],
        (E::CpuMove, T::Sarcastic) => &["I thought about it for ages.", "Barely had to think."],
        (E::CpuMove, T::Competitive) => &["Pressure's on.", "Your move."],

        (E::Capture, T::Neutral) => &["A piece comes off.", "Trade made."],
        (E::Capture, T::Playful) => &["Snack.", "One fewer to worry about."],
        (E::Capture, T::Sarcastic) => &["You won't be needing that.", "Oops."],
        (E::Capture, T::Competitive) => &["That's material.", "Advantage taken."],

        (E::MultiCapture, T::Neutral) => &["Several pieces at once.", "A chain."],
        (E::MultiCapture, T::Playful) => &["Buffet.", "Couldn't stop myself."],
        (E::MultiCapture, T::Sarcastic) => &["I'll take the whole row, thanks.", "Generous."],
        (E::MultiCapture, T::Competitive) => &["That's the game shifting.", "Decisive."],

        (E::Promotion, T::Neutral) => &["A king is crowned.", "Promotion."],
        (E::Promotion, T::Playful) => &["Someone got a hat.", "Upgraded."],
        (E::Promotion, T::Sarcastic) => &["About time.", "Congratulations, I suppose."],
        (E::Promotion, T::Competitive) => &["That changes the back rank.", "Kings matter now."],

        (E::Win, T::Neutral) => &["That's the game.", "Result recorded."],
        (E::Win, T::Playful) => &["Good game. Really.", "Rematch?"],
        (E::Win, T::Sarcastic) => &["Shocking outcome.", "Nobody saw that coming."],
        (E::Win, T::Competitive) => &["Game.", "Closed out."],

        (E::Loss, T::Neutral) => &["Well played.", "You had it."],
        (E::Loss, T::Playful) => &["Fine, fine. You win.", "I'll allow it."],
        (E::Loss, T::Sarcastic) => &["Enjoy it while it lasts.", "Beginner's luck."],
        (E::Loss, T::Competitive) => &["Good. Again.", "Noted. Rematch."],

        (E::Draw, T::Neutral) => &["A draw.", "Even at the end."],
        (E::Draw, T::Playful) => &["Nobody wins, nobody cries.", "Share the point."],
        (E::Draw, T::Sarcastic) => &["Thrilling.", "We'll call it even."],
        (E::Draw, T::Competitive) => &["Not settled.", "Next one decides it."],

        (E::IdleTaunt, T::Neutral) => &["Still your move.", "Waiting."],
        (E::IdleTaunt, T::Playful) => &["Take your time. Really.", "Still here."],
        (E::IdleTaunt, T::Sarcastic) => &["I've aged.", "Should I come back later?"],
        (E::IdleTaunt, T::Competitive) => &["Clock's running.", "Move."],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::GameStatusSummary;
    use crate::rules::Side;

    fn context(event: CommentaryEvent, tone: Tone, ply: u32) -> CommentaryContext {
        CommentaryContext {
            event,
            tone,
            ply,
            side_to_move: Side::Black,
            material_difference: 0,
            game_status: GameStatusSummary::Ongoing,
            max_tokens: 64,
        }
    }

    /// §20.7: every `(event, tone)` pair returns a non-empty line. A missing
    /// pair is a blank commentary pane in production.
    #[test]
    fn every_event_and_tone_pair_is_covered() {
        let adapter = CannedFaceAdapter::new();

        for event in CommentaryEvent::ALL {
            for tone in Tone::ALL {
                let lines = lines_for(event, tone);
                assert!(!lines.is_empty(), "no lines for {event:?}/{tone:?}");

                for ply in 0..8 {
                    let commentary = adapter.canned(
                        &context(event, tone, ply),
                        FallbackReason::CircuitOpen,
                        CircuitState::Open,
                    );
                    assert!(
                        !commentary.text.trim().is_empty(),
                        "blank line for {event:?}/{tone:?} at ply {ply}"
                    );
                    assert!(commentary.fallback_used);
                }
            }
        }
    }

    /// §7.7: the Face layer never names a move. The canned lines are held to
    /// the same standard as generated ones, because the UI cannot tell them
    /// apart and neither can a player.
    #[test]
    fn no_canned_line_names_a_square_or_a_move() {
        for event in CommentaryEvent::ALL {
            for tone in Tone::ALL {
                for line in lines_for(event, tone) {
                    assert!(
                        !line.chars().any(|c| c.is_ascii_digit()),
                        "canned line mentions a number, which reads as a square: {line:?}"
                    );
                    for banned in ["square", "jump to", "move to", "->"] {
                        assert!(
                            !line.to_lowercase().contains(banned),
                            "canned line {line:?} contains {banned:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_line_varies_with_ply() {
        let adapter = CannedFaceAdapter::new();
        let first = adapter.canned(
            &context(CommentaryEvent::Capture, Tone::Playful, 0),
            FallbackReason::CircuitOpen,
            CircuitState::Open,
        );
        let second = adapter.canned(
            &context(CommentaryEvent::Capture, Tone::Playful, 1),
            FallbackReason::CircuitOpen,
            CircuitState::Open,
        );
        assert_ne!(first.text, second.text);
    }
}
