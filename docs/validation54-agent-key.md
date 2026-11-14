# Validation 54 revision 1: withheld agent reference

**Do not give this file, source fixtures, manifests containing positions, or
reference tests to the learner.** Learner instructions are a separate file.
This key is fixed before any human results. It is a factual scoring reference,
not an expert assessment, strategy recommendation or efficacy claim.

## Independent position facts

`practice_catalogue/reserved.rs` anchors the hands, completes the remaining
cards deterministically and replays every action through core. All are closed
Hand games with unknown Skat. V/M/H denote physical seats. A/10/K/Q/J/9/8/7
are ranks. Each task is version 1 and uses the exact integer or typed equality
below, with no partial credit. The analysis does not supply these references.

| Task suffix | Reference | Independent reason and difference from public practice |
|---|---|---|
| initial.trumpf | 2 | Grand: four Buben minus M's own Herz-J minus the public Kreuz-J. A completed Karo trick precedes this incomplete Trumpf trick; public counting cases have no completed trick and answers 5/1/0. |
| initial.void | 1 | Null: M discarded Pik-7 on Herz-J, proving M void in Herz; H served Herz-7. The later Kreuz lead/discard is separate evidence. Null J belongs to its suit. Public void positive cases use Farbspiel/Grand and have no completed trick. |
| initial.side | Possible | After a completed Kreuz trick, V in Karo holds every non-J Pik card, including the queried Pik-A. Both opponents are necessarily Pik-void, but unseen Trumpf may or may not be in their hands. Public analogous lead has no previous trick, a different Trumpf contract and different unseen Trumpf count. |
| initial.partner | Possible | Grand, after Kreuz-J/Herz-J/Karo-J, V leads Herz-10. M holds all six other non-J Herz including Herz-A and plays Herz-7. H can overtake only by trumping with the one unseen Pik-J, whose ownership is unknown. Public overtaking positives allow a higher side card; here none exists outside M. |
| delayed.trumpf | 7 | Kreuzspiel: eleven Trumpf minus V's four (Kreuz-A/10/K and Karo-J); none in the completed Karo trick. Different own Trumpf count and historical prefix from every public case and initial.trumpf. |
| delayed.void | 1 | Grand: M served Pik-J in the first Trumpf trick, then discarded Herz-7 on Karo-J. That later failure proves M Trumpf-void; H's earlier Herz-J followed. Public changed void has no completed trick or earlier successful Trumpf service. |
| delayed.side | Absent | Grand: V owns all four Buben. After the Kreuz-A/8/9 trick, V's queried Kreuz-10 cannot be trumped. Unlike the public Null boundary, this is a Trumpf game with all Trumpf visibly in own hand. |
| delayed.partner | Absent | Grand: after a completed Herz trick, V leads Kreuz-J. M must follow with Pik-J; no card can beat the highest Trumpf. Unlike the public Karo boundary, M has three Grand Trumpf and a completed side trick. |

These are newly authored permitted views with different factual constraints,
not renamed catalogue cases, rotations or changed completion seeds. Tests
compare every reserved view against all twelve public views and each other,
check independent references and core-legal continuations, and substitute
concealed hands/unknown Skat while retaining the exact permitted view.
Positive and counterexample mechanism tests additionally use legal concrete
completions; the source's independent one-trick core oracle must remain
separate from `mechanisms`.

Changing a position, question, meaning, scoring or protocol after results exist
requires a new version and retains original records unchanged. Never export
reserved views to a gallery, screenshot, learner preview or ordinary practice.
