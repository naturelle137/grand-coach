// SPDX-License-Identifier: EUPL-1.2

//! Prints the exact tournament/calibrate deal stream for contamination audits.
//! Arguments: tuning|validation, first index, count. No games are played.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 3, "suite first count");
    let key = match args[0].as_str() {
        "tuning" => 0x7475_6e69_6e67,
        "validation" => 0x7661_6c69_6461,
        _ => panic!("suite must be tuning or validation"),
    };
    let first: usize = args[1].parse().expect("first index");
    let count: usize = args[2].parse().expect("count");
    let mut rng = ChaCha8Rng::seed_from_u64(key);
    for i in 0..first.checked_add(count).expect("index overflow") {
        let seed = rng.next_u64();
        if i >= first {
            println!("{i}\t{seed}");
        }
    }
}
