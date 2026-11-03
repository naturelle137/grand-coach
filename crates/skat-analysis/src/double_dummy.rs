// SPDX-License-Identifier: EUPL-1.2

//! Double-dummy evaluation: the exact result of a fully specified position
//! when every seat sees every card and plays perfectly.
//!
//! Positions are hypothetical simulation inputs (sampled deals). Normal
//! analysis never builds one from the real hidden cards; the only bridge
//! from a real game is the explicitly named [`Position::from_omniscient`]
//! for open mode and post-game analysis (D-23).
//!
//! # Objectives
//!
//! The declarer maximises and the defenders minimise the declarer's game
//! points as computed by [`skat_core::settle`]. For a fixed contract these
//! points only depend on the outcome class (no trick, at most 30 Augen,
//! 31–60, 61–89, at least 90, every trick) and never decrease from one class
//! to the next. Minimax commutes with such a monotone map, so the solver
//! finds the class from searches that do not depend on the contract:
//!
//! - two null-window alpha-beta searches with a transposition table: can
//!   the declarer force 61, then 90 or 31 Augen,
//! - whether the declarer can force every trick (only if it can force 90),
//! - whether the defenders can keep the declarer without a trick (only if
//!   the declarer cannot force more than 30).
//!
//! The exact Augen total is available separately ([`Solver::exact_augen`]);
//! it costs a full-window search and is not needed for scoring.
//!
//! Null has its own objective: the declarer wins if it can avoid every
//! trick, which is the last search alone.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use skat_core::{
    Card, CardSet, Contract, GameOutcome, GameResult, GameType, KontraLevel, OmniscientView, Seat,
    Spitzen, Trick, legal_cards, settle, trick_winner,
};

use crate::budget::{Interrupted, Meter};

/// A fully specified card-play position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position {
    /// Game type being played.
    pub game: GameType,
    /// Declarer.
    pub declarer: Seat,
    /// Current cards of every seat (index by [`Seat::index`]).
    pub hands: [CardSet; 3],
    /// The Skat as it counts for the declarer (the discard after pickup).
    pub skat: CardSet,
    /// Trick in progress; `leader: None` means Vorhand leads the first trick.
    pub trick: Trick,
    /// Augen the declarer won in completed tricks, without the Skat.
    pub declarer_augen: u8,
    /// Tricks the declarer won so far.
    pub declarer_tricks: u8,
    /// Completed tricks.
    pub tricks_played: u8,
}

/// Why a [`Position`] cannot occur in a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PositionError {
    /// A card is in two places.
    CardTwice(Card),
    /// Hand sizes do not fit the number of tricks and the trick in progress.
    HandSizes,
    /// The Skat does not hold two cards.
    SkatSize,
    /// A card in the trick in progress breaks the Bedienen rule as far as
    /// the hands show (the seat still holds a card it had to follow with).
    MustFollowSuit(Card),
    /// Trick counts or Augen are out of range.
    Counts,
}

/// Tricks the declarer gets, as far as the outcome depends on them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TrickClass {
    /// No trick at all.
    None,
    /// At least one trick but not all.
    Some,
    /// Every trick.
    All,
}

/// Range of the declarer's final Augen (with the Skat) that scoring tells
/// apart (ISkO 5.2: 61 wins, 90 Schneider, 30 or fewer Schneider lost).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AugenClass {
    /// 0 to 30: Schneider against the declarer.
    UpTo30,
    /// 31 to 60: lost.
    UpTo60,
    /// 61 to 89: won.
    UpTo89,
    /// 90 to 120: won Schneider.
    From90,
}

impl AugenClass {
    /// Class of a final Augen total.
    pub fn of(augen: u8) -> AugenClass {
        match augen {
            0..=30 => AugenClass::UpTo30,
            31..=60 => AugenClass::UpTo60,
            61..=89 => AugenClass::UpTo89,
            _ => AugenClass::From90,
        }
    }

    /// Lowest total of the class.
    pub fn min(self) -> u8 {
        match self {
            AugenClass::UpTo30 => 0,
            AugenClass::UpTo60 => 31,
            AugenClass::UpTo89 => 61,
            AugenClass::From90 => 90,
        }
    }
}

/// Double-dummy value of a position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DdValue {
    /// Class of the declarer's final Augen including the Skat under perfect
    /// play; `None` for Null, where Augen do not matter.
    pub augen: Option<AugenClass>,
    /// Tricks of the declarer under perfect play (for the class that
    /// matters: forcing every trick or avoiding every trick).
    pub tricks: TrickClass,
}

/// What a game is played for, besides the cards: needed to turn a
/// [`DdValue`] into game points via [`skat_core::settle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stakes {
    /// The contract.
    pub contract: Contract,
    /// Final bid.
    pub bid: u16,
    /// Spitzen of the declarer's ten cards plus Skat; `None` for Null.
    pub spitzen: Option<Spitzen>,
    /// Kontra/Re level.
    pub kontra: KontraLevel,
}

impl DdValue {
    /// The settled game for this value (ISkO scoring through `skat-core`).
    pub fn settle(&self, declarer: Seat, stakes: &Stakes) -> GameResult {
        let (declarer_tricks, tricks_played) = match self.tricks {
            TrickClass::None => (0, 10),
            // Any count strictly between 0 and 10 settles the same way; a lost
            // Null ends with the declarer's first trick.
            TrickClass::Some if stakes.contract.game == GameType::Null => (1, 1),
            TrickClass::Some => (5, 10),
            TrickClass::All => (10, 10),
        };
        settle(&GameOutcome {
            declarer,
            contract: stakes.contract,
            bid: stakes.bid,
            spitzen: stakes.spitzen,
            // Settling only compares with 30, 61 and 90.
            declarer_augen: match self.tricks {
                TrickClass::All => 120,
                _ => self.augen.map_or(0, AugenClass::min),
            },
            declarer_tricks,
            tricks_played,
            kontra: stakes.kontra,
        })
    }
}

impl Position {
    /// Seat to play next.
    pub fn to_move(&self) -> Seat {
        let leader = self.trick.leader.unwrap_or(Seat::Vorhand);
        Seat::from_index(leader.index() + self.trick.len())
    }

    /// Cards the seat to move may play.
    pub fn legal_cards(&self) -> CardSet {
        let seat = self.to_move();
        legal_cards(self.game, self.hands[seat.index()], self.trick.led())
    }

    /// `true` when no card is left to play, or a Null game is decided.
    pub fn is_over(&self) -> bool {
        let null_lost = self.game == GameType::Null && self.declarer_tricks > 0;
        null_lost || (self.hands.iter().all(|h| h.is_empty()) && self.trick.is_empty())
    }

    /// Plays `card` for the seat to move (no legality check; see
    /// [`Position::legal_cards`]).
    pub fn play(&mut self, card: Card) {
        let seat = self.to_move();
        self.hands[seat.index()].remove(card);
        let n = self.trick.len();
        if n == 0 {
            self.trick.leader = Some(seat);
        }
        self.trick.cards[n] = Some(card);
        if n < 2 {
            return;
        }
        let leader = self.trick.leader.expect("trick has a leader");
        let cards = self.trick.cards.map(|c| c.expect("complete trick"));
        let winner = Seat::from_index(leader.index() + trick_winner(self.game, cards));
        if winner == self.declarer {
            self.declarer_augen += cards.iter().map(|c| c.augen()).sum::<u8>();
            self.declarer_tricks += 1;
        }
        self.tricks_played += 1;
        self.trick = Trick {
            leader: Some(winner),
            cards: [None; 3],
        };
    }

    /// Checks that the position can occur in a game.
    pub fn check(&self) -> Result<(), PositionError> {
        let mut seen = CardSet::EMPTY;
        let table: CardSet = self.trick.cards.iter().flatten().copied().collect();
        for part in [
            self.hands[0],
            self.hands[1],
            self.hands[2],
            self.skat,
            table,
        ] {
            if let Some(card) = (seen & part).iter().next() {
                return Err(PositionError::CardTwice(card));
            }
            seen |= part;
        }
        if self.skat.len() != 2 {
            return Err(PositionError::SkatSize);
        }
        if self.tricks_played > 10
            || self.declarer_tricks > self.tricks_played
            || self.declarer_augen > 120
        {
            return Err(PositionError::Counts);
        }
        let left = 10 - usize::from(self.tricks_played);
        let leader = self.trick.leader.unwrap_or(Seat::Vorhand);
        for (i, card) in self.trick.cards.iter().enumerate() {
            let seat = Seat::from_index(leader.index() + i);
            let size = self.hands[seat.index()].len();
            let expected = left - usize::from(card.is_some());
            if size != expected {
                return Err(PositionError::HandSizes);
            }
            if let (Some(card), Some(led)) = (card, self.trick.led())
                && i > 0
            {
                let hand = self.hands[seat.index()] | CardSet::single(*card);
                if !legal_cards(self.game, hand, Some(led)).contains(*card) {
                    return Err(PositionError::MustFollowSuit(*card));
                }
            }
        }
        if self.trick.leader.is_none() && !self.trick.is_empty() {
            return Err(PositionError::HandSizes);
        }
        Ok(())
    }

    /// The real position of a game in card play, from full knowledge. Only
    /// for open mode and post-game analysis (D-23); `None` outside card
    /// play or while the declarer still holds twelve cards.
    pub fn from_omniscient(view: &OmniscientView) -> Option<Position> {
        let seat_view = &view.seats[0];
        let declarer = seat_view.declarer?;
        let contract = seat_view.contract?;
        if seat_view.phase != skat_core::Phase::Spiel || view.skat.len() != 2 {
            return None;
        }
        let won = seat_view.tricks.iter().filter(|t| t.winner == declarer);
        Some(Position {
            game: contract.game,
            declarer,
            hands: view.hands,
            skat: view.skat,
            trick: seat_view.current_trick,
            declarer_augen: won.clone().map(|t| t.augen()).sum(),
            declarer_tricks: won.count() as u8,
            tricks_played: seat_view.tricks.len() as u8,
        })
    }
}

/// Cards on the table in the trick in progress.
fn table(pos: &Position) -> CardSet {
    pos.trick.cards.iter().flatten().copied().collect()
}

/// Augen of a card set by rank masks (hot path of the search).
fn fast_augen(cards: CardSet) -> u8 {
    const RANK: u32 = 0x0101_0101;
    let count = |rank: skat_core::Rank| (cards.bits() & (RANK << rank.index())).count_ones() as u8;
    use skat_core::Rank::{Ass, Bube, Dame, Koenig, Zehn};
    11 * count(Ass) + 10 * count(Zehn) + 4 * count(Koenig) + 3 * count(Dame) + 2 * count(Bube)
}

/// Minimum Augen contributed by any n cards of a hand.
fn cheapest_points(hand: CardSet, n: usize) -> u8 {
    let bits = hand.bits();
    let mut needed = n.saturating_sub((bits & 0x07070707).count_ones() as usize);
    let mut points = 0;
    for rank in [
        skat_core::Rank::Bube,
        skat_core::Rank::Dame,
        skat_core::Rank::Koenig,
        skat_core::Rank::Zehn,
        skat_core::Rank::Ass,
    ] {
        if needed == 0 {
            break;
        }
        let take = needed.min((bits & (0x01010101 << rank.index())).count_ones() as usize);
        points += take as u8 * rank.augen();
        needed -= take;
    }
    points
}

/// Moves of one seat (at most twelve cards) without allocation.
struct Moves {
    cards: [Card; 12],
    len: usize,
}

impl Default for Moves {
    fn default() -> Moves {
        Moves {
            cards: [Card::new(skat_core::Suit::Kreuz, skat_core::Rank::Sieben); 12],
            len: 0,
        }
    }
}

impl Moves {
    fn push(&mut self, card: Card) {
        self.cards[self.len] = card;
        self.len += 1;
    }

    fn as_slice(&self) -> &[Card] {
        &self.cards[..self.len]
    }

    fn as_mut_slice(&mut self) -> &mut [Card] {
        &mut self.cards[..self.len]
    }
}

/// Multiplicative hasher for the transposition tables (keys are already
/// well mixed card bits).
#[derive(Default)]
struct FastHasher(u64);

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(b)).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }

    fn write_u128(&mut self, n: u128) {
        let mixed = (n as u64) ^ ((n >> 64) as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.0 = (self.0 ^ mixed).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        self.0 ^= self.0 >> 29;
    }
}

type Table<V> = HashMap<u128, V, BuildHasherDefault<FastHasher>>;

/// Entries after which a table is cleared to bound memory.
const TABLE_LIMIT: usize = 1 << 18;

/// Goal of a yes/no search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Goal {
    /// The declarer wins no further trick.
    DeclarerNoTrick,
    /// The declarer wins every further trick.
    DeclarerAllTricks,
}

/// Double-dummy solver for one game type and declarer. Reuse it for several
/// positions (also of different deals): its tables are keyed by the full
/// card distribution.
pub struct Solver {
    extra_bound: bool,
    game: GameType,
    declarer: Seat,
    /// Follow class (0–3 side suits, 4 Trumpf) of every card index.
    class: [u8; 32],
    /// Cards of each follow class, strongest first.
    order: [Vec<Card>; 5],
    /// Cards of each follow class as a set.
    class_cards: [CardSet; 5],
    /// Bounds of the declarer's future Augen and the best card found.
    augen: Table<(u8, u8, Option<Card>)>,
    no_trick: Table<bool>,
    all_tricks: Table<bool>,
}

impl Solver {
    /// A solver for `game` with `declarer`.
    pub fn new(game: GameType, declarer: Seat) -> Solver {
        let mut class = [0u8; 32];
        let mut order: [Vec<Card>; 5] = Default::default();
        for card in CardSet::ALL {
            let c = if game.is_trump(card) {
                4
            } else {
                card.suit().index()
            };
            class[usize::from(card.index())] = c;
            order[usize::from(c)].push(card);
        }
        for cards in &mut order {
            // Within one follow class `beats` with the card itself led is the
            // plain strength order.
            cards.sort_by(|&a, &b| {
                if a == b {
                    std::cmp::Ordering::Equal
                } else if game.beats(a, b, a) {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            });
        }
        let class_cards = order.clone().map(|cards| cards.into_iter().collect());
        Solver {
            extra_bound: false,
            game,
            declarer,
            class,
            order,
            class_cards,
            augen: Table::default(),
            no_trick: Table::default(),
            all_tricks: Table::default(),
        }
    }

    /// Enables the experimental lower bound on the defenders' contribution.
    pub fn with_discard_bound(mut self) -> Self {
        self.extra_bound = true;
        self
    }

    /// Solves `pos` (which must match the solver's game and declarer and
    /// pass [`Position::check`]).
    pub fn solve(&mut self, pos: &Position, meter: &mut Meter) -> Result<DdValue, Interrupted> {
        assert!(
            pos.game == self.game && pos.declarer == self.declarer,
            "solver used for another game or declarer"
        );
        for table_len in [self.augen.len(), self.no_trick.len(), self.all_tricks.len()] {
            if table_len > TABLE_LIMIT {
                self.augen.clear();
                self.no_trick.clear();
                self.all_tricks.clear();
            }
        }
        let mut p = *pos;
        if self.game == GameType::Null {
            let clean = pos.declarer_tricks == 0;
            let safe = clean && self.forced(&mut p, Goal::DeclarerNoTrick, meter)?;
            let tricks = if safe {
                TrickClass::None
            } else {
                TrickClass::Some
            };
            return Ok(DdValue {
                augen: None,
                tricks,
            });
        }
        let base = pos.declarer_augen + pos.skat.augen();
        let class = if self.reaches(&mut p, base, 61, meter)? {
            if self.reaches(&mut p, base, 90, meter)? {
                AugenClass::From90
            } else {
                AugenClass::UpTo89
            }
        } else if self.reaches(&mut p, base, 31, meter)? {
            AugenClass::UpTo60
        } else {
            AugenClass::UpTo30
        };
        let tricks = if class == AugenClass::From90 && pos.declarer_tricks == pos.tricks_played {
            if self.forced(&mut p, Goal::DeclarerAllTricks, meter)? {
                TrickClass::All
            } else {
                TrickClass::Some
            }
        } else if class == AugenClass::UpTo30 && pos.declarer_tricks == 0 {
            if self.forced(&mut p, Goal::DeclarerNoTrick, meter)? {
                TrickClass::None
            } else {
                TrickClass::Some
            }
        } else {
            // More than 30 Augen need a trick: the Skat holds at most 21.
            TrickClass::Some
        };
        Ok(DdValue {
            augen: Some(class),
            tricks,
        })
    }

    /// Whether a suit or Grand declarer can force the given final Augen total.
    /// The position must match this solver, as for [`Solver::solve`].
    pub fn reaches_augen(
        &mut self,
        pos: &Position,
        target: u8,
        meter: &mut Meter,
    ) -> Result<bool, Interrupted> {
        assert!(
            self.game != GameType::Null && pos.game == self.game && pos.declarer == self.declarer
        );
        let mut p = *pos;
        self.reaches(&mut p, pos.declarer_augen + pos.skat.augen(), target, meter)
    }

    /// Exact final Augen of the declarer (with the Skat) under perfect play
    /// by both sides; a full-window search, slower than [`Solver::solve`].
    pub fn exact_augen(&mut self, pos: &Position, meter: &mut Meter) -> Result<u8, Interrupted> {
        let mut p = *pos;
        let future = self.max_augen(&mut p, 0, 120, meter)?;
        Ok(pos.declarer_augen + pos.skat.augen() + future)
    }

    /// Whether the declarer can force a final total of `target` Augen when
    /// it already has `base`.
    fn reaches(
        &mut self,
        pos: &mut Position,
        base: u8,
        target: u8,
        meter: &mut Meter,
    ) -> Result<bool, Interrupted> {
        let Some(needed) = target.checked_sub(base).filter(|&n| n > 0) else {
            return Ok(true);
        };
        Ok(self.max_augen(pos, needed - 1, needed, meter)? >= needed)
    }

    fn key(pos: &Position) -> u128 {
        let leader = pos.trick.leader.unwrap_or(Seat::Vorhand).index() as u128;
        u128::from(pos.hands[0].bits())
            | u128::from(pos.hands[1].bits()) << 32
            | u128::from(pos.hands[2].bits()) << 64
            | leader << 96
    }

    /// Legal moves of the seat to move, keeping one card of each group of
    /// equivalent cards: neighbours in strength with nothing in between that
    /// another seat holds or that lies on the table (and, when Augen count,
    /// with equal Augen).
    fn moves(&self, pos: &Position, care_augen: bool) -> Moves {
        let seat = pos.to_move();
        let legal = pos.legal_cards();
        let blockers =
            (pos.hands[0] | pos.hands[1] | pos.hands[2] | table(pos)) - pos.hands[seat.index()];
        let mut moves = Moves::default();
        for (c, order) in self.order.iter().enumerate() {
            if (legal & self.class_cards[c]).is_empty() {
                continue;
            }
            let mut last: Option<Card> = None;
            let mut blocked = false;
            for &card in order {
                if legal.contains(card) {
                    let same = last.is_some_and(|l| !care_augen || l.augen() == card.augen());
                    if !same || blocked {
                        moves.push(card);
                        last = Some(card);
                    }
                    blocked = false;
                } else if blockers.contains(card) {
                    blocked = true;
                }
            }
        }
        moves
    }

    /// Tricks the declarer, on lead at the start of a trick, wins for sure
    /// by cashing masters, and the Augen of its own cards in them: the top
    /// run of Trumpf, plus the top run of each side suit once that run of
    /// Trumpf draws every defender's trumps. A lower bound for search.
    fn sure_tricks(&self, pos: &Position) -> (usize, u8) {
        let own = pos.hands[self.declarer.index()];
        let all = pos.hands[0] | pos.hands[1] | pos.hands[2];
        let run = |order: &[Card]| {
            let mut cards = CardSet::EMPTY;
            for &card in order.iter().filter(|&&c| all.contains(c)) {
                if !own.contains(card) {
                    break;
                }
                cards.insert(card);
            }
            cards
        };
        let trumps = run(&self.order[4]);
        let defenders = (all - own) & self.class_cards[4];
        let most = Seat::ALL
            .iter()
            .filter(|&&s| s != self.declarer)
            .map(|s| (pos.hands[s.index()] & defenders).len())
            .max()
            .unwrap_or(0);
        let mut sure = trumps;
        if most <= trumps.len() {
            for order in &self.order[..4] {
                sure |= run(order);
            }
        }
        (sure.len(), fast_augen(sure))
    }

    /// `true` at the start of a trick the declarer leads (suit and Grand).
    fn declarer_leads(&self, pos: &Position) -> bool {
        self.game != GameType::Null && pos.trick.is_empty() && pos.to_move() == self.declarer
    }

    /// Most Augen the declarer can force from now on (fail-soft alpha-beta
    /// within `alpha..beta`), counting cards on the table when their trick
    /// completes.
    fn max_augen(
        &mut self,
        pos: &mut Position,
        mut alpha: u8,
        mut beta: u8,
        meter: &mut Meter,
    ) -> Result<u8, Interrupted> {
        meter.tick()?;
        if pos.is_over() || beta == 0 {
            return Ok(0);
        }
        let remaining = fast_augen(pos.hands[0] | pos.hands[1] | pos.hands[2] | table(pos));
        if remaining <= alpha {
            return Ok(remaining);
        }
        if self.declarer_leads(pos) {
            let (tricks, augen) = self.sure_tricks(pos);
            if tricks == pos.hands[self.declarer.index()].len() {
                return Ok(remaining);
            }
            // Each guaranteed trick also consumes one card from each defender.
            let mut augen = augen;
            if self.extra_bound && tricks > 0 && augen < beta {
                for seat in Seat::ALL {
                    if seat != self.declarer {
                        augen += cheapest_points(pos.hands[seat.index()], tricks);
                    }
                }
            }
            if augen >= beta {
                return Ok(augen);
            }
        }
        let at_start = pos.trick.is_empty();
        let key = Self::key(pos);
        let mut hint = None;
        if at_start && let Some(&(lo, hi, best)) = self.augen.get(&key) {
            hint = best;
            if lo >= beta || lo == hi {
                return Ok(lo);
            }
            if hi <= alpha {
                return Ok(hi);
            }
            alpha = alpha.max(lo);
            beta = beta.min(hi);
        }
        let (alpha0, beta0) = (alpha, beta);
        let maximise = pos.to_move() == self.declarer;
        let mut moves = self.moves(pos, true);
        self.order_moves(pos, moves.as_mut_slice());
        if let Some(i) = hint.and_then(|h| moves.as_slice().iter().position(|&c| c == h)) {
            moves.as_mut_slice()[..=i].rotate_right(1);
        }
        let mut best: Option<u8> = None;
        let mut best_card = None;
        for &card in moves.as_slice() {
            let mut child = *pos;
            let before = child.declarer_augen;
            child.play(card);
            let gained = child.declarer_augen - before;
            let a = alpha.saturating_sub(gained);
            let b = beta.saturating_sub(gained);
            let value = gained + self.max_augen(&mut child, a, b.max(a + 1), meter)?;
            let better = best.is_none_or(|v| if maximise { value > v } else { value < v });
            if better {
                best = Some(value);
                best_card = Some(card);
            }
            if maximise {
                alpha = alpha.max(value);
            } else {
                beta = beta.min(value);
            }
            if alpha >= beta {
                break;
            }
        }
        let value = best.expect("a seat with cards has a legal card");
        if at_start {
            let entry = self.augen.entry(key).or_insert((0, 120, None));
            entry.2 = best_card;
            if value <= alpha0 {
                entry.1 = entry.1.min(value);
            } else if value >= beta0 {
                entry.0 = entry.0.max(value);
            } else {
                (entry.0, entry.1) = (value, value);
            }
        }
        Ok(value)
    }

    /// Move ordering: when leading, high Augen first; when following and the
    /// own side wins the trick, smear high Augen; otherwise the cheapest card
    /// that takes the trick, then the lowest losers.
    fn order_moves(&self, pos: &Position, moves: &mut [Card]) {
        let strength = |c: Card| {
            let order = &self.order[usize::from(self.class[usize::from(c.index())])];
            order.iter().position(|&o| o == c).unwrap_or(0)
        };
        let Some(led) = pos.trick.led() else {
            moves.sort_by_key(|&c| (std::cmp::Reverse(c.augen()), strength(c)));
            return;
        };
        let leader = pos.trick.leader.expect("trick has a leader");
        let (mut best, mut best_seat) = (led, leader);
        for (i, card) in pos.trick.cards.iter().enumerate().skip(1) {
            if let Some(card) = *card
                && self.game.beats(card, best, led)
            {
                best = card;
                best_seat = Seat::from_index(leader.index() + i);
            }
        }
        let me = pos.to_move();
        let own_side = (best_seat == self.declarer) == (me == self.declarer);
        moves.sort_by_key(|&c| {
            let wins = self.game.beats(c, best, led);
            if own_side {
                (u8::from(wins), 0, 255 - c.augen())
            } else if wins {
                (0, 255 - strength(c) as u8, c.augen())
            } else {
                (1, 0, c.augen())
            }
        });
    }

    /// Whether the side that wants `goal` can force it from here on (the
    /// declarer wants it for [`Goal::DeclarerAllTricks`] and in Null, the
    /// defenders otherwise).
    fn forced(
        &mut self,
        pos: &mut Position,
        goal: Goal,
        meter: &mut Meter,
    ) -> Result<bool, Interrupted> {
        meter.tick()?;
        if pos.hands.iter().all(|h| h.is_empty()) && pos.trick.is_empty() {
            return Ok(true);
        }
        if self.declarer_leads(pos) {
            let (tricks, _) = self.sure_tricks(pos);
            match goal {
                Goal::DeclarerAllTricks if tricks == pos.hands[self.declarer.index()].len() => {
                    return Ok(true);
                }
                Goal::DeclarerNoTrick if tricks > 0 => return Ok(false),
                _ => {}
            }
        }
        let at_start = pos.trick.is_empty();
        let key = Self::key(pos);
        let table = match goal {
            Goal::DeclarerNoTrick => &self.no_trick,
            Goal::DeclarerAllTricks => &self.all_tricks,
        };
        if at_start && let Some(&known) = table.get(&key) {
            return Ok(known);
        }
        let declarer_wants = goal == Goal::DeclarerAllTricks || self.game == GameType::Null;
        let wanting = (pos.to_move() == self.declarer) == declarer_wants;
        let mut result = !wanting;
        for &card in self.moves(pos, false).as_slice() {
            let mut child = *pos;
            let tricks = child.declarer_tricks;
            let played = child.tricks_played;
            child.play(card);
            let broken = child.tricks_played > played
                && match goal {
                    Goal::DeclarerNoTrick => child.declarer_tricks > tricks,
                    Goal::DeclarerAllTricks => child.declarer_tricks == tricks,
                };
            let ok = !broken && self.forced(&mut child, goal, meter)?;
            if ok == wanting {
                result = wanting;
                break;
            }
        }
        if at_start {
            let table = match goal {
                Goal::DeclarerNoTrick => &mut self.no_trick,
                Goal::DeclarerAllTricks => &mut self.all_tricks,
            };
            table.insert(key, result);
        }
        Ok(result)
    }
}

/// Solves one position with a fresh solver.
pub fn solve(pos: &Position, meter: &mut Meter) -> Result<DdValue, Interrupted> {
    Solver::new(pos.game, pos.declarer).solve(pos, meter)
}
