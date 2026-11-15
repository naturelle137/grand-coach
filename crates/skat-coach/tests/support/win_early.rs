// SPDX-License-Identifier: EUPL-1.2

//! Frozen handwritten #161 A–E oracle. Never derived from policy output.
use super::fixture::*;
use skat_core::{Deal, Game, GameType, Seat};

pub fn deals() -> [Deal; 5] {
    let a = swap(deal(), card("CA"), card("C7"));
    let b = swap(swap(a, card("C7"), card("H8")), card("CQ"), card("H7"));
    let mut c = deal();
    for (x, y) in [("C7", "D7"), ("C8", "D8"), ("CT", "D9")] {
        c = swap(c, card(x), card(y));
    }
    let mut d = c;
    for (x, y) in [
        ("CA", "H7"),
        ("CQ", "H8"),
        ("C7", "H9"),
        ("C8", "HT"),
        ("CT", "HQ"),
        ("HA", "CJ"),
    ] {
        d = swap(d, card(x), card(y));
    }
    [a, b, c, d, deal()]
}

#[allow(dead_code)]
pub fn position(deal: Deal, declarer: Seat) -> Game {
    contract_position(deal, declarer, hand_game(GameType::Grand, false), &["CK"])
}

pub const LEGAL: [&str; 5] = [
    "C8 CT CA",
    "C8 CT CA",
    "D7 D8 D9 S7 S8 S9 ST SJ SQ SK",
    "D7 D8 D9 S7 S8 S9 ST SJ SQ SK",
    "C7 C8 CT",
];
pub const WINNERS: [&str; 5] = ["CT CA", "CT CA", "SJ", "SJ", "CT"];
pub const EASY: [&str; 5] = ["CT", "CT", "SJ", "SJ", "C7"];
pub const REPLY: [&str; 5] = ["CQ", "DJ", "C7", "CJ", "CA"];

/// Literal completed winner for every legal alternative with REPLY.
pub fn winner(case: usize, mine: &str) -> Seat {
    match case {
        0 if mine != "C8" => Seat::Mittelhand,
        2 if mine == "SJ" => Seat::Mittelhand,
        0 | 2 => Seat::Vorhand,
        1 | 3 | 4 => Seat::Hinterhand,
        _ => panic!("case"),
    }
}
