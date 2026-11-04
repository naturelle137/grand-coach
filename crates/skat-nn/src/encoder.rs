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
//! | 294 | 32 | play number (1..30) / 30 by card in completed tricks; zero if absent |
//! | 326 | 15 | proven void by relative seat and follow class |
//! | 341 | 32 | unseen cards (concealed hands and unknown Skat together) |
//! | 373 | 32 | Trumpf of this contract |
//! | 405 | 25 | counts / 12 by pool and class (see below) |
//! | 430 | 3 | trick Augen won by relative seat / 120 |
//! | 433 | 6 | Augen / 120 by pool, then on the table |
//! | 439 | 128 | how each card of a completed trick was played: led, followed, trumped, discarded |
//! | 567 | 64 | card fell into a trick won by its player's own party, by the other party |
//! | 631 | 3 | bid standing when each relative seat passed / 264 |
//!
//! Pools: unplayed (including Skat), own hand, known Skat, another seat's
//! exposed hand, unseen. Classes: Kreuz, Pik, Herz, Karo, Trumpf. Before
//! the announcement only printed suits apply. Zero is a valid count;
//! the hand-game, phase and role inputs identify when the Skat is known.
//!
//! The last three blocks describe how earlier decisions were taken, the
//! evidence about hidden hands that card sets alone do not show: a
//! discarded or trumped card proves a void and reveals what the player
//! chose to give up, a card played into the own party's trick (schmieren)
//! signals, and the bid at which a seat passed bounds its hand. Trumped and
//! discarded use the follow classes of the contract (none before it; Null
//! has no Trumpf); the parties need the declarer.

use std::ops::Range;

use skat_core::{
    Call, Card, CardSet, Follow, GameType, KontraLevel, Phase, PlayerView, Seat, Suit,
};

/// Version of the feature layout and of the action space
/// ([`crate::actions`]) used for new supervised models.
pub const ENCODER_VERSION: u16 = 2;

/// Length of the feature vector.
pub const FEATURES: usize = PASS_AT + 3;

pub(crate) const V1_FEATURES: usize = 294;
const PLAY_ORDER: usize = V1_FEATURES;
const VOIDS: usize = PLAY_ORDER + 32;
const UNSEEN: usize = VOIDS + 15;
const TRUMPS: usize = UNSEEN + 32;
const COUNTS: usize = TRUMPS + 32;
const SEAT_AUGEN: usize = COUNTS + 25;
const POOL_AUGEN: usize = SEAT_AUGEN + 3;
const PLAY_CONTEXT: usize = POOL_AUGEN + 6;
const PLAY_INTO: usize = PLAY_CONTEXT + 4 * 32;
const PASS_AT: usize = PLAY_INTO + 2 * 32;

/// Named blocks of the feature vector in layout order, for diagnostics
/// such as ablations (`nn-train probe`). They tile `0..FEATURES`; the
/// first ones up to [`V1_FEATURES`] are the v1 layout.
pub const BLOCKS: [(&str, Range<usize>); 30] = [
    ("hand", HAND..OWN_SKAT),
    ("own_skat", OWN_SKAT..OUVERT),
    ("ouvert", OUVERT..PLAYED),
    ("played_by_seat", PLAYED..TRICK_LED),
    ("trick_led", TRICK_LED..TRICK_SECOND),
    ("trick_second", TRICK_SECOND..TRICK_LEADER),
    ("trick_leader", TRICK_LEADER..POSITION),
    ("position", POSITION..PHASE),
    ("phase", PHASE..DECLARER),
    ("declarer", DECLARER..BID_VALUE),
    ("bid_value", BID_VALUE..PASSED),
    ("passed", PASSED..CURRENT_BID),
    ("current_bid", CURRENT_BID..HAND_GAME),
    ("hand_game", HAND_GAME..GAME_TYPE),
    ("game_type", GAME_TYPE..LEVELS),
    ("levels", LEVELS..KONTRA),
    ("kontra", KONTRA..KONTRA_RULE),
    ("kontra_rule", KONTRA_RULE..TRICKS),
    ("tricks", TRICKS..AUGEN),
    ("augen", AUGEN..V1_FEATURES),
    ("play_order", PLAY_ORDER..VOIDS),
    ("voids", VOIDS..UNSEEN),
    ("unseen", UNSEEN..TRUMPS),
    ("trumps", TRUMPS..COUNTS),
    ("counts", COUNTS..SEAT_AUGEN),
    ("seat_augen", SEAT_AUGEN..POOL_AUGEN),
    ("pool_augen", POOL_AUGEN..PLAY_CONTEXT),
    ("play_context", PLAY_CONTEXT..PLAY_INTO),
    ("play_into", PLAY_INTO..PASS_AT),
    ("pass_at", PASS_AT..FEATURES),
];

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
    let mut f = vec![0.0; FEATURES];
    encode_base(view, &mut f);
    encode_counting(view, &mut f);
    encode_behaviour(view, &mut f);
    f
}

/// Frozen input layout of the shipped v1 model.
pub(crate) fn encode_v1(view: &PlayerView) -> Vec<f32> {
    let mut f = vec![0.0; V1_FEATURES];
    encode_base(view, &mut f);
    f
}

fn encode_base(view: &PlayerView, f: &mut [f32]) {
    let me = view.seat;
    set_cards(f, HAND, view.hand);
    if let Some(skat) = view.own_skat {
        set_cards(f, OWN_SKAT, skat);
    }
    if let Some(cards) = view.declarer_cards {
        set_cards(f, OUVERT, cards);
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
}

/// Cards `view.seat` cannot locate: not played (completed tricks and the
/// trick on the table), not in the own hand, not in the own discarded Skat,
/// not in a revealed Ouvert hand. They are in the other hands or the Skat.
pub fn unseen_cards(view: &PlayerView) -> CardSet {
    let mut seen =
        view.hand | view.own_skat.unwrap_or_default() | view.declarer_cards.unwrap_or_default();
    for trick in &view.tricks {
        seen |= trick.cards.iter().copied().collect::<CardSet>();
    }
    seen |= view
        .current_trick
        .cards
        .iter()
        .flatten()
        .copied()
        .collect::<CardSet>();
    CardSet::ALL - seen
}

fn encode_counting(view: &PlayerView, f: &mut [f32]) {
    let table: CardSet = view.current_trick.cards.iter().flatten().copied().collect();
    let mut played = table;
    for (t, trick) in view.tricks.iter().enumerate() {
        for (i, &card) in trick.cards.iter().enumerate() {
            f[PLAY_ORDER + usize::from(card.index())] = (3 * t + i + 1) as f32 / 30.0;
            played.insert(card);
        }
        f[SEAT_AUGEN + relative(view.seat, trick.winner)] += f32::from(trick.augen()) / 120.0;
    }

    let unplayed = CardSet::ALL - played;
    let skat = view.own_skat.unwrap_or_default();
    let exposed = view.declarer_cards.unwrap_or_default() - view.hand;
    let unseen = unseen_cards(view);
    debug_assert_eq!(unseen, unplayed - view.hand - skat - exposed);
    set_cards(f, UNSEEN, unseen);

    let game = view.contract.map(|c| c.game);
    let trumps = game.map_or(CardSet::EMPTY, GameType::trumps);
    set_cards(f, TRUMPS, trumps);
    let classes = [
        Follow::Farbe(Suit::Kreuz),
        Follow::Farbe(Suit::Pik),
        Follow::Farbe(Suit::Herz),
        Follow::Farbe(Suit::Karo),
        Follow::Trumpf,
    ];
    let pools = [unplayed, view.hand, skat, exposed, unseen];
    let voids = view.proven_voids();
    for (c, class) in classes.into_iter().enumerate() {
        // Null has no trumps and follows printed suits, also the desired
        // grouping before the contract is known.
        let cards = game.unwrap_or(GameType::Null).cards_of(class);
        for seat in Seat::ALL {
            f[VOIDS + 5 * relative(view.seat, seat) + c] =
                flag(!voids[seat.index()].is_disjoint(cards));
        }
        for (p, pool) in pools.iter().enumerate() {
            f[COUNTS + 5 * p + c] = (*pool & cards).len() as f32 / 12.0;
        }
    }
    for (p, pool) in pools.into_iter().chain([table]).enumerate() {
        f[POOL_AUGEN + p] = f32::from(pool.augen()) / 120.0;
    }
}

/// How a card of a completed trick was played (index in the
/// play-context block).
fn play_kind(game: Option<GameType>, led: Card, i: usize, card: Card) -> usize {
    if i == 0 {
        return 0;
    }
    let Some(game) = game else {
        // Before the contract no trick is played; keep a defined value.
        return 1;
    };
    let (class, own) = (game.follow(led), game.follow(card));
    if own == class {
        1
    } else if own == Follow::Trumpf {
        2
    } else {
        3
    }
}

fn encode_behaviour(view: &PlayerView, f: &mut [f32]) {
    let game = view.contract.map(|c| c.game);
    for trick in &view.tricks {
        for (i, &card) in trick.cards.iter().enumerate() {
            let c = usize::from(card.index());
            let kind = play_kind(game, trick.cards[0], i, card);
            f[PLAY_CONTEXT + 32 * kind + c] = 1.0;
            if let Some(declarer) = view.declarer {
                let player = trick.seat_of(i);
                let same = (player == declarer) == (trick.winner == declarer);
                f[PLAY_INTO + if same { c } else { 32 + c }] = 1.0;
            }
        }
    }
    let mut standing = 0u16;
    for record in &view.bidding {
        match record.call {
            Call::Bid(v) | Call::Hold(v) => standing = standing.max(v),
            Call::Pass => {
                f[PASS_AT + relative(view.seat, record.seat)] = f32::from(standing) / MAX_BID;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skat_core::{
        Announcement, BidRecord, CompletedTrick, Contract, Game, Rank, RuleConfig, Trick,
    };

    #[test]
    fn ordered_history_separates_equal_v1_inputs() {
        let mut view = Game::new(1, RuleConfig::default()).view(Seat::Mittelhand);
        let trick = |suit| CompletedTrick {
            leader: Seat::Vorhand,
            cards: [Rank::Ass, Rank::Zehn, Rank::Neun].map(|r| Card::new(suit, r)),
            winner: Seat::Vorhand,
        };
        view.tricks = vec![trick(Suit::Pik), trick(Suit::Karo)];
        let before = encode(&view);
        for (t, trick) in view.tricks.iter().enumerate() {
            for (i, card) in trick.cards.iter().enumerate() {
                assert_eq!(
                    before[PLAY_ORDER + usize::from(card.index())],
                    (3 * t + i + 1) as f32 / 30.0
                );
            }
        }
        view.tricks.swap(0, 1);
        let after = encode(&view);
        assert_eq!(before[..V1_FEATURES], after[..V1_FEATURES]);
        assert_ne!(before[PLAY_ORDER..VOIDS], after[PLAY_ORDER..VOIDS]);
        assert_eq!(
            after[PLAY_ORDER..VOIDS]
                .iter()
                .filter(|&&x| x != 0.0)
                .count(),
            6
        );
    }

    #[test]
    fn count_points_and_public_voids_by_follow_class() {
        use Suit::{Herz, Karo, Kreuz, Pik};
        let mut view = Game::new(3, RuleConfig::default()).view(Seat::Hinterhand);
        view.declarer = Some(view.seat);
        view.phase = Phase::Spiel;
        view.hand = [
            Card::new(Karo, Rank::Ass),
            Card::new(Pik, Rank::Koenig),
            Card::new(Pik, Rank::Zehn),
            Card::new(Pik, Rank::Acht),
            Card::new(Pik, Rank::Neun),
            Card::new(Herz, Rank::Sieben),
            Card::new(Herz, Rank::Acht),
            Card::new(Karo, Rank::Sieben),
            Card::new(Karo, Rank::Acht),
        ]
        .into_iter()
        .collect();
        let skat: CardSet = [Card::new(Karo, Rank::Zehn), Card::new(Pik, Rank::Sieben)]
            .into_iter()
            .collect();
        view.own_skat = Some(skat);
        view.tricks = vec![CompletedTrick {
            leader: Seat::Vorhand,
            cards: [
                Card::bube(Kreuz),
                Card::bube(Herz),
                Card::new(Herz, Rank::Zehn),
            ],
            winner: Seat::Vorhand,
        }];
        view.current_trick = Trick {
            leader: Some(Seat::Vorhand),
            cards: [
                Some(Card::new(Pik, Rank::Ass)),
                Some(Card::bube(Karo)),
                None,
            ],
        };
        for (game, void_flags, trump_counts) in [
            (GameType::Farbe(Herz), vec![11], [7, 2, 0, 0, 5]),
            (GameType::Grand, vec![4, 11], [1, 0, 0, 0, 1]),
            (GameType::Null, vec![0, 10, 11], [0; 5]),
        ] {
            view.contract = Some(Contract::new(Announcement::plain(game), false));
            let f = encode(&view);
            let actual: Vec<_> = f[VOIDS..UNSEEN]
                .iter()
                .enumerate()
                .filter_map(|(i, &flag)| (flag == 1.0).then_some(i))
                .collect();
            assert_eq!(actual, void_flags, "{game:?}");
            for (pool, count) in trump_counts.into_iter().enumerate() {
                assert_eq!(f[COUNTS + 5 * pool + 4], count as f32 / 12.0);
            }
            assert_eq!(f[SEAT_AUGEN..POOL_AUGEN], [0.0, 14.0 / 120.0, 0.0]);
            assert_eq!(
                f[POOL_AUGEN..PLAY_CONTEXT],
                [93, 25, 10, 0, 58, 13].map(|a| a as f32 / 120.0)
            );
            assert_eq!(f[UNSEEN..TRUMPS].iter().sum::<f32>(), 16.0);
        }

        // A defender now sees the same hand exposed, but cannot locate the Skat.
        view.declarer = Some(Seat::Vorhand);
        view.own_skat = None;
        view.declarer_cards = Some(skat);
        let f = encode(&view);
        assert_eq!(
            f[POOL_AUGEN..PLAY_CONTEXT],
            [93, 25, 0, 10, 58, 13].map(|a| a as f32 / 120.0)
        );
        view.declarer_cards = None;
        let f = encode(&view);
        assert_eq!(
            f[POOL_AUGEN..PLAY_CONTEXT],
            [93, 25, 0, 0, 68, 13].map(|a| a as f32 / 120.0)
        );
        for card in skat {
            assert_eq!(f[UNSEEN + usize::from(card.index())], 1.0);
        }
    }

    #[test]
    fn play_context_party_and_pass_values() {
        use Suit::{Herz, Karo, Kreuz, Pik};
        let mut view = Game::new(5, RuleConfig::default()).view(Seat::Mittelhand);
        view.declarer = Some(Seat::Vorhand);
        view.phase = Phase::Spiel;
        view.bidding = vec![
            BidRecord {
                seat: Seat::Mittelhand,
                call: Call::Bid(18),
            },
            BidRecord {
                seat: Seat::Vorhand,
                call: Call::Hold(18),
            },
            BidRecord {
                seat: Seat::Mittelhand,
                call: Call::Bid(20),
            },
            BidRecord {
                seat: Seat::Vorhand,
                call: Call::Hold(20),
            },
            BidRecord {
                seat: Seat::Mittelhand,
                call: Call::Pass,
            },
            BidRecord {
                seat: Seat::Hinterhand,
                call: Call::Pass,
            },
        ];
        // Vorhand leads Pik Ass, Mittelhand discards Karo Zehn (schmieren
        // is impossible: the declarer wins), Hinterhand plays Herz Bube.
        let ass = Card::new(Pik, Rank::Ass);
        let zehn = Card::new(Karo, Rank::Zehn);
        let bube = Card::bube(Herz);
        view.tricks = vec![CompletedTrick {
            leader: Seat::Vorhand,
            cards: [ass, zehn, bube],
            winner: Seat::Hinterhand,
        }];
        let kind = |f: &[f32], card: Card| {
            (0..4)
                .filter(|k| f[PLAY_CONTEXT + 32 * k + usize::from(card.index())] == 1.0)
                .collect::<Vec<_>>()
        };
        let into = |f: &[f32], card: Card| {
            let c = usize::from(card.index());
            [f[PLAY_INTO + c], f[PLAY_INTO + 32 + c]]
        };
        for (game, bube_kind) in [
            (GameType::Farbe(Kreuz), 2),
            (GameType::Grand, 2),
            (GameType::Null, 3),
        ] {
            view.contract = Some(Contract::new(Announcement::plain(game), false));
            let f = encode(&view);
            assert_eq!(kind(&f, ass), [0], "{game:?}");
            assert_eq!(kind(&f, zehn), [3], "{game:?}");
            assert_eq!(kind(&f, bube), [bube_kind], "{game:?}");
            // Defenders Mittelhand and Hinterhand are one party.
            assert_eq!(into(&f, ass), [0.0, 1.0]);
            assert_eq!(into(&f, zehn), [1.0, 0.0]);
            assert_eq!(into(&f, bube), [1.0, 0.0]);
            // Relative seats: self (Mittelhand) passed at 20, the next seat
            // (Hinterhand) too, the declarer never passed.
            assert_eq!(f[PASS_AT..], [20.0 / MAX_BID, 20.0 / MAX_BID, 0.0]);
        }
        let followed = Card::new(Pik, Rank::Neun);
        view.tricks[0].cards[1] = followed;
        assert_eq!(kind(&encode(&view), followed), [1]);
    }

    #[test]
    fn blocks_tile_the_vector() {
        assert_eq!(AUGEN + 2, V1_FEATURES);
        let mut next = 0;
        for (name, block) in BLOCKS {
            assert_eq!(block.start, next, "{name}");
            assert!(block.end > block.start, "{name}");
            next = block.end;
        }
        assert_eq!(next, FEATURES);
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
