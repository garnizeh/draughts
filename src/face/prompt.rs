//! Prompt construction — §7.3, §7.7.
//!
//! The prompt is rendered from a [`CommentaryContext`] and from nothing else.
//! §20.7 holds a golden-file test asserting that no rendered prompt contains a
//! move, a square index, a legal-move list, or wording that would invite one,
//! and the fixture set is extended whenever a `CommentaryEvent` is added.
//!
//! The constraint is easier to keep than to recover: there is no move in the
//! context type, so there is nothing here to leak.

use super::{CommentaryContext, CommentaryEvent, GameStatusSummary, Tone};

/// The system prompt. Static, because a system prompt that varies with game
/// state is a system prompt that can leak game state.
pub const SYSTEM_PROMPT: &str = "\
You are the voice of a draughts engine that has already chosen its move. \
You do not play, suggest, evaluate, or describe moves. \
Reply with one short sentence of banter and nothing else. \
Never mention squares, coordinates, numbers, or specific pieces.";

/// Render the user turn of the prompt.
#[must_use]
pub fn render(context: &CommentaryContext) -> String {
    let event = match context.event {
        CommentaryEvent::GameStart => "the game has just started",
        CommentaryEvent::HumanMove => "the opponent has just moved",
        CommentaryEvent::CpuMove => "you have just moved",
        CommentaryEvent::Capture => "a piece was captured",
        CommentaryEvent::MultiCapture => "several pieces were captured in one turn",
        CommentaryEvent::Promotion => "a piece was crowned",
        CommentaryEvent::Win => "you won",
        CommentaryEvent::Loss => "you lost",
        CommentaryEvent::Draw => "the game was drawn",
        CommentaryEvent::IdleTaunt => "the opponent has been thinking for a while",
    };

    let tone = match context.tone {
        Tone::Neutral => "neutral",
        Tone::Playful => "playful",
        Tone::Sarcastic => "sarcastic",
        Tone::Competitive => "competitive",
    };

    // Material is given as a coarse band rather than a number: the exact figure
    // is a search-adjacent fact, and a band is all a taunt can use anyway.
    let standing = match context.material_difference {
        d if d >= 3 => "you are well ahead",
        d if d >= 1 => "you are slightly ahead",
        0 => "material is level",
        d if d >= -2 => "you are slightly behind",
        _ => "you are well behind",
    };

    let phase = match context.ply {
        0..=15 => "the opening",
        16..=45 => "the middlegame",
        _ => "the endgame",
    };

    let status = match context.game_status {
        GameStatusSummary::Ongoing => "the game is still going",
        GameStatusSummary::HumanWon => "the opponent has won",
        GameStatusSummary::CpuWon => "you have won",
        GameStatusSummary::Drawn => "the game ended level",
    };

    format!(
        "Situation: {event}. It is {phase} and {standing}; {status}. \
         Tone: {tone}. One sentence."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Side;

    fn context(event: CommentaryEvent, tone: Tone, ply: u32, material: i32) -> CommentaryContext {
        CommentaryContext {
            event,
            tone,
            ply,
            side_to_move: Side::Black,
            material_difference: material,
            game_status: GameStatusSummary::Ongoing,
            max_tokens: 64,
        }
    }

    /// §20.7's prompt-safety test. It is exhaustive over events and tones on
    /// purpose: the property must hold for every prompt the system can render,
    /// not for a sample of them.
    #[test]
    fn no_rendered_prompt_names_a_move_or_a_square() {
        for event in CommentaryEvent::ALL {
            for tone in Tone::ALL {
                for ply in [0, 20, 80] {
                    for material in [-5, -1, 0, 1, 5] {
                        let prompt = render(&context(event, tone, ply, material));

                        assert!(
                            !prompt.chars().any(|c| c.is_ascii_digit()),
                            "a digit in a prompt reads as a square index: {prompt:?}"
                        );

                        for banned in [
                            "square",
                            "move to",
                            "jump",
                            "coordinate",
                            "legal",
                            "best",
                            "should play",
                            "suggest",
                            "recommend",
                            "->",
                        ] {
                            assert!(
                                !prompt.to_lowercase().contains(banned),
                                "prompt for {event:?}/{tone:?} contains {banned:?}: {prompt:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_system_prompt_forbids_playing() {
        let lowered = SYSTEM_PROMPT.to_lowercase();
        assert!(lowered.contains("do not play"));
        assert!(lowered.contains("never mention squares"));
    }

    #[test]
    fn material_is_given_as_a_band_not_a_number() {
        let prompt = render(&context(CommentaryEvent::Capture, Tone::Neutral, 20, 7));
        assert!(prompt.contains("well ahead"));
        assert!(!prompt.contains('7'));
    }
}
