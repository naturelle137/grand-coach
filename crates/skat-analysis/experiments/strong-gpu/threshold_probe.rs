// SPDX-License-Identifier: EUPL-1.2

//! Test exact 61-Augen queries with one GPU invocation per sampled position.
mod gpu_batch;
use gpu_batch::Backend;
use rand_chacha::{ChaCha8Rng, rand_core::SeedableRng};
use skat_analysis::{
    Meter,
    double_dummy::{Position, Solver},
    sampling::HiddenCards,
};
use skat_core::{Action, CardSet, Follow, GameType, PlayerView, Seat};
use std::time::Instant;

fn pack(p: &Position, limit: u32, card_table: &[u32; 37]) -> [u32; 52] {
    let mut words = [0; 52];
    for (dst, hand) in words.iter_mut().zip(p.hands) {
        *dst = hand.bits();
    }
    words[3] = p.to_move().index() as u32;
    words[4] = p.trick.len() as u32;
    if let Some(led) = p.trick.led() {
        words[5] = led.index().into();
        let mut best = led;
        let mut winner = p.trick.leader.unwrap_or(Seat::Vorhand);
        for (i, c) in p.trick.cards.iter().flatten().enumerate() {
            if p.game.beats(*c, best, led) {
                best = *c;
                winner = Seat::from_index(p.trick.leader.unwrap_or(Seat::Vorhand).index() + i);
            }
            words[8] += u32::from(c.augen());
        }
        words[6] = best.index().into();
        words[7] = winner.index() as u32;
    }
    words[9] = u32::from(p.declarer_augen + p.skat.augen());
    words[10] = words[8] + p.hands.iter().map(|h| u32::from(h.augen())).sum::<u32>();
    words[12] = p.declarer.index() as u32;
    words[13] = 61;
    words[14] = limit;
    words[15..].copy_from_slice(card_table);
    words
}

fn card_table(game: GameType) -> [u32; 37] {
    let mut words = [0; 37];
    for c in CardSet::ALL {
        let suit = match game.follow(c) {
            Follow::Trumpf => 4,
            Follow::Farbe(s) => s.index(),
        };
        let rank = CardSet::ALL
            .into_iter()
            .filter(|&other| game.follow(c) == game.follow(other) && game.beats(c, other, other))
            .count();
        words[usize::from(c.index())] =
            u32::from(suit) | ((rank as u32) << 3) | (u32::from(c.augen()) << 8);
        words[32 + usize::from(suit)] |= 1u32 << c.index();
    }
    words
}

fn cpu(p: &Position) -> bool {
    Solver::new(p.game, p.declarer)
        .reaches_augen(p, 61, &mut Meter::unlimited())
        .unwrap()
}

fn cpu_batch(positions: &[Position], gpu_answers: Option<&[u32]>) -> Vec<bool> {
    if let Some(rows) = gpu_answers
        && rows.iter().all(|&r| r < 2)
    {
        return rows.iter().map(|&r| r == 1).collect();
    }
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    let chunk = positions.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = positions
            .chunks(chunk)
            .enumerate()
            .map(|(i, group)| {
                scope.spawn(move || {
                    group
                        .iter()
                        .enumerate()
                        .map(|(j, p)| match gpu_answers.map(|rows| rows[i * chunk + j]) {
                            Some(0) => false,
                            Some(1) => true,
                            _ => cpu(p),
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
}

fn main() {
    let gpu =
        Backend::new(include_str!("threshold.wgsl"), 52 * 4 * 2400, 4 * 2400).expect("RX 6900 XT");
    let data = include_str!("../../../../tools/nn-train/fixtures/selfplay/shard-00000.jsonl");
    for tricks in [0, 3, 7] {
        let v = data
            .lines()
            .map(|line| {
                let row: serde_json::Value = serde_json::from_str(line).unwrap();
                serde_json::from_value::<PlayerView>(row["view"].clone()).unwrap()
            })
            .find(|v| {
                v.phase == skat_core::Phase::Spiel
                    && v.tricks.len() == tricks
                    && v.legal_actions.iter().any(|a| matches!(a, Action::Play(_)))
            })
            .unwrap();
        let table = card_table(v.contract.unwrap().game);
        let hidden = HiddenCards::from_view(&v).unwrap();
        let declarer = v.declarer.unwrap();
        let won: Vec<_> = v.tricks.iter().filter(|t| t.winner == declarer).collect();
        for samples in [24, 240, 2400] {
            let mut rng = ChaCha8Rng::seed_from_u64(1);
            let positions: Vec<_> = (0..samples)
                .map(|i| {
                    let w = hidden.sample(&mut rng);
                    let mut p = Position {
                        game: v.contract.unwrap().game,
                        declarer,
                        hands: w.hands,
                        skat: w.skat,
                        trick: v.current_trick,
                        declarer_augen: won.iter().map(|t| t.augen()).sum(),
                        declarer_tricks: won.len() as u8,
                        tricks_played: tricks as u8,
                    };
                    let cards: Vec<_> = p.legal_cards().into_iter().collect();
                    p.play(cards[i % cards.len()]);
                    assert_ne!(p.game, GameType::Null);
                    p
                })
                .collect();
            // CPU oracle outside GPU timing. Every conclusive answer must match.
            let start = Instant::now();
            let reference = cpu_batch(&positions, None);
            let cpu_time = start.elapsed();
            for limit in [512, 8192] {
                let start = Instant::now();
                let jobs: Vec<_> = positions
                    .iter()
                    .flat_map(|p| pack(p, limit, &table))
                    .collect();
                let values: Vec<u32> = gpu.run(bytemuck::cast_slice(&jobs), samples).unwrap();
                let elapsed = start.elapsed();
                let finished = cpu_batch(&positions, Some(&values));
                let hybrid = start.elapsed();
                assert_eq!(finished, reference);
                let mut solved = 0;
                for (value, &expected) in values.iter().zip(&reference) {
                    if *value != 2 {
                        assert_eq!(*value, u32::from(expected));
                        solved += 1;
                    }
                }
                println!(
                    "trick {} samples {samples} cap {limit} solved {solved} round_trip {elapsed:?} CPU {cpu_time:?} hybrid {hybrid:?}",
                    tricks + 1
                );
            }
        }
    }
}
