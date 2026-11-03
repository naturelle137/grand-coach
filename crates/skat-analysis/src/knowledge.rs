// SPDX-License-Identifier: EUPL-1.2

//! What one seat knows about the cards: the card counter.
//!
//! Everything here is exact knowledge derived only from the seat's
//! [`PlayerView`] (D-23): own cards, played cards, the own discard, revealed
//! Ouvert cards, and voids proven by failing to follow suit. Heuristic
//! evidence (bidding) lives in [`crate::evidence`].

use skat_core::{Card, CardSet, GameType, PlayerView, Seat};

/// Card knowledge of one seat, derived only from its [`PlayerView`] (D-23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Knowledge {
    /// The viewing seat.
    pub seat: Seat,
    /// The game being played, once announced.
    pub game: Option<GameType>,
    /// Declarer, once bidding is over.
    pub declarer: Option<Seat>,
    /// Own cards.
    pub own: CardSet,
    /// Cards played so far, including the trick in progress.
    pub played: CardSet,
    /// Cards whose place is known without being own or played: the own
    /// discarded Skat and the revealed cards of an Ouvert declarer.
    pub known_elsewhere: CardSet,
    /// The Skat cards this seat knows: its own discard as declarer.
    pub skat: Option<CardSet>,
    /// Cards each seat provably does not hold: the whole follow class
    /// (Trumpf or a side suit) in which it did not follow suit. Exact.
    pub voids: [CardSet; 3],
    /// Number of cards each seat holds.
    pub hand_sizes: [u8; 3],
    /// Augen each seat won in completed tricks.
    pub augen_won: [u8; 3],
}

impl Knowledge {
    /// Derives the knowledge of `view.seat`.
    pub fn from_view(view: &PlayerView) -> Knowledge {
        let mut played: CardSet = view.tricks.iter().flat_map(|t| t.cards).collect();
        played |= view.current_trick.cards.iter().flatten().copied().collect();
        let mut known_elsewhere = view.own_skat.unwrap_or_default();
        if view.declarer != Some(view.seat) {
            known_elsewhere |= view.declarer_cards.unwrap_or_default();
        }
        let game = view.contract.map(|c| c.game);
        let mut augen_won = [0u8; 3];
        for trick in &view.tricks {
            augen_won[trick.winner.index()] += trick.augen();
        }
        Knowledge {
            seat: view.seat,
            game,
            declarer: view.declarer,
            own: view.hand,
            played,
            known_elsewhere,
            skat: view.own_skat,
            voids: view.proven_voids(),
            hand_sizes: view.hand_sizes,
            augen_won,
        }
    }

    /// Cards this seat has not seen: in other hands or in the Skat.
    pub fn unseen(&self) -> CardSet {
        CardSet::ALL - self.own - self.played - self.known_elsewhere
    }

    /// Trumps not yet played and not held by this seat.
    pub fn trumps_out(&self) -> CardSet {
        self.game.map_or(CardSet::EMPTY, |g| {
            g.trumps() & (CardSet::ALL - self.own - self.played)
        })
    }

    /// `true` if no card of the same follow class (Trumpf or the side suit)
    /// still out elsewhere can beat `card` when it is led. A side-suit card
    /// may still be trumped.
    pub fn is_master(&self, card: Card) -> bool {
        let Some(game) = self.game else {
            return false;
        };
        let elsewhere = (CardSet::ALL - self.own - self.played) & game.cards_of(game.follow(card));
        elsewhere.iter().all(|other| !game.beats(other, card, card))
    }

    /// Augen won so far by the declarer's party (tricks only, without the
    /// Skat) and by the defenders; `None` before the declarer is known.
    pub fn party_augen(&self) -> Option<(u8, u8)> {
        let declarer = self.declarer?;
        let own = self.augen_won[declarer.index()];
        let total: u8 = self.augen_won.iter().sum();
        Some((own, total - own))
    }

    /// Augen not yet played and not held by this seat (other hands and an
    /// unknown Skat).
    pub fn augen_out(&self) -> u8 {
        (CardSet::ALL - self.own - self.played).augen()
    }

    /// Where the Trumpf of the game are, as far as this seat knows; `None`
    /// before the announcement and in Null. Unlike [`Knowledge::trumps_out`]
    /// it keeps the own discarded Skat and revealed Ouvert cards apart from
    /// the unseen ones.
    pub fn trumpf_sets(&self) -> Option<TrumpfSets> {
        let game = self.game.filter(|&g| g != GameType::Null)?;
        let total = game.trumps();
        let own_skat = total & self.skat.unwrap_or_default();
        let revealed = total & (self.known_elsewhere - self.skat.unwrap_or_default());
        Some(TrumpfSets {
            total,
            own: total & self.own,
            played: total & self.played,
            own_skat,
            revealed,
            unseen: total & self.unseen(),
            skat_unknown: self.skat.is_none(),
        })
    }
}

/// The Trumpf of a Farbspiel or Grand split into disjoint sets by what one
/// seat knows (exact; `docs/analysis.md`). `own`, `played`, `own_skat`,
/// `revealed` and `unseen` partition `total`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TrumpfSets {
    /// Every Trumpf of the game: 11 in a Farbspiel, the 4 Buben in Grand.
    pub total: CardSet,
    /// In the seat's hand.
    pub own: CardSet,
    /// Played, including the trick in progress.
    pub played: CardSet,
    /// In the seat's own discarded Skat (declarer after Drücken): known,
    /// in nobody's hand.
    pub own_skat: CardSet,
    /// In the revealed hand of an Ouvert declarer, seen by a defender.
    pub revealed: CardSet,
    /// Not seen: in a concealed hand or, if `skat_unknown`, in the Skat.
    /// Which seat holds them is not known from this set.
    pub unseen: CardSet,
    /// The seat does not know the Skat (defender, or Hand declarer), so
    /// unseen cards may lie in it.
    pub skat_unknown: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::{Action, Announcement, Game, Phase, Rank, RuleConfig, Suit};

    /// Plays a seeded game to the first card of the play phase with a fixed
    /// policy: first legal action, Grand announced.
    fn game_in_play(seed: u64) -> Option<Game> {
        let mut game = Game::new(seed, RuleConfig::default());
        while game.phase() != Phase::Spiel {
            let action = match game.phase() {
                Phase::Reizen => {
                    if game.bid() == 0 {
                        Action::Bid(18)
                    } else {
                        Action::Pass
                    }
                }
                Phase::Ansage => Action::Announce(Announcement::plain(GameType::Grand)),
                Phase::Beendet | Phase::Eingepasst => return None,
                _ => game.legal_actions()[0],
            };
            game.apply(action).ok()?;
        }
        Some(game)
    }

    #[test]
    fn unseen_cards_exclude_own_and_played() {
        let mut game = game_in_play(3).expect("seed 3 reaches play");
        let first = game.legal_actions()[0];
        game.apply(first).unwrap();
        for seat in Seat::ALL {
            let k = Knowledge::from_view(&game.view(seat));
            assert!(k.unseen().is_disjoint(k.own));
            assert!(k.unseen().is_disjoint(k.played));
            assert_eq!(k.played.len(), 1);
            let skat_known =
                game.declarer() == Some(seat) && game.view(seat).hand_game == Some(false);
            assert_eq!(k.known_elsewhere.len(), if skat_known { 2 } else { 0 });
            assert_eq!(
                k.unseen().len() + k.own.len() + k.played.len() + k.known_elsewhere.len(),
                32
            );
            assert_eq!(k.game, Some(GameType::Grand));
        }
    }

    #[test]
    fn master_cards() {
        let own =
            CardSet::single(Card::bube(Suit::Kreuz)) | Card::new(Suit::Herz, Rank::Zehn).into();
        let mut k = Knowledge {
            seat: Seat::Vorhand,
            game: Some(GameType::Grand),
            declarer: None,
            own,
            played: CardSet::EMPTY,
            known_elsewhere: CardSet::EMPTY,
            skat: None,
            voids: [CardSet::EMPTY; 3],
            hand_sizes: [10; 3],
            augen_won: [0; 3],
        };
        assert!(k.is_master(Card::bube(Suit::Kreuz)));
        assert!(!k.is_master(Card::new(Suit::Herz, Rank::Zehn)));
        k.played.insert(Card::new(Suit::Herz, Rank::Ass));
        assert!(k.is_master(Card::new(Suit::Herz, Rank::Zehn)));
        assert_eq!(k.trumps_out().len(), 3);
    }
}
