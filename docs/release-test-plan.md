# Release 1.0 test plan

After the release scope is implemented, the maintainer tests the release
candidate on Linux and Windows 11. Finalise reproducible seeds and steps
against that build before starting. Use only features included in the release.

## Test cases

| Area | Steps | Expected result |
|------|-------|-----------------|
| Installation and lifecycle | Install on a clean system; launch, restart, upgrade and uninstall. Check resources, notices and user-data locations. | Packages work with documented prerequisites; upgrades preserve supported data and uninstall follows the documented policy. |
| Full game and series | Play a complete series and free play; exercise bidding, Hand/Skat, discarding, suit/Grand/Null, Ouvert, Kontra/Re and all-pass handling using reproducible deals. | Legal actions, scores, seat rotation and completion are understandable and consistent. |
| Presentation | Switch between both languages and all three card faces during play. | Cards and controls remain readable; text is complete and the running game is preserved. |
| Opponents | Select each strength independently and play Goldfish, Mephisto and God Mode table modes. | Turns complete at usable pacing; table modes and their information rules are clear. |
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
