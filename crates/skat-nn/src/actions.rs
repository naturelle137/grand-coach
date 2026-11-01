// SPDX-License-Identifier: EUPL-1.2

//! The fixed action space of the policy head: every [`Action`] the engine
//! can offer has one index, in this order:
//!
//! | Block | Size | Actions |
//! |-------|------|---------|
//! | bids | 63 | `Bid(v)` for every value of [`reizwerte`], ascending |
//! | calls | 4 | `Hold`, `Pass`, `Hand`, `TakeSkat` |
//! | discards | 496 | `Discard` of each pair of cards `i < j` (card index order) |
//! | announcements | 22 | `Announce(a)` in the order of [`Announcement::all`]`(true)` |
//! | cards | 32 | `Play(c)` by card index |
//! | Kontra/Re | 4 | `Kontra`, `KeinKontra`, `Re`, `KeinRe` |
//!
//! The layout is part of the encoder version ([`crate::ENCODER_VERSION`]).

use std::sync::LazyLock;

use skat_core::{Action, Announcement, Card, CardSet, reizwerte};

const BIDS: usize = 63;
const CALLS: usize = 4;
const DISCARDS: usize = 32 * 31 / 2;
const ANNOUNCEMENTS: usize = 22;
const CARDS: usize = 32;
const KONTRA: usize = 4;

const CALL_START: usize = BIDS;
const DISCARD_START: usize = CALL_START + CALLS;
const ANNOUNCE_START: usize = DISCARD_START + DISCARDS;
const CARD_START: usize = ANNOUNCE_START + ANNOUNCEMENTS;
const KONTRA_START: usize = CARD_START + CARDS;

/// Number of actions in the policy head.
pub const ACTIONS: usize = KONTRA_START + KONTRA;

static BID_VALUES: LazyLock<Vec<u16>> = LazyLock::new(reizwerte);
static ANNOUNCEMENT_LIST: LazyLock<Vec<Announcement>> = LazyLock::new(|| Announcement::all(true));

/// Index of the pair `i < j` among all pairs of the 32 cards.
const fn pair_index(i: usize, j: usize) -> usize {
    i * 32 - i * (i + 1) / 2 + (j - i - 1)
}

/// The index of `action` in the policy head; `None` for an action the
/// engine never offers (an invalid bid value, a discard of other than two
/// cards, an invalid announcement).
pub fn action_index(action: Action) -> Option<usize> {
    match action {
        Action::Bid(v) => BID_VALUES.binary_search(&v).ok(),
        Action::Hold => Some(CALL_START),
        Action::Pass => Some(CALL_START + 1),
        Action::Hand => Some(CALL_START + 2),
        Action::TakeSkat => Some(CALL_START + 3),
        Action::Discard(cards) => {
            if cards.len() != 2 {
                return None;
            }
            let mut it = cards.iter();
            let i = usize::from(it.next()?.index());
            let j = usize::from(it.next()?.index());
            Some(DISCARD_START + pair_index(i.min(j), i.max(j)))
        }
        Action::Announce(a) => ANNOUNCEMENT_LIST
            .iter()
            .position(|&x| x == a)
            .map(|p| ANNOUNCE_START + p),
        Action::Play(card) => Some(CARD_START + usize::from(card.index())),
        Action::Kontra => Some(KONTRA_START),
        Action::KeinKontra => Some(KONTRA_START + 1),
        Action::Re => Some(KONTRA_START + 2),
        Action::KeinRe => Some(KONTRA_START + 3),
    }
}

/// The action at `index` of the policy head; `None` outside the range.
pub fn action_at(index: usize) -> Option<Action> {
    if index < CALL_START {
        return BID_VALUES.get(index).map(|&v| Action::Bid(v));
    }
    if index < DISCARD_START {
        return Some(
            [Action::Hold, Action::Pass, Action::Hand, Action::TakeSkat][index - CALL_START],
        );
    }
    if index < ANNOUNCE_START {
        let mut rest = index - DISCARD_START;
        for i in 0..32u8 {
            let row = 31 - usize::from(i);
            if rest < row {
                let j = i + 1 + rest as u8;
                let pair =
                    CardSet::single(Card::from_index(i)?) | CardSet::single(Card::from_index(j)?);
                return Some(Action::Discard(pair));
            }
            rest -= row;
        }
        return None;
    }
    if index < CARD_START {
        return ANNOUNCEMENT_LIST
            .get(index - ANNOUNCE_START)
            .map(|&a| Action::Announce(a));
    }
    if index < KONTRA_START {
        return Card::from_index((index - CARD_START) as u8).map(Action::Play);
    }
    [
        Action::Kontra,
        Action::KeinKontra,
        Action::Re,
        Action::KeinRe,
    ]
    .get(index - KONTRA_START)
    .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_sizes_match_the_engine() {
        assert_eq!(reizwerte().len(), BIDS);
        assert_eq!(Announcement::all(true).len(), ANNOUNCEMENTS);
        assert!(
            Announcement::all(false)
                .iter()
                .all(|a| Announcement::all(true).contains(a))
        );
        assert_eq!(ACTIONS, 621);
    }

    #[test]
    fn indices_round_trip() {
        for index in 0..ACTIONS {
            let action = action_at(index).expect("every index has an action");
            assert_eq!(action_index(action), Some(index), "{action:?}");
        }
        assert_eq!(action_at(ACTIONS), None);
    }

    #[test]
    fn invalid_actions_have_no_index() {
        assert_eq!(action_index(Action::Bid(19)), None);
        assert_eq!(action_index(Action::Discard(CardSet::EMPTY)), None);
        let three: CardSet = (0..3).filter_map(Card::from_index).collect();
        assert_eq!(action_index(Action::Discard(three)), None);
    }
}
