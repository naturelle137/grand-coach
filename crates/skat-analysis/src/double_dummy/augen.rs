// SPDX-License-Identifier: EUPL-1.2

//! The Augen search numbers cards in follow-class order, strongest first.
//! Each follow class occupies one contiguous run of bits. The public
//! position and the rule engine retain their usual card representation.

use super::{Interrupted, Meter, Position, Table};
use skat_core::{Card, CardSet, GameType, Seat, Suit};
use std::sync::OnceLock;

#[cfg(feature = "endgame")]
pub mod endgame;

const NO_CARD: u8 = 32;

struct Cards {
    index: [u8; 32],
    suit: [u32; 32],
    points: [u8; 32],
    trump: u32,
    suits: [u32; 5],
}

impl Cards {
    fn for_game(game: GameType, order: &[Vec<Card>; 5]) -> &'static Self {
        static TABLES: [OnceLock<Cards>; 6] = [const { OnceLock::new() }; 6];
        let index = match game {
            GameType::Farbe(s) => usize::from(s.index()),
            GameType::Grand => Suit::ALL.len(),
            GameType::Null => 5,
        };
        TABLES[index].get_or_init(|| {
            let mut t = Self {
                index: [0; 32],
                suit: [0; 32],
                points: [0; 32],
                trump: 0,
                suits: [0; 5],
            };
            let mut i = 0;
            for (class, cards) in order.iter().enumerate() {
                let start = i;
                for card in cards {
                    t.index[usize::from(card.index())] = i as u8;
                    t.points[i] = card.augen();
                    t.suits[class] |= 1 << i;
                    i += 1;
                }
                t.suit[start..i].fill(t.suits[class]);
            }
            t.trump = t.suits[4];
            t
        })
    }

    fn encode(&self, hand: CardSet) -> u32 {
        hand.into_iter()
            .fold(0, |bits, c| bits | 1 << self.index[usize::from(c.index())])
    }

    fn beats(&self, card: u8, best: u8) -> bool {
        let suit = self.suit[usize::from(card)];
        let best_suit = self.suit[usize::from(best)];
        if suit == best_suit {
            card < best
        } else {
            suit == self.trump
        }
    }
}

#[derive(Clone, Copy)]
struct Node {
    hands: [u32; 3],
    // Includes cards on the table until the trick is complete.
    live: u32,
    remaining: u8,
    turn: u8,
    count: u8,
    led: u8,
    best: u8,
    winner: u8,
    table_points: u8,
    table: u32,
}

impl Node {
    fn new(p: &Position, cards: &Cards) -> Self {
        let hands = p.hands.map(|h| cards.encode(h));
        let mut n = Self {
            hands,
            live: hands[0] | hands[1] | hands[2],
            remaining: p.hands.iter().map(|h| h.augen()).sum(),
            turn: p.to_move().index() as u8,
            count: 0,
            led: 0,
            best: 0,
            winner: 0,
            table_points: 0,
            table: 0,
        };
        let leader = p.trick.leader.unwrap_or(Seat::Vorhand);
        for (i, card) in p.trick.cards.iter().flatten().enumerate() {
            let c = cards.index[usize::from(card.index())];
            n.take(c, Seat::from_index(leader.index() + i).index() as u8, cards);
            n.live |= 1 << c;
            n.remaining += card.augen();
        }
        n
    }

    fn take(&mut self, c: u8, seat: u8, cards: &Cards) {
        if self.count == 0 {
            self.led = c;
        }
        if self.count == 0 || cards.beats(c, self.best) {
            self.best = c;
            self.winner = seat;
        }
        self.table |= 1 << c;
        self.table_points += cards.points[usize::from(c)];
        self.count += 1;
    }

    fn play(&mut self, c: u8, cards: &Cards, declarer: u8) -> u8 {
        let turn = self.turn;
        self.hands[usize::from(turn)] ^= 1 << c;
        self.take(c, turn, cards);
        if self.count == 3 {
            let gain = if self.winner == declarer {
                self.table_points
            } else {
                0
            };
            self.turn = self.winner;
            self.remaining -= self.table_points;
            self.live ^= self.table;
            self.table = 0;
            self.table_points = 0;
            self.count = 0;
            gain
        } else {
            self.turn = (turn + 1) % 3;
            0
        }
    }

    fn key(&self) -> u128 {
        u128::from(self.hands[0])
            | u128::from(self.hands[1]) << 32
            | u128::from(self.hands[2]) << 64
            | u128::from(self.turn) << 96
    }
}

pub(super) struct Search {
    cards: &'static Cards,
    declarer: u8,
    bounds: Table<(u8, u8, u8)>,
    #[cfg(feature = "endgame")]
    experiment: endgame::Probe,
}

impl Search {
    pub(super) fn new(game: GameType, declarer: Seat, order: &[Vec<Card>; 5]) -> Self {
        Self {
            cards: Cards::for_game(game, order),
            declarer: declarer.index() as u8,
            bounds: Table::default(),
            #[cfg(feature = "endgame")]
            experiment: endgame::Probe::new(),
        }
    }

    pub(super) fn clear(&mut self) {
        self.bounds.clear();
    }
    pub(super) fn len(&self) -> usize {
        self.bounds.len()
    }

    pub(super) fn run(
        &mut self,
        p: &Position,
        alpha: u8,
        beta: u8,
        extra: bool,
        meter: &mut Meter,
    ) -> Result<u8, Interrupted> {
        #[cfg(feature = "endgame")]
        let start = self.experiment.start_root(p.tricks_played);
        if p.game == GameType::Null && p.declarer_tricks > 0 {
            meter.tick()?;
            return Ok(0);
        }
        let root = Node::new(p, self.cards);
        let result = if extra {
            self.search::<true>(&root, alpha, beta, meter)
        } else {
            self.search::<false>(&root, alpha, beta, meter)
        };
        #[cfg(feature = "endgame")]
        if let Some(start) = start {
            self.experiment.root_time += start.elapsed();
        }
        result
    }

    fn sure(&self, n: &Node, extra: bool) -> (bool, u8) {
        let own = n.hands[usize::from(self.declarer)];
        let defenders = n.live & !own;
        let run = |suit| {
            let others: u32 = defenders & suit;
            own & suit & (others & others.wrapping_neg()).wrapping_sub(1)
        };
        let trump = run(self.cards.trump);
        let count = trump.count_ones();
        let drawn = n.hands.iter().enumerate().all(|(i, &hand)| {
            i == usize::from(self.declarer) || (hand & self.cards.trump).count_ones() <= count
        });
        let mut sure = trump;
        if drawn {
            for &suit in &self.cards.suits[..4] {
                sure |= run(suit);
            }
        }
        if sure == own {
            return (true, n.remaining);
        }
        let tricks = sure.count_ones();
        let mut lower = 0;
        while sure != 0 {
            lower += self.cards.points[sure.trailing_zeros() as usize];
            sure &= sure - 1;
        }
        if extra && tricks > 0 {
            for (i, &hand) in n.hands.iter().enumerate() {
                if i == usize::from(self.declarer) {
                    continue;
                }
                let mut hand = hand;
                let mut points = [0; 10];
                let mut len = 0;
                while hand != 0 {
                    points[len] = self.cards.points[hand.trailing_zeros() as usize];
                    hand &= hand - 1;
                    len += 1;
                }
                points[..len].sort_unstable();
                lower += points[..tricks as usize].iter().sum::<u8>();
            }
        }
        (false, lower)
    }

    fn search<const EXPERIMENTAL: bool>(
        &mut self,
        n: &Node,
        mut alpha: u8,
        mut beta: u8,
        meter: &mut Meter,
    ) -> Result<u8, Interrupted> {
        meter.tick()?;
        if n.remaining <= alpha {
            return Ok(n.remaining);
        }
        let start = n.count == 0;
        let maximise = n.turn == self.declarer;
        if start && maximise && self.cards.trump != 0 {
            let (all, lower) = self.sure(n, EXPERIMENTAL);
            if lower >= beta || all {
                return Ok(lower);
            }
        }
        let key = n.key();
        let mut hint = NO_CARD;
        if start && let Some(&(lo, hi, card)) = self.bounds.get(&key) {
            if lo >= beta || lo == hi {
                return Ok(lo);
            }
            if hi <= alpha {
                return Ok(hi);
            }
            hint = card;
            alpha = alpha.max(lo);
            beta = beta.min(hi);
        }
        #[cfg(feature = "endgame")]
        let trace = if meter.endgames_allowed && start && beta == alpha + 1 && self.cards.trump != 0
        {
            match self
                .experiment
                .frontier(n, self.cards, self.declarer, beta, meter)?
            {
                endgame::Visit::Bound(answer) => return Ok(if answer { beta } else { alpha }),
                endgame::Visit::Trace(trace) => trace,
            }
        } else {
            None
        };
        let hand = n.hands[usize::from(n.turn)];
        let follow = hand & self.cards.suit[usize::from(n.led)];
        let legal = if !start && follow != 0 { follow } else { hand };
        let (a0, b0) = (alpha, beta);
        let mut best = if maximise { 0 } else { 120 };
        let mut best_card = NO_CARD;
        let mut moves = [0u16; 10];
        let blockers = n.live & !hand;
        let len = self.order(n, legal, blockers, hint, &mut moves);
        // For adjacent cards of the same follow class with no intervening
        // outside rank, swapping their identities preserves every trick
        // winner. The future declarer total changes by at most their
        // point difference. Only propagate proven one-sided bounds.
        let mut previous: Option<(u8, u8)> = None;
        for &item in &moves[..len] {
            let c = (item & 31) as u8;
            let sibling = previous.filter(|_| EXPERIMENTAL).and_then(|(p, value)| {
                let suit = self.cards.suit[c as usize];
                if self.cards.suit[p as usize] != suit {
                    return None;
                }
                let between = (1u32 << p).abs_diff(1u32 << c);
                if between & blockers != 0 {
                    return None;
                }
                let delta = self.cards.points[p as usize].abs_diff(self.cards.points[c as usize]);
                if maximise {
                    Some(value.saturating_add(delta)).filter(|&v| v <= alpha)
                } else {
                    Some(value.saturating_sub(delta)).filter(|&v| v >= beta)
                }
            });
            let value = if let Some(bound) = sibling {
                bound
            } else {
                let mut child = *n;
                let gain = child.play(c, self.cards, self.declarer);
                let a = alpha.saturating_sub(gain);
                let b = beta.saturating_sub(gain).max(a + 1);
                let value =
                    if self.cards.trump == 0 && child.count == 0 && child.turn == self.declarer {
                        meter.tick()?;
                        gain
                    } else {
                        gain + self.search::<EXPERIMENTAL>(&child, a, b, meter)?
                    };
                previous = ((maximise && value <= alpha) || (!maximise && value >= beta))
                    .then_some((c, value));
                value
            };
            if best_card == NO_CARD || (maximise && value > best) || (!maximise && value < best) {
                best = value;
                best_card = c;
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
        if start {
            let entry = self.bounds.entry(key).or_insert((0, 120, NO_CARD));
            entry.2 = best_card;
            if best <= a0 {
                entry.1 = entry.1.min(best);
            } else if best >= b0 {
                entry.0 = entry.0.max(best);
            } else {
                entry.0 = best;
                entry.1 = best;
            }
        }
        #[cfg(feature = "endgame")]
        if let Some(trace) = trace {
            self.experiment.finish(
                trace,
                || endgame::pack(n, self.cards, self.declarer, b0),
                best >= b0,
            );
        }
        Ok(best)
    }

    fn order(
        &self,
        n: &Node,
        mut legal: u32,
        blockers: u32,
        hint: u8,
        out: &mut [u16; 10],
    ) -> usize {
        if legal.is_power_of_two() {
            out[0] = legal.trailing_zeros() as u16;
            return 1;
        }
        let own_side = (n.winner == self.declarer) == (n.turn == self.declarer);
        let mut previous: Option<u8> = None;
        let mut len = 0;
        while legal != 0 {
            let c = legal.trailing_zeros() as u8;
            legal &= legal - 1;
            let suit = self.cards.suit[c as usize];
            let pts = self.cards.points[c as usize];
            if let Some(p) = previous
                && self.cards.suit[p as usize] == suit
                && ((1u32 << c) - (1u32 << p)) & blockers == 0
                && self.cards.points[p as usize] == pts
            {
                continue;
            }
            previous = Some(c);
            let key: u16 = if c == hint {
                0
            } else if n.count == 0 {
                1 + u16::from(11 - pts) * 32 + u16::from(c) - suit.trailing_zeros() as u16
            } else {
                let wins = self.cards.beats(c, n.best);
                if own_side {
                    1 + u16::from(wins) * 16 + u16::from(11 - pts)
                } else if wins {
                    1 + (31 - (u16::from(c) - suit.trailing_zeros() as u16)) * 16 + u16::from(pts)
                } else {
                    513 + u16::from(pts)
                }
            };
            out[len] = (key << 5) | u16::from(c);
            len += 1;
        }
        out[..len].sort_unstable();
        len
    }
}
