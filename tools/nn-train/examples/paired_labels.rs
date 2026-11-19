// SPDX-License-Identifier: EUPL-1.2

//! Compare teacher labels on the same reference-generated views. The JSONL
//! output is diagnostic, not a training shard or a candidate trajectory.
//! Arguments: run seed, number of deals, repeat index.

use nn_train::selfplay::{RunSeed, Table, Teacher, TeacherProfile, play_deal};
use skat_ai::Opponent;
use skat_analysis::TwoStage;

fn main() {
    let mut args = std::env::args().skip(1);
    let seed = RunSeed(args.next().expect("run seed").parse().expect("run seed"));
    let deals: u64 = args
        .next()
        .expect("deal count")
        .parse()
        .expect("deal count");
    let repeat: usize = args
        .next()
        .unwrap_or_else(|| "0".into())
        .parse()
        .expect("repeat");
    for deal in 0..deals {
        for (index, record) in play_deal(seed, TeacherProfile::STRONG.profile, Table::Teacher, deal)
            .into_iter()
            .enumerate()
        {
            let opponent = Opponent {
                profile: TeacherProfile::STRONG.profile,
                seed: seed.teacher_seed(deal, record.view.seat),
            };
            let reference = || {
                opponent
                    .decide_analysed(&record.view, None)
                    .expect("decision")
            };
            let candidate = || {
                opponent
                    .decide_two_stage(&record.view, None, TwoStage::PROPOSAL)
                    .expect("decision")
            };
            let first = (index + deal as usize + repeat).is_multiple_of(2);
            let (r, c) = if first {
                let r = reference();
                (r, candidate())
            } else {
                let c = candidate();
                (reference(), c)
            };
            assert_eq!(r.0.action, record.action);
            assert_eq!(r.0.status, record.teacher.status);
            assert_eq!(r.0.fallback, record.teacher.fallback);
            assert_eq!(
                r.1.as_ref().map_or_else(Vec::new, |a| a.options.clone()),
                record.teacher.options
            );
            let teacher = Teacher {
                status: c.0.status,
                fallback: c.0.fallback,
                options: c.1.as_ref().map_or_else(Vec::new, |a| a.options.clone()),
            };
            println!(
                "{}",
                serde_json::json!({"deal": deal, "index": index, "repeat": repeat, "reference_first": first, "reference_record": record, "reference_analysis": r.1, "candidate_action": c.0.action, "candidate_teacher": teacher, "candidate_analysis": c.1, "stage_report": c.2})
            );
        }
    }
}
