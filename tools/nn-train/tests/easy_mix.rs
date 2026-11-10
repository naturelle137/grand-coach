// SPDX-License-Identifier: EUPL-1.2

//! Easy-mix tables: the teacher bids and declares, card play seats 0 to 3
//! Easy players drawn by weighted roles, and every record keeps the
//! teacher's label while the game follows Easy's action.

use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;
use std::time::Duration;

use nn_train::selfplay::{
    EASY_MIX_WEIGHTS, Player, Roles, RunSeed, Table, TeacherProfile, check_record,
};
use nn_train::shards::{Limits, Manifest, Stop, generate, read_shard, verify};
use nn_train::train::{Hyper, load};
use skat_ai::{Budget, Level, Profile};
use skat_core::{Action, Phase, Seat};

const TEST_TEACHER: TeacherProfile = TeacherProfile {
    name: "test",
    profile: Profile {
        budget: Some(Budget {
            samples: 2,
            nodes_per_task: Budget::INTERACTIVE.nodes_per_task,
        }),
        ..Level::Strong.profile()
    },
};

const LINEUPS: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

#[test]
fn roles_round_trip_through_seats_and_labels() {
    for weak in [Player::Rookie, Player::Easy, Player::Medium] {
        for bits in LINEUPS {
            let roles = Roles::from_bits(bits, weak);
            assert_eq!(roles.weak(), bits.count_ones() as usize);
            for declarer in Seat::ALL {
                let lineup = roles.lineup(declarer);
                assert_eq!(Roles::of(lineup, declarer), roles);
                assert_eq!(lineup[declarer.index()], roles.declarer);
                assert_eq!(lineup[declarer.next().index()], roles.defenders[0]);
            }
        }
    }
    assert_eq!(Roles::TEACHERS.label(), "T|TT");
    assert_eq!(Roles::from_bits(0, Player::Easy), Roles::TEACHERS);
    assert_eq!(Roles::from_bits(1, Player::Easy).label(), "E|TT");
    assert_eq!(Roles::from_bits(4, Player::Rookie).label(), "T|TR");
    assert_eq!(Roles::from_bits(7, Player::Medium).label(), "M|MM");
    // Hinterhand declares: Vorhand defends first, then Mittelhand.
    assert_eq!(
        Roles::from_bits(2, Player::Easy).lineup(Seat::Hinterhand),
        [Player::Easy, Player::Teacher, Player::Teacher]
    );
}

#[test]
fn easy_mix_draws_follow_the_weights() {
    for table in Table::ALL {
        assert_eq!(Table::parse(table.name()), Some(table));
    }
    assert_eq!(Table::parse("strong"), None);
    assert_eq!(Table::Teacher.roles(12345), None);
    // Exactly one Easy player weighs 2, every other lineup 1.
    for bits in LINEUPS {
        assert_eq!(
            EASY_MIX_WEIGHTS[usize::from(bits)],
            if bits.count_ones() == 1 { 2 } else { 1 },
            "{bits}"
        );
    }
    let total: u32 = EASY_MIX_WEIGHTS.iter().sum();
    let draws = 110_000u64;
    let mut counts = [0u64; 8];
    for deal in 0..draws {
        let roles = Table::EasyMix
            .roles(RunSeed(3).table_draw(deal))
            .expect("easy-mix draws roles");
        let bits = LINEUPS
            .into_iter()
            .find(|&b| Roles::from_bits(b, Player::Easy) == roles)
            .expect("an Easy lineup");
        counts[usize::from(bits)] += 1;
    }
    for (bits, &n) in counts.iter().enumerate() {
        let expected = draws as f64 * f64::from(EASY_MIX_WEIGHTS[bits]) / f64::from(total);
        let deviation = (n as f64 - expected).abs() / expected;
        assert!(deviation < 0.05, "bits {bits}: {n} vs {expected:.0}");
    }
    // The draw has its own key: it is neither the deal nor a teacher seed.
    let seed = RunSeed(3);
    assert_ne!(seed.table_draw(5), seed.deal_seed(5));
    assert_ne!(
        seed.player_seed(5, Seat::Vorhand),
        seed.teacher_seed(5, Seat::Vorhand)
    );
}

#[test]
fn rookie_medium_and_level_mix_cover_their_lineups_evenly() {
    let draws = 270_000u64;
    for (table, weak) in [
        (Table::RookieMix, Player::Rookie),
        (Table::MediumMix, Player::Medium),
    ] {
        let mut counts = BTreeMap::new();
        for deal in 0..draws {
            let roles = table.roles(RunSeed(5).table_draw(deal)).unwrap();
            *counts.entry(roles).or_insert(0u64) += 1;
        }
        assert_eq!(counts.len(), 8, "{}", table.name());
        for bits in LINEUPS {
            let n = counts[&Roles::from_bits(bits, weak)];
            let expected = draws as f64 / 8.0;
            assert!((n as f64 - expected).abs() / expected < 0.05, "{bits}: {n}");
        }
    }
    let mut counts = BTreeMap::new();
    for deal in 0..draws {
        let roles = Table::LevelMix.roles(RunSeed(5).table_draw(deal)).unwrap();
        assert!(
            [roles.declarer, roles.defenders[0], roles.defenders[1]]
                .iter()
                .all(|p| *p != Player::Rookie)
        );
        *counts.entry(roles).or_insert(0u64) += 1;
    }
    assert_eq!(counts.len(), 27);
    for (roles, n) in counts {
        let expected = draws as f64 / 27.0;
        assert!(
            (n as f64 - expected).abs() / expected < 0.08,
            "{}: {n}",
            roles.label()
        );
    }
}

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn easy_mix_runs_keep_teacher_labels_and_replay() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("nn-train-easy-mix");
    let _ = fs::remove_dir_all(&dir);
    let deals = 8;
    let manifest = Manifest::new("test-commit", TEST_TEACHER, Table::EasyMix, 11, deals, 2);
    assert_eq!(manifest.table, "easy-mix");
    let limits = Limits {
        wall_time: Duration::from_secs(3600),
        disk_bytes: 1 << 30,
        cpus: NonZeroUsize::new(4).unwrap(),
        workers: NonZeroUsize::new(2).unwrap(),
    };
    let report = generate(&dir, &manifest, TEST_TEACHER, limits).unwrap();
    assert_eq!(report.stop, Stop::Finished);
    // The verifier replays Easy's actions and checks every lineup against
    // the run's draw.
    let v = verify(&dir).unwrap();
    assert_eq!((v.complete, v.incomplete), (4, 0));

    let mut roles_seen = BTreeMap::new();
    let mut easy_decisions = 0;
    for index in 0..manifest.shards() {
        for r in read_shard(&dir, index).unwrap() {
            let drawn = Table::EasyMix
                .roles(RunSeed(11).table_draw(r.deal))
                .unwrap();
            if r.view.phase == Phase::Spiel {
                let declarer = r.view.declarer.expect("card play has a declarer");
                assert_eq!(r.lineup, Some(drawn.lineup(declarer)));
                assert_eq!(r.roles(), drawn);
                roles_seen.insert(r.deal, drawn);
            } else {
                // The teacher bids and declares at every seat.
                assert_eq!((r.lineup, r.played), (None, None));
            }
            if r.player() != Player::Teacher {
                easy_decisions += 1;
                let played = r.played.expect("an Easy seat records its action");
                assert!(r.view.legal_actions.contains(&played));
                // The label stays the teacher's, with its analysis.
                assert!(r.view.legal_actions.contains(&r.action));
                // Card play is analysed; Kontra by the rules of thumb is not (#108).
                let rules_kontra = matches!(r.action, Action::Kontra | Action::KeinKontra);
                assert!(r.teacher.status.is_some() || rules_kontra, "{r:?}");
                // Without its lineup the record is inconsistent.
                let mut broken = r.clone();
                broken.lineup = None;
                assert!(check_record(&broken).is_err());
            }
        }
    }
    let total: u64 = report.roles.values().sum();
    assert_eq!(total, roles_seen.len() as u64);
    if roles_seen.values().any(|r| r.weak() > 0) {
        assert!(easy_decisions > 0);
    }

    // Value targets of mixed card play come from the teacher's analysis,
    // not from final points that reflect Easy's play.
    let data = load(std::slice::from_ref(&dir.to_path_buf()), &Hyper::default()).unwrap();
    for examples in [&data.train, &data.validation, &data.test] {
        for row in 0..examples.rows() {
            if examples.roles[row].weak() > 0 && examples.has_options[row] {
                let chosen = examples.offsets[row] + examples.chosen[row] as usize;
                assert!((examples.value[row] - examples.option_score[chosen]).abs() < 1e-3);
            }
        }
        let easy = examples.select(|r| examples.player[r] == Player::Easy);
        let teacher = examples.select(|r| examples.player[r] == Player::Teacher);
        assert_eq!(easy.rows() + teacher.rows(), examples.rows());
        assert!(easy.player.iter().all(|&p| p == Player::Easy));
    }
}
