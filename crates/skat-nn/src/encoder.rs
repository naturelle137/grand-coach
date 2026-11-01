// SPDX-License-Identifier: EUPL-1.2

//! The permitted-view encoder: a fixed-length feature vector from one
//! seat's [`PlayerView`] and nothing else (D-07, D-23).
//!
//! Seats are encoded relative to the viewing seat (0 = self, 1 = next seat
//! in playing order, 2 = the one after), so one model serves every seat;
//! the own position at the table is a feature of its own. Every block below
//! reads only fields of the view, which already hides what the seat may
//! not know: the own Skat appears only for the declarer after Drücken
//! (`PlayerView::own_skat`), an Ouvert declarer's cards only after the
//! reveal (`PlayerView::declarer_cards`).
//!
//! | Offset | Size | Feature |
//! |--------|------|---------|
//! | 0 | 32 | own hand |
//! | 32 | 32 | own discarded Skat (declarer after Drücken) |
//! | 64 | 32 | revealed Ouvert cards of the declarer |
//! | 96 | 96 | cards of completed tricks, by relative seat that played them |
//! | 192 | 32 | led card of the trick in progress |
//! | 224 | 32 | second card of the trick in progress |
//! | 256 | 3 | relative seat that led the trick in progress |
//! | 259 | 3 | own position: Vorhand, Mittelhand, Hinterhand |
//! | 262 | 5 | phase: Reizen, Skat pickup, Drücken, Ansage, Spiel |
//! | 267 | 3 | relative seat of the declarer |
//! | 270 | 3 | highest value bid or held per relative seat / 264 |
//! | 273 | 3 | passed, per relative seat |
//! | 276 | 1 | current bid / 264 |
//! | 277 | 2 | Hand game, game with pickup |
//! | 279 | 6 | game type: Kreuz, Pik, Herz, Karo, Grand, Null |
//! | 285 | 3 | Schneider, Schwarz, Ouvert announced |
//! | 288 | 2 | Kontra, Re said |
//! | 290 | 1 | Kontra/Re switched on |
//! | 291 | 1 | completed tricks / 10 |
//! | 292 | 2 | Augen won by the own party, by the other party / 120 |

use std::ops::Range;

use skat_core::{Call, CardSet, GameType, KontraLevel, Phase, PlayerView, Seat, Suit};

/// Version of the feature layout and of the action space
/// ([`crate::actions`]). A model trained for another version is rejected.
pub const ENCODER_VERSION: u16 = 1;

/// Length of the feature vector.
pub const FEATURES: usize = 294;

/// Highest bid value, used to scale bids into [0, 1].
const MAX_BID: f32 = 264.0;

/// Features of the own discarded Skat.
pub const OWN_SKAT_BLOCK: Range<usize> = OWN_SKAT..OWN_SKAT + 32;
/// Features of the revealed Ouvert cards.
pub const OUVERT_BLOCK: Range<usize> = OUVERT..OUVERT + 32;

const HAND: usize = 0;
const OWN_SKAT: usize = 32;
const OUVERT: usize = 64;
const PLAYED: usize = 96;
const TRICK_LED: usize = 192;
const TRICK_SECOND: usize = 224;
const TRICK_LEADER: usize = 256;
const POSITION: usize = 259;
const PHASE: usize = 262;
const DECLARER: usize = 267;
const BID_VALUE: usize = 270;
const PASSED: usize = 273;
const CURRENT_BID: usize = 276;
const HAND_GAME: usize = 277;
const GAME_TYPE: usize = 279;
const LEVELS: usize = 285;
const KONTRA: usize = 288;
const KONTRA_RULE: usize = 290;
const TRICKS: usize = 291;
const AUGEN: usize = 292;

/// The index of a decision phase in the phase block, `None` for a finished
/// or thrown-in game.
pub(crate) fn phase_index(phase: Phase) -> Option<usize> {
    match phase {
        Phase::Reizen => Some(0),
        Phase::SkatAufnahme => Some(1),
        Phase::Druecken => Some(2),
        Phase::Ansage => Some(3),
        Phase::Spiel => Some(4),
        Phase::Beendet | Phase::Eingepasst => None,
    }
}

/// Position of `seat` relative to the viewing seat `me`.
fn relative(me: Seat, seat: Seat) -> usize {
    (seat.index() + 3 - me.index()) % 3
}

fn set_cards(features: &mut [f32], offset: usize, cards: CardSet) {
    for card in cards {
        features[offset + usize::from(card.index())] = 1.0;
    }
}

fn flag(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}

/// Encodes `view` into [`FEATURES`] numbers. Depends on nothing but the
/// view: two games in which the seat sees the same view give the same
/// features.
pub fn encode(view: &PlayerView) -> Vec<f32> {
    let me = view.seat;
    let mut f = vec![0.0; FEATURES];
    set_cards(&mut f, HAND, view.hand);
    if let Some(skat) = view.own_skat {
        set_cards(&mut f, OWN_SKAT, skat);
    }
    if let Some(cards) = view.declarer_cards {
        set_cards(&mut f, OUVERT, cards);
    }
    for trick in &view.tricks {
        for (i, &card) in trick.cards.iter().enumerate() {
            let seat = relative(me, trick.seat_of(i));
            f[PLAYED + 32 * seat + usize::from(card.index())] = 1.0;
        }
    }
    let trick = &view.current_trick;
    if let Some(card) = trick.cards[0] {
        f[TRICK_LED + usize::from(card.index())] = 1.0;
    }
    if let Some(card) = trick.cards[1] {
        f[TRICK_SECOND + usize::from(card.index())] = 1.0;
    }
    if let (Some(leader), false) = (trick.leader, trick.is_empty()) {
        f[TRICK_LEADER + relative(me, leader)] = 1.0;
    }
    f[POSITION + me.index()] = 1.0;
    if let Some(p) = phase_index(view.phase) {
        f[PHASE + p] = 1.0;
    }
    if let Some(declarer) = view.declarer {
        f[DECLARER + relative(me, declarer)] = 1.0;
    }
    for record in &view.bidding {
        let seat = relative(me, record.seat);
        match record.call {
            Call::Bid(v) | Call::Hold(v) => {
                let value = f32::from(v) / MAX_BID;
                f[BID_VALUE + seat] = f[BID_VALUE + seat].max(value);
            }
            Call::Pass => f[PASSED + seat] = 1.0,
        }
    }
    f[CURRENT_BID] = f32::from(view.bid) / MAX_BID;
    match view.hand_game {
        Some(true) => f[HAND_GAME] = 1.0,
        Some(false) => f[HAND_GAME + 1] = 1.0,
        None => {}
    }
    if let Some(contract) = view.contract {
        let game = match contract.game {
            GameType::Farbe(Suit::Kreuz) => 0,
            GameType::Farbe(Suit::Pik) => 1,
            GameType::Farbe(Suit::Herz) => 2,
            GameType::Farbe(Suit::Karo) => 3,
            GameType::Grand => 4,
            GameType::Null => 5,
        };
        f[GAME_TYPE + game] = 1.0;
        f[LEVELS] = flag(contract.schneider);
        f[LEVELS + 1] = flag(contract.schwarz);
        f[LEVELS + 2] = flag(contract.ouvert);
    }
    match view.kontra {
        KontraLevel::None => {}
        KontraLevel::Kontra => f[KONTRA] = 1.0,
        KontraLevel::Re => {
            f[KONTRA] = 1.0;
            f[KONTRA + 1] = 1.0;
        }
    }
    f[KONTRA_RULE] = flag(view.config.kontra_re);
    f[TRICKS] = view.tricks.len() as f32 / 10.0;
    if let Some(declarer) = view.declarer {
        let own_party = |seat: Seat| (seat == declarer) == (me == declarer);
        for trick in &view.tricks {
            let side = if own_party(trick.winner) { 0 } else { 1 };
            f[AUGEN + side] += f32::from(trick.augen()) / 120.0;
        }
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::{Game, RuleConfig};

    #[test]
    fn blocks_tile_the_vector() {
        assert_eq!(AUGEN + 2, FEATURES);
        let game = Game::new(1, RuleConfig { kontra_re: true });
        for seat in Seat::ALL {
            let f = encode(&game.view(seat));
            assert_eq!(f.len(), FEATURES);
            assert_eq!(f[HAND..HAND + 32].iter().sum::<f32>(), 10.0);
            assert_eq!(f[POSITION + seat.index()], 1.0);
            assert_eq!(f[PHASE], 1.0);
            assert!(f.iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x)));
        }
    }

    #[test]
    fn relative_seats() {
        assert_eq!(relative(Seat::Mittelhand, Seat::Mittelhand), 0);
        assert_eq!(relative(Seat::Mittelhand, Seat::Hinterhand), 1);
        assert_eq!(relative(Seat::Mittelhand, Seat::Vorhand), 2);
    }
}
