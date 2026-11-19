// SPDX-License-Identifier: EUPL-1.2
//! Real ordinary widgets of the Farbspiel LeadMaster route at both supported sizes.
#[allow(dead_code)]
#[path = "support/guided.rs"]
mod guided_support;
#[path = "../examples/support/suit_lead_widgets.rs"]
mod widgets;

#[test]
#[ignore = "slow: CI only (docs/testing.md)"]
fn suit_lead_master_all_ordinary_stages_both_languages_and_sizes() {
    for name in widgets::names() {
        widgets::render(&name, None);
    }
}
