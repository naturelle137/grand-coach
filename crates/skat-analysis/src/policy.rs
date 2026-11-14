// SPDX-License-Identifier: EUPL-1.2

//! Continuation policy: fixed rules of thumb for the decisions before card
//! play (bidding, Hand or pickup, Drücken, announcement, Kontra/Re).
//!
//! The option evaluation ([`crate::analyze`]) enumerates the options of the
//! decision at hand exactly; every later decision of every seat in a
//! simulated deal follows this policy, and card play follows double-dummy
//! search. The baseline opponent in `skat-ai` plays by the same rules, so
//! AI and coach share one strategic computation. Every rule only looks at
//! the deciding seat's own cards (and public facts such as the bid).

use skat_core::{
    Action, Announcement, Card, CardSet, Contract, GameType, Phase, PlayerView, reizwerte,
};

use crate::hand::{TRUMP_GAMES, game_value, null_risks, power, strength, trump_count};

/// Strength from which a suit game is bid.
const SUIT_GAME: i32 = 60;
/// Strength from which a Grand is bid (with at least two Jacks).
const GRAND: i32 = 70;
/// Suits that may be unsafe for a Null game (see `null_risks`).
const NULL_RISKS: usize = 1;
/// Strength from which the declarer plays Hand instead of picking up.
const HAND: i32 = 100;
/// Trumps in a defender's hand from which it says Kontra.
pub const KONTRA_TRUMPS: usize = 5;
/// Strength from which the declarer answers Kontra with Re.
pub const RE: i32 = 90;

/// Which branch of [`choose_game`] supplied the candidate game and ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GameBranch {
    /// At least two Buben and Grand strength; value from own Spitzen.
    Grand,
    /// Strongest suit reaches the suit threshold; value from own Spitzen.
    Suit,
    /// No suit game, at most one unsafe Null suit; fixed value 23.
    Null,
    /// Nothing reaches a threshold: ceiling 0, the seat passes.
    Weak,
}

/// Best game for `cards` and the highest bid the policy goes to (0: pass).
pub fn choose_game(cards: CardSet) -> (GameType, u16) {
    let (game, limit, _) = choose_game_branch(cards);
    (game, limit)
}

/// [`choose_game`] plus the branch that produced it.
pub fn choose_game_branch(cards: CardSet) -> (GameType, u16, GameBranch) {
    let grand_ok =
        (cards & GameType::BUBEN).len() >= 2 && strength(GameType::Grand, cards) >= GRAND;
    if grand_ok {
        let value = game_value(GameType::Grand, cards, false).unwrap_or(0);
        return (GameType::Grand, value, GameBranch::Grand);
    }
    let best = TRUMP_GAMES
        .into_iter()
        .filter(|&g| g != GameType::Grand)
        .max_by_key(|&g| (strength(g, cards), trump_count(g, cards)))
        .expect("four suit games");
    if strength(best, cards) >= SUIT_GAME {
        (
            best,
            game_value(best, cards, false).unwrap_or(0),
            GameBranch::Suit,
        )
    } else if null_risks(cards) <= NULL_RISKS {
        (GameType::Null, 23, GameBranch::Null)
    } else {
        (best, 0, GameBranch::Weak)
    }
}

/// Bidding action for a seat whose highest acceptable bid is `limit`, given
/// the current bid and the legal actions.
pub fn bid(limit: u16, current: u16, legal: &[Action]) -> Action {
    if legal.contains(&Action::Hold) {
        return if current <= limit {
            Action::Hold
        } else {
            Action::Pass
        };
    }
    match next_bid(current) {
        Some(value) if value <= limit && legal.contains(&Action::Bid(value)) => Action::Bid(value),
        _ => Action::Pass,
    }
}

/// Next standard bid value above `current`, if any.
pub fn next_bid(current: u16) -> Option<u16> {
    reizwerte().into_iter().find(|&v| v > current)
}

/// Exact inputs and outcome of Easy's bidding rule for one permitted view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BidRule {
    /// Actual selected action: Hold, Bid(value) or Pass.
    pub action: Action,
    /// Public current bid (0: nothing bid yet).
    pub current: u16,
    /// Whether this seat answers (holds) the current bid rather than naming one.
    pub holds: bool,
    /// Value this decision is about: the current bid when holding, otherwise
    /// the next standard value (none above the highest value).
    pub stake: Option<u16>,
    /// Heuristic ceiling from the own ten cards only (0: pass).
    pub ceiling: u16,
    /// Candidate game behind the ceiling; conditional, not a contract.
    pub game: GameType,
    /// Branch of the game choice.
    pub branch: GameBranch,
    /// Easy's wanted action was illegal and the first legal action was taken.
    pub fallback: bool,
}

/// Easy's bidding rule for a Reizen decision; none in other phases or
/// without a turn.
pub fn bid_rule(view: &PlayerView) -> Option<BidRule> {
    let legal = &view.legal_actions;
    if view.phase != Phase::Reizen || legal.is_empty() {
        return None;
    }
    let (game, ceiling, branch) = choose_game_branch(view.hand);
    let wanted = bid(ceiling, view.bid, legal);
    let fallback = !legal.contains(&wanted);
    let holds = legal.contains(&Action::Hold);
    Some(BidRule {
        action: if fallback { legal[0] } else { wanted },
        current: view.bid,
        holds,
        stake: if holds {
            Some(view.bid)
        } else {
            next_bid(view.bid)
        },
        ceiling,
        game,
        branch,
        fallback,
    })
}

/// Hand or pickup for the declarer's ten cards.
pub fn hand_or_pickup(cards: CardSet) -> Action {
    let (game, _) = choose_game(cards);
    if strength(game, cards) >= HAND {
        Action::Hand
    } else {
        Action::TakeSkat
    }
}

/// Exact inputs and outcome of Easy's Hand-or-pickup rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PickupRule {
    /// Actual selected action: Hand or TakeSkat.
    pub action: Action,
    /// Candidate game from the own ten cards.
    pub game: GameType,
    /// Branch of the game choice.
    pub branch: GameBranch,
    /// Strength of the own ten cards for `game` (0 for Null).
    pub strength: i32,
    /// Strength from which Easy plays Hand.
    pub threshold: i32,
    /// Easy's wanted action was illegal and the first legal action was taken.
    pub fallback: bool,
}

/// Easy's Hand-or-pickup rule; none outside Skataufnahme or without a turn.
pub fn pickup_rule(view: &PlayerView) -> Option<PickupRule> {
    let legal = &view.legal_actions;
    if view.phase != Phase::SkatAufnahme || legal.is_empty() {
        return None;
    }
    let (game, _, branch) = choose_game_branch(view.hand);
    let wanted = hand_or_pickup(view.hand);
    let fallback = !legal.contains(&wanted);
    Some(PickupRule {
        action: if fallback { legal[0] } else { wanted },
        game,
        branch,
        strength: strength(game, view.hand),
        threshold: HAND,
        fallback,
    })
}

/// Two cards to put away: never Aces or trumps if avoidable; prefer short
/// suits and unguarded Tens (their Augen count for the declarer).
pub fn discard(cards: CardSet) -> CardSet {
    let (game, _) = choose_game(cards);
    let mut ranked: Vec<Card> = cards.iter().collect();
    ranked.sort_by_key(|&card| keep_score(game, cards, card));
    ranked[..2].iter().copied().collect()
}

/// Why the discard rule ranks a card where it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Keep {
    /// Null: the higher the card, the earlier it goes.
    Null,
    /// Trumpf: kept before everything else.
    Trumpf,
    /// Side Ace: kept.
    Ass,
    /// Ten guarded by the Ace of its suit: kept.
    GuardedTen,
    /// Other side card; shorter suits and more Augen (loose Tens) go first.
    Side {
        /// Cards of this suit (follow class) among the declarer's cards.
        suit: u8,
    },
}

/// Exact inputs and outcome of Easy's discard rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiscardRule {
    /// Actual selected action: Discard of two cards.
    pub action: Action,
    /// Candidate game for the twelve cards.
    pub game: GameType,
    /// Branch of the game choice.
    pub branch: GameBranch,
    /// The two cards Easy puts away, with the reason for each.
    pub reasons: [(Card, Keep); 2],
    /// Every card ranked no higher than the second discard (ties included):
    /// naming one of these is a correct answer.
    pub eligible: CardSet,
    /// Easy's wanted action was illegal and the first legal action was taken.
    pub fallback: bool,
}

/// Easy's discard rule; none outside Drücken or without a turn.
pub fn discard_rule(view: &PlayerView) -> Option<DiscardRule> {
    let legal = &view.legal_actions;
    if view.phase != Phase::Druecken || legal.is_empty() {
        return None;
    }
    let cards = view.hand;
    let (game, _, branch) = choose_game_branch(cards);
    let put = discard(cards);
    let wanted = Action::Discard(put);
    let fallback = !legal.contains(&wanted);
    let action = if fallback { legal[0] } else { wanted };
    let put = match action {
        Action::Discard(c) => c,
        _ => put,
    };
    let limit = put
        .iter()
        .map(|c| keep_score(game, cards, c))
        .max()
        .unwrap_or(i32::MIN);
    let mut it = put.iter().map(|c| (c, keep_class(game, cards, c)));
    let first = it.next()?;
    Some(DiscardRule {
        action,
        game,
        branch,
        reasons: [first, it.next()?],
        eligible: cards
            .iter()
            .filter(|&c| keep_score(game, cards, c) <= limit)
            .collect(),
        fallback,
    })
}

fn keep_class(game: GameType, cards: CardSet, card: Card) -> Keep {
    if game == GameType::Null {
        return Keep::Null;
    }
    if game.is_trump(card) {
        return Keep::Trumpf;
    }
    let suit = cards & game.cards_of(game.follow(card));
    match card.augen() {
        11 => Keep::Ass,
        10 if suit.iter().any(|c| c.augen() == 11) => Keep::GuardedTen,
        _ => Keep::Side {
            suit: suit.len() as u8,
        },
    }
}

/// Higher means rather keep.
fn keep_score(game: GameType, cards: CardSet, card: Card) -> i32 {
    if game == GameType::Null {
        return -power(game, card);
    }
    if game.is_trump(card) {
        return 1000 + power(game, card);
    }
    let suit = cards & game.cards_of(game.follow(card));
    let has_ass = suit.iter().any(|c| c.augen() == 11);
    match card.augen() {
        11 => 500,
        10 if has_ass => 400,
        augen => suit.len() as i32 * 20 - i32::from(augen),
    }
}

/// One game of Easy's fallback candidate group (suit games and Grand).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnnounceCandidate {
    /// Suit game or Grand.
    pub game: GameType,
    /// Plain value with Spitzen from the declarer's known cards (Hand level
    /// included in a Hand game).
    pub value: u16,
    /// Strength of the declarer's known cards for `game`.
    pub strength: i32,
    /// The value reaches the public bid.
    pub fits: bool,
}

/// Exact inputs and outcome of Easy's announcement rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnnounceRule {
    /// Actual selected action: an announcement out of the legal actions.
    pub action: Action,
    /// Public bid the game must reach.
    pub bid: u16,
    /// Public: the declarer plays Hand.
    pub hand_game: bool,
    /// Own known Skat counted with the ten cards (the discard); empty in a
    /// Hand game, where the Skat stays unknown.
    pub skat: CardSet,
    /// Preferred game of [`choose_game`] for the ten cards plus `skat`.
    pub preferred: GameType,
    /// Branch of the preferred game choice.
    pub branch: GameBranch,
    /// Plain value of the preferred game (Null: 23, Null Hand 35).
    pub value: u16,
    /// The preferred game's value reaches the bid; then it is announced.
    pub fits: bool,
    /// Otherwise: suit games and Grand in tie order; the fitting one with the
    /// highest strength is announced, the later one on equal strength.
    pub candidates: [AnnounceCandidate; 5],
    /// Game of the announcement.
    pub game: GameType,
    /// Null: unsafe suits among the ten cards; none means Null ouvert.
    pub null_risks: Option<u8>,
    /// Easy wants Null ouvert (Null without an unsafe suit).
    pub ouvert: bool,
    /// No candidate's plain value from the known cards reaches the bid: Easy
    /// falls back to Grand. The actual game value stays unknown here (Spitzen
    /// in a hidden Hand Skat, earned levels), so this is not an overbid claim.
    pub no_fit: bool,
    /// Wanted Null ouvert was not legal: plain Null instead.
    pub plain: bool,
    /// The rule's announcement was not legal; the first legal action is taken.
    pub fallback: bool,
}

impl AnnounceRule {
    /// Fitting candidates sharing the announced game's strength when the
    /// preferred game did not fit (the tie-break picks the last one).
    pub fn tied(&self) -> Vec<GameType> {
        if self.fits || self.no_fit {
            return Vec::new();
        }
        let top = self.candidates.iter().find(|c| c.game == self.game);
        let tied: Vec<_> = self
            .candidates
            .iter()
            .filter(|c| c.fits && top.is_some_and(|t| t.strength == c.strength))
            .map(|c| c.game)
            .collect();
        if tied.len() > 1 { tied } else { Vec::new() }
    }
}

fn plain_value(game: GameType, cards: CardSet, hand_game: bool) -> u16 {
    Contract::new(Announcement::plain(game), hand_game)
        .null_value()
        .or_else(|| game_value(game, cards, hand_game))
        .unwrap_or(0)
}

/// Easy's announcement rule for the declarer's ten cards `hand` plus the
/// Skat it knows (`skat`: the discard, empty in a Hand game). `action` is the
/// rule's choice before the legal-action fallback.
fn announce_choice(
    hand: CardSet,
    skat: CardSet,
    bid: u16,
    hand_game: bool,
    legal: &[Action],
) -> AnnounceRule {
    let cards = hand | skat;
    let (preferred, _, branch) = choose_game_branch(cards);
    let value = plain_value(preferred, cards, hand_game);
    let fits = value >= bid;
    let candidates = TRUMP_GAMES.map(|game| {
        let value = plain_value(game, cards, hand_game);
        AnnounceCandidate {
            game,
            value,
            strength: strength(game, cards),
            fits: value >= bid,
        }
    });
    let best = candidates
        .iter()
        .filter(|c| c.fits)
        .max_by_key(|c| c.strength)
        .map(|c| c.game);
    let no_fit = !fits && best.is_none();
    let game = if fits {
        preferred
    } else {
        best.unwrap_or(GameType::Grand)
    };
    let null_risks = (game == GameType::Null).then(|| null_risks(hand) as u8);
    // A Null hand without any unsafe suit is played open.
    let ouvert = null_risks == Some(0);
    let plain = Action::Announce(Announcement::plain(game));
    let wanted = Action::Announce(Announcement {
        ouvert,
        ..Announcement::plain(game)
    });
    let unavailable = ouvert && !legal.contains(&wanted);
    AnnounceRule {
        action: if unavailable { plain } else { wanted },
        bid,
        hand_game,
        skat,
        preferred,
        branch,
        value,
        fits,
        candidates,
        game,
        null_risks,
        ouvert,
        no_fit,
        plain: unavailable,
        fallback: false,
    }
}

/// Announcement for the declarer's ten cards `hand` plus the Skat it knows
/// (`skat`: the discard, empty in a Hand game), out of `legal`.
pub fn announce(
    hand: CardSet,
    skat: CardSet,
    bid: u16,
    hand_game: bool,
    legal: &[Action],
) -> Action {
    announce_choice(hand, skat, bid, hand_game, legal).action
}

/// Easy's announcement rule from the permitted view (own cards, own known
/// Skat, public bid and Hand state); none outside Ansage or without a turn.
pub fn announce_rule(view: &PlayerView) -> Option<AnnounceRule> {
    let legal = &view.legal_actions;
    if view.phase != Phase::Ansage || legal.is_empty() {
        return None;
    }
    let mut rule = announce_choice(
        view.hand,
        view.own_skat.unwrap_or_default(),
        view.bid,
        view.hand_game == Some(true),
        legal,
    );
    if !legal.contains(&rule.action) {
        rule.action = legal[0];
        rule.fallback = true;
    }
    Some(rule)
}

/// Whether a defender holding `hand` says Kontra against `game`.
pub fn wants_kontra(game: GameType, hand: CardSet) -> bool {
    game != GameType::Null && trump_count(game, hand) >= KONTRA_TRUMPS
}

/// Whether the declarer with `cards` (hand plus own Skat) answers Kontra
/// with Re.
pub fn wants_re(game: GameType, cards: CardSet) -> bool {
    strength(game, cards) >= RE
}
