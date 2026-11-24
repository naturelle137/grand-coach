# Release 1.0 test plan

After the release scope is implemented, agents test the release
candidate using Linux checks and Windows CI. Finalise reproducible seeds and steps
against that build before starting. Use only features included in the release.

Before an RC can ship, dispatch CI on its exact ref and record green
`complete` suites (including ignored tests) on both Ubuntu and Windows,
plus required native render/backend smoke. Record run URLs and commit SHA.
A fast PR gate does not replace these complete suites or the functional,
package and visual checks below. Successful Windows CI suffices for Windows
verification; no separate Windows device or CEO interactive acceptance is
required (D-36). Agents execute the checks and inspect actual visual results,
including any model-assisted findings. These checks do not establish learner
comprehension, learning efficacy or transfer. Missing human observations do not
block development or release; never simulate them. Human participation is
considered only for a concretely necessary test demonstrably impossible otherwise
(D-51). M7 save/load and M8 history/training are implemented and must be exercised.

Release 1.0 requires [modern egui #134](https://github.com/naturelle137/grand-coach/issues/134)
implemented, independently reviewed, tested and merged through the protected
PR process. Its full acceptance criteria govern; #131's dialogue is not a
prerequisite and Godot animation is outside this scope. The presentation and
opponent checks below are required and pending until supported by recorded
evidence. M8 functional and package checks remain required.

## Test cases

Use the [private RC packaging procedure](packaging.md) to build and identify
the exact archives, prerequisites, lifecycle policy and automated smoke limits.
Attach archive SHA-256, source commit and packaging workflow run to each record.

| Area | Steps | Expected result |
|------|-------|-----------------|
| Installation and lifecycle | Use isolated package/runtime checks; launch, restart, upgrade and uninstall. Check resources, notices and user-data locations. | Packages work with documented prerequisites; upgrades preserve supported data and uninstall follows the documented policy. |
| Full game and series | Play a complete series and free play; exercise bidding, Hand/Skat, discarding, suit/Grand/Null, Ouvert, Kontra/Re and all-pass handling using reproducible deals. | Legal actions, scores, seat rotation and completion are understandable and consistent. |
| Presentation | Exercise all existing flows and modes in German and English with all three card faces; inspect actual egui screenshots at 1600 × 1000, 1280 × 820 and 960 × 640, including scaled fonts and long names. Hold and release completed-trick Augen with mouse and keyboard; also lose focus and change games while held. Inspect tooltips and accessibility labels while concealed. | The responsive coach-left, table-centre and last-trick/series-right layout preserves the trick triangle; cards and actions remain readable and reachable. Only permitted completed-trick Augen appear while held, excluding an unknown Skat; release, focus loss and game change conceal them immediately, including in tooltips and accessibility labels. The series score list stays visible and independent of the reveal. |
| Opponents | Select each strength independently and play all existing table modes, including Goldfish, Mephisto, God Mode and open display. Repeat a seeded series and switch German → English → German; compare deals and AI decisions with the same seed. | Turns complete at usable pacing and information rules remain clear. Two distinct named portraits stay paired and at their physical seats as roles rotate; the language roundtrip restores the original pair. Deterministic selection uses an isolated random stream and changes neither deals nor AI decisions. |
| Live coach | Try off, on request, important decisions and every move; repeat help requests; enable the warning and try proceed/cancel. Change settings while analysis runs. | Ordinary play stays responsive; no stale hints or duplicate actions; warnings and intervention levels behave as described. |
| Live Abstich | In an ordinary closed-display non-Ouvert suit or Grand series at a human decision with multiple legal actions, deliberately select a legal hypothetical candidate; verify selection neither plays nor recommends it. Answer before viewing feedback; separately request help, skip, return unanswered and reopen. Try recording menus while the question is open and after Return; cancel each available menu. Try unsupported and illegal/stale candidates, then advance the decision and retry old commands. Inspect the separate live-Abstich overview in German and English. Restart with saved observations; confirm reset with active and parked series, reopen and present a new question. Compare cards, deal, recording, scores and series before and after. | Certainty, derivation, counting aid, coach results including late results and replay panels stay hidden until answer or requested help. Help remains sticky; skip reveals no evidence and earns no score; return preserves unanswered state and reopening preserves settlement/help. Empty analysis is unsupported, never expected Absent; unsupported selections remain separate from supported scored responses, and illegal/stale candidates and old commands are refused. Both languages show the same separate live-Abstich counts, split by assistance; these deliberately entered, positive-risk-selected observations cannot estimate ordinary-play risk/error prevalence or learning effects. Saved observations survive restart; attempt IDs are never reused across restart/reset; reset clears active/parked questions, prevents old-answer resurrection and starts new presentations unanswered. Recording menus remain unavailable while the live question is open and become cancellable after Return. Returning restores legal game actions without changing cards, deal, recording, scores or series. |
| Lessons and review | Exercise all six lessons, learning focus and historical decision moments. Compare figures off with explicitly requested figures. | Wording and uncertainty are understandable; historical context is clear; figures follow the opt-in behavior. Agents inspect the actual wording and interaction; optional learner research is separate. |
| Counting aid and quiz | Toggle aid/quiz/coach combinations; answer correctly and incorrectly, skip, disable and change language/settings. | Counts and feedback are understandable; assistance does not reveal a pending answer; dismissing a question preserves the turn. Package smoke: the extracted executable opens the coach/aid/quiz question and answers it through an app command in German and English ([packaging](packaging.md#automated-evidence-and-remaining-release-checks)); clicks remain widget-test coverage. The same smoke is configured to play a 36-game series to its result, an open-table game and the recording viewer in both languages; replay review/practice clicks stay with widget tests. Record actual package-run evidence before accepting the full series and quiz-answer extension. |
| Open mode and replay | Play openly; save/load a game; navigate both directions; retry the same deal with different actions. | Views and hindsight are clearly labelled; navigation is usable; recordings and original scores stay intact. |
| History and training | Restart with saved progress; reset it deliberately; enter, retry and leave training; compare assisted, repeated and new tasks. | Persistence and reset match their descriptions; practice does not corrupt series scores; progress distinguishes assistance and repetition. |
| Optional model packages | If included, play offline with each shipped backend; exercise missing, corrupt and version-incompatible (format/encoder) model recovery, fallback, coach and language resources. Agents run the package smoke (`scripts/package.py`: Neural and Hybrid decisions and missing, corrupt, `format` and `encoder` model fallback probes) and the skat-app loader tests; Windows CI is the Windows evidence; the RC workflow runs the Linux smoke in a network-less container and the Windows smoke with program-scoped outbound Firewall blocks on the installed app and probe (`--block-network`). A curl canary must reach the network before and fail after the rules; `smoke.json` records this. The [post-merge RC run](https://github.com/naturelle137/grand-coach/actions/runs/44699223650) verified the Windows block on commit `2521e1e92dd452df6452d108fc9a397631d8404e`; it is a program-level block, not an isolated network. | Backend choice and recovery are clear; packaged resources work offline, a rejected model reports its reason, the corresponding Classic level plays on and user data is unchanged. Learned levels stay labelled experimental; target-machine latency/RSS is reported only if measured, otherwise the missing hardware is stated. |

## Learner sessions

Optional research only, never a release checklist prerequisite. If undertaken, use [coach-validation.md](coach-validation.md#b-coach-learner-uat) for
comprehension/usability and its [unaided-task protocol](coach-validation.md#c-initial-and-delayed-unaided-task-protocol)
for initial and delayed attempts on new positions. Freeze task versions,
expected answers, scoring, assistance rules and the follow-up interval
before collecting results. A participant responds as a learner;
these observations do not establish expert certification or general
learning effectiveness.

## Results and release

Store completed personal learner records privately under
`grand-coach-data/studies/`, never public git or `docs/uat/`. Public technical
platform evidence must omit personal learner data. Each record identifies the
build/package, date, OS/hardware, seed or scenario, settings, steps,
expected and observed result, and any finding. Record the actual interval
and prior exposure for learner follow-up sessions.

Fix technical release-blocking findings and repeat affected checks before release.
Record green required CI, one fresh independent review and protected PR integration.
Document remaining limitations, including absent learner observations, without
claiming comprehension, learning efficacy or transfer. This plan authorises no
release or publication.
