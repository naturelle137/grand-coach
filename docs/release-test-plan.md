# Release 1.0 test plan

After the release scope is implemented, agents test the release
candidate on Linux and Windows 11. Finalise reproducible seeds and steps
against that build before starting. Use only features included in the release.

Before an RC can ship, dispatch CI on its exact ref and record green
`complete` suites (including ignored tests) on both Ubuntu and Windows,
plus required native render/backend smoke. Record run URLs and commit SHA.
A fast PR gate does not replace these complete suites or the Linux and
Windows 11 package/platform cases below. Headless Windows Server CI does
not establish interactive Windows 11 behaviour or teaching effectiveness.
Missing platform or actual learner/UAT evidence stays pending; agents never
simulate learner observations. M7 save/load cases apply once implemented;
current wire serialization tests are not persistence evidence.

Release 1.0 requires [modern egui #134](https://github.com/naturelle137/grand-coach/issues/134)
implemented, independently reviewed, tested and merged through the protected
PR process. Its full acceptance criteria govern; #131's dialogue is not a
prerequisite and Godot animation is outside this scope. The presentation and
opponent checks below are required and pending until supported by recorded
evidence. Existing M8, learner and platform gates remain required.

## Test cases

| Area | Steps | Expected result |
|------|-------|-----------------|
| Installation and lifecycle | Install on a clean system; launch, restart, upgrade and uninstall. Check resources, notices and user-data locations. | Packages work with documented prerequisites; upgrades preserve supported data and uninstall follows the documented policy. |
| Full game and series | Play a complete series and free play; exercise bidding, Hand/Skat, discarding, suit/Grand/Null, Ouvert, Kontra/Re and all-pass handling using reproducible deals. | Legal actions, scores, seat rotation and completion are understandable and consistent. |
| Presentation | Exercise all existing flows and modes in German and English with all three card faces; inspect actual egui screenshots at 1600 × 1000, 1280 × 820 and 960 × 640, including scaled fonts and long names. Hold and release completed-trick Augen with mouse and keyboard; also lose focus and change games while held. Inspect tooltips and accessibility labels while concealed. | The responsive coach-left, table-centre and last-trick/series-right layout preserves the trick triangle; cards and actions remain readable and reachable. Only permitted completed-trick Augen appear while held, excluding an unknown Skat; release, focus loss and game change conceal them immediately, including in tooltips and accessibility labels. The series score list stays visible and independent of the reveal. |
| Opponents | Select each strength independently and play all existing table modes, including Goldfish, Mephisto, God Mode and open display. Repeat a seeded series and switch German → English → German; compare deals and AI decisions with the same seed. | Turns complete at usable pacing and information rules remain clear. Two distinct named portraits stay paired and at their physical seats as roles rotate; the language roundtrip restores the original pair. Deterministic selection uses an isolated random stream and changes neither deals nor AI decisions. |
| Live coach | Try off, on request, important decisions and every move; repeat help requests; enable the warning and try proceed/cancel. Change settings while analysis runs. | Ordinary play stays responsive; no stale hints or duplicate actions; warnings and intervention levels behave as described. |
| Lessons and review | Exercise all six lessons, learning focus and historical decision moments. Compare figures off with explicitly requested figures. | Wording and uncertainty are understandable; historical context is clear; figures follow the opt-in behavior. Use the learner script below. |
| Counting aid and quiz | Toggle aid/quiz/coach combinations; answer correctly and incorrectly, skip, disable and change language/settings. | Counts and feedback are understandable; assistance does not reveal a pending answer; dismissing a question preserves the turn. |
| Open mode and replay | Play openly; save/load a game; navigate both directions; retry the same deal with different actions. | Views and hindsight are clearly labelled; navigation is usable; recordings and original scores stay intact. |
| History and training | Restart with saved progress; reset it deliberately; enter, retry and leave training; compare assisted, repeated and new tasks. | Persistence and reset match their descriptions; practice does not corrupt series scores; progress distinguishes assistance and repetition. |
| Optional model packages | If included, play offline with each shipped backend; exercise missing/corrupt-model recovery, fallback, coach and language resources. | Backend choice and recovery are clear; packaged resources work offline and the app remains usable. |

## Learner sessions

Use [coach-validation.md](coach-validation.md#b-coach-learner-uat) for
comprehension/usability and its [unaided-task protocol](coach-validation.md#c-initial-and-delayed-unaided-task-protocol)
for initial and delayed attempts on new positions. Freeze task versions,
expected answers, scoring, assistance rules and the follow-up interval
before collecting results. The maintainer participates as a learner;
these observations do not establish expert certification or general
learning effectiveness.

## Results and release

Store completed records under `docs/uat/`. Each record identifies the
build/package, date, OS/hardware, seed or scenario, settings, steps,
expected and observed result, and any finding. Record the actual interval
and prior exposure for learner follow-up sessions.

Fix release-blocking findings and repeat affected checks before release.
Document remaining limitations in the release notes.
