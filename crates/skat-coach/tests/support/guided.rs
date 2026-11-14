// SPDX-License-Identifier: EUPL-1.2

use skat_core::{
    Action, Announcement, Card, CardSet, Deal, Game, GameType, Phase, Rank, RuleConfig, Seat, Suit,
};

pub fn card(s: &str) -> Card {
    let suit = match s.as_bytes()[0] {
        b'C' => Suit::Kreuz,
        b'S' => Suit::Pik,
        b'H' => Suit::Herz,
        b'D' => Suit::Karo,
        _ => panic!("suit"),
    };
    let rank = match s.as_bytes()[1] {
        b'7' => Rank::Sieben,
        b'8' => Rank::Acht,
        b'9' => Rank::Neun,
        b'T' => Rank::Zehn,
        b'J' => Rank::Bube,
        b'Q' => Rank::Dame,
        b'K' => Rank::Koenig,
        b'A' => Rank::Ass,
        _ => panic!("rank"),
    };
    Card::new(suit, rank)
}

pub fn cards(s: &str) -> CardSet {
    s.split_whitespace().map(card).collect()
}

pub fn deal() -> Deal {
    Deal::new(
        [
            cards("CK CJ C9 H7 H8 H9 HT HJ HQ HK"),
            cards("C7 C8 CT S7 S8 S9 ST SJ SQ SK"),
            cards("CA CQ SA HA D7 D8 D9 DT DJ DQ"),
        ],
        cards("DK DA"),
    )
    .unwrap()
}

/// HH holds `hh`, the Skat `skat`; VH and MH get the rest in card order.
#[allow(dead_code)]
pub fn deal_hh(hh: &str, skat: &str) -> Deal {
    let (hh, skat) = (cards(hh), cards(skat));
    let rest: Vec<Card> = (CardSet::ALL - hh - skat).iter().collect();
    let vh: CardSet = rest[..10].iter().copied().collect();
    let mh: CardSet = rest[10..].iter().copied().collect();
    Deal::new([vh, mh, hh], skat).unwrap()
}

pub fn swap(deal: Deal, a: Card, b: Card) -> Deal {
    let swap = |mut set: CardSet| {
        if set.contains(a) != set.contains(b) {
            if set.remove(a) {
                set.insert(b);
            } else {
                set.remove(b);
                set.insert(a);
            }
        }
        set
    };
    Deal::new(Seat::ALL.map(|s| swap(deal.hand(s))), swap(deal.skat())).unwrap()
}

/// HH declares Herz Hand; all core actions are checked, never fake PlayerViews.
pub fn position(deal: Deal, declarer: bool) -> Game {
    position_led(deal, declarer, if declarer { "C9" } else { "CK" })
}

pub fn position_led(deal: Deal, declarer: bool, lead: &str) -> Game {
    let mut g = Game::from_deal(deal, RuleConfig::default());
    while g.phase() != Phase::Spiel {
        let a = match g.phase() {
            Phase::Reizen if g.current_seat() == Some(Seat::Hinterhand) => Action::Bid(18),
            Phase::Reizen => Action::Pass,
            Phase::SkatAufnahme => Action::Hand,
            Phase::Ansage => Action::Announce(Announcement::plain(GameType::Farbe(Suit::Herz))),
            _ => panic!("unexpected phase"),
        };
        g.apply(a).unwrap();
    }
    g.apply(Action::Play(card(lead))).unwrap();
    if declarer {
        g.apply(Action::Play(card("C8"))).unwrap();
    }
    g
}
