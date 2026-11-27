# SPDX-License-Identifier: CC-BY-SA-4.0
# English user interface texts of Grand-Coach (D-32).

## Menu and settings

menu-game = Game
menu-new-series = New series …
menu-end-series = End series
menu-settings = Settings
setting-language = Language
language-de = Deutsch
language-en = English
setting-deck = Card faces
deck-french4 = French, four colours (tournament)
deck-french2 = French, two colours
deck-german = German suits (Altenburg pattern)
setting-developer = Developer options
setting-show-seed = Show seed

## New series

new-series-title = New series
series-fixed = Series with a fixed number of games
series-games = games
series-free = Free play
option-kontra-re = With Kontra and Re
option-see-all = See all cards
option-see-all-description = Shows the opponents' cards and the Skat face up. Switch it off to play without looking.
option-open-display = Open display: show all hands and the Skat
option-open-display-description = Open display: you see every hand and the Skat while you play. The opponents keep playing with only their own cards, and the coach keeps judging only from your own cards as at a closed table. Levels, seeds and scores stay the same.
option-open-display-god = God Mode always shows every card.
option-open-display-mephisto = Mephisto still sees every card, with or without open display.
status-open-display = Open display: all cards
status-coach-own-view = Coach: your cards only
status-coach-real-deal = Coach: all cards (God Mode)
coach-basis-own-view-open = The table shows every card, but the coach judges only from what you could know at a closed table.
coach-basis-real-deal = God Mode: the coach evaluates the options in the real deal with every card.
series-seed = Seed:
seed-description = The seed is the start value of the random generator: the same seed deals the same cards and, when you play the same way, makes the opponents decide the same way, so a series can be replayed. Allowed are whole numbers from 0 to 18446744073709551615.
seed-invalid = Not a valid seed: enter a whole number from 0 to 18446744073709551615.
button-start = Start
button-cancel = Cancel

## Opponents

mode-levels = Choose each opponent
level-rookie = Rookie
level-rookie-description = Rookie: knows the rules only in part and now and then misjudges the bidding, the game choice, the discard or a card.
mode-goldfish = Goldfish
mode-goldfish-description = Goldfish: one opponent, chosen at random, plays strong but forgets which cards have been played; the other plays Medium.
mode-mephisto = Mephisto
mode-god = God Mode
mode-god-description = God Mode: you see every card, the Skat included, and play against Mephisto. The coach sees every card too; it still does not tell you the move but helps you think.
mode-neural = Neural (experimental)
mode-neural-description = Experimental Easy/Medium/Strong levels: seeded temperature policy, without measured ordering or equal strength to Classic. Bidding, Skat, announcements, Kontra and unsupported decisions use the selected Classic level. Model and levels apply to the whole new series; retrying model loading requires an app restart.
mode-neural-missing = Model missing, damaged or different from the recording: Classic uses the selected levels (legacy Neural tables: Strong).
opponent-left = Left opponent:
opponent-right = Right opponent:
level-easy = Easy
level-medium = Medium
level-strong = Strong
level-easy-description = Easy: plays by simple rules of thumb and does not calculate ahead.
level-medium-description = Medium: works through a few possible card distributions for card play and Kontra/Re; bids, discards and announces by rules of thumb.
level-strong-description = Strong: like Medium, but works through four times as many possible card distributions. The strongest level; thinks a little longer.

## Status line

status-game = Game { $number } of { $total }
status-game-free = Game { $number }
status-dealer = Dealer: { $player }
status-turn = To move: { $player }
status-bid = Bid: { $value }
status-declarer = Declarer: { $player }
status-seed = Seed { $seed }
status-levels = Opponents: { $left } (left), { $right } (right)
status-opponents = Opponents: { $mode }
status-waiting = { $player } is thinking …

## Players and seats

player-human = You
player-left = Left
player-right = Right
seat-vorhand = Forehand
seat-mittelhand = Middlehand
seat-hinterhand = Rearhand
label-declarer = declarer
label-cards-tricks = { $cards } cards, { $tricks } tricks
label-open-cards = Open cards:
label-skat = Skat
label-own-skat = Discarded:

## Phases

phase-reizen = Bidding
phase-skataufnahme = Hand or pick up the Skat
phase-druecken = Discarding
phase-ansage = Announcing the game
phase-spiel = Card play
phase-beendet = Game over
phase-eingepasst = Thrown in

## Cards and games

suit-kreuz = Clubs
suit-pik = Spades
suit-herz = Hearts
suit-karo = Diamonds
suit-eichel = Acorns
suit-laub = Leaves
suit-schellen = Bells
suit-cross = Crosses
suit-mond = Moons
suit-stern = Stars
name-alias = { $name } ({ $alias })
rank-sieben = Seven
rank-acht = Eight
rank-neun = Nine
rank-zehn = Ten
rank-bube = Jack
rank-dame = Queen
rank-koenig = King
rank-ass = Ace
card-name = { $rank } of { $suit }
game-grand = Grand
game-null = Null
contract-hand = Hand
contract-schneider = Schneider announced
contract-schwarz = Schwarz announced
contract-ouvert = Ouvert
kontra-level-kontra = Kontra
kontra-level-re = Kontra and Re

## Bidding and actions

call-bid = { $value }
call-hold = Yes ({ $value })
call-pass = Pass
button-bid = Bid { $value }
label-jump-bid = Jump to:
button-jump = Jump
button-hold = Yes, hold { $value }
button-pass = Pass
button-hand = Play Hand
button-take-skat = Pick up the Skat
hint-discard = Choose two cards to discard.
button-discard = Discard
label-announce = Announce the game:
button-announce = Announce
button-kontra = Kontra
button-kein-kontra = No Kontra
button-re = Re
button-kein-re = No Re
hint-play = Your turn: play a card.
button-next-game = Next game

## Tricks

trick-current = Current trick
trick-last = Last trick
trick-winner = { $player } takes the trick ({ $augen } card points)

## Results

result-title = Result
result-won = { $player } wins { $game }.
result-lost = { $player } loses { $game }.
result-augen = Declarer's card points: { $augen }
result-tricks = Declarer's tricks: { $count }
result-spitzen-mit = With { $count }
result-spitzen-ohne = Without { $count }
result-schneider = Schneider
result-schwarz = Schwarz
result-value = Game value: { $value }
result-ueberreizt = Overbid: bid { $bid }, scored as { $value }
result-score = Score: { $score }
result-points = { $player }: { $points } points (Seeger-Fabian)
result-eingepasst = Everybody passed – the deal is thrown in.

## Score sheet and series

sheet-title = Score sheet
sheet-number = No.
sheet-game = Game
sheet-total = Total
sheet-eingepasst = thrown in
series-finished = Series over after { $games } games
series-rank = { $rank }. { $player }: { $total } points ({ $won } won, { $lost } lost)

## Errors

error-not-your-turn = It is not your turn.
error-no-opponent-to-move = No opponent is to move.
error-stale-opponent-move = The opponent move no longer fits the current position.
error-game-in-progress = The current game is not over yet.
error-series-finished = The series is over. Start a new series.
error-not-free-play = Only a free-play series can be ended early.
error-game-over = The game is already over.
error-unexpected-action = That is not possible during “{ $phase }”.
error-invalid-bid-value = { $value } is not a valid bid.
error-bid-too-low = { $value } is not above the current bid of { $current }.
error-invalid-discard = Discard exactly two cards from your hand.
error-invalid-announcement = This combination of game and announcements is not allowed.
error-null-value-below-bid = This Null game is only worth { $value }, but the bid is { $bid }.
error-card-not-in-hand = { $card } is not in your hand.
error-must-follow-suit = You cannot play { $card }: you must follow suit.
error-kontra-not-allowed = Kontra is not allowed here.

## Coach

menu-coach = Coach
setting-coach-level = Coach speaks up
coach-level-off = Off
coach-level-on-request = On request
coach-level-important = At important decisions
coach-level-every-move = At every move
setting-coach-warnings = Warn before weak moves
setting-coach-warnings-description = Holds back a clearly weaker move, explains why and lets you play it anyway or decide again.
setting-coach-figures = Show figures
setting-coach-focus = Learning focus
setting-coach-focus-description = One topic the coach looks at first in this game. You choose it yourself; the coach makes no diagnosis. It stays for the next game until you change it.
coach-focus-none = No focus
coach-focus-current = Your focus: { $focus }
coach-concept-hand-structure = Own Jacks, trumps and Spitzen
coach-concept-game-value = Game value and bid
coach-concept-proven-void = Who could not follow suit
coach-concept-remaining-trumpf = Where the trumps are
coach-concept-trumpf-vulnerability = Can a full side card be trumped?
coach-concept-partner-overtake = Can the partner take over?
coach-question-compare = Check every option against what you know for sure: what does it change for you and for the others?
coach-status-no-lesson = The coach has no checked lesson for this decision. Hints are available on request.
button-coach-skip = Hide
coach-moment-focus = your focus
coach-moment-chosen-provisional = You chose { $action }. The analysis did not finish here, so the model does not rate the move.
coach-title = Coach
button-coach-considerations = What should I consider?
button-coach-situation = What is the situation?
button-coach-figures = Show the figures
coach-section-situation = The situation
coach-situation-forced = Only one move is possible.
coach-situation-one = One option is clearly better than all others. Which one?
coach-situation-several = { $count } options are about equally good.
coach-situation-unclear = The situation cannot be judged reliably.
coach-section-figures = Figures per option (not judged – compare them yourself)
coach-note = Your last move ({ $action }): { $note }
coach-note-good = good.
coach-note-about-equal = about as good as the best.
coach-note-somewhat-weaker = somewhat weaker than the best.
coach-note-clearly-weaker = clearly weaker than the best.
coach-note-forced = there was no choice.
coach-note-uncertain = no reliable judgement.
coach-note-computing = the coach is still judging it …
coach-debriefing-title = Debriefing
coach-debriefing-computing = The coach is still evaluating { $count } decisions …
coach-debriefing-none = No decision in this game had a checked lesson.
coach-moment-trick = Trick { $trick }
coach-moment-chosen-best = You chose { $action }. The model rates it highest here.
coach-moment-chosen-equivalent = You chose { $action }. The model rates it about as high as { $best }.
coach-moment-chosen-worse = You chose { $action }. The model rates it somewhat lower than { $best }.
coach-moment-chosen-weak = You chose { $action }. The model rates it clearly lower than { $best }.
coach-moment-chosen-unrated = You chose { $action }. The model gives no reliable rating for it.
coach-moment-no-reason = The lesson does not explain the model's rating; the model's comparison is in the analysis details.
coach-context-position = Then: you in { $seat }, { $role }; dealer { $dealer }.
coach-context-bid = Bid so far: { $value }
coach-context-no-bid = No bid yet.
coach-context-declarer = Declarer: { $player }, bid { $value }
coach-context-contract = Game: { $game } by { $player }, bid { $value }
coach-context-kontra = Announced: { $level }
coach-context-own-skat = Discarded: { $cards }
coach-context-open = Open cards of { $player }: { $cards }
coach-context-hand = Your cards before the move:
coach-context-trick = Trick { $trick } so far:
coach-context-lead = You lead trick { $trick }.
coach-context-history = History up to this decision
coach-context-calls = Bidding so far: { $calls }
coach-context-no-calls = Bidding so far: no bid yet.
coach-context-call = { $player } { $call }
coach-context-past-trick = Trick { $trick }: { $cards } – { $winner } takes it ({ $augen } card points)
coach-context-card = { $player } { $card }
coach-context-no-tricks = No trick played yet.
coach-details = Analysis details
coach-details-comparison = Model comparison of { $action } with { $other }:
coach-details-rating = Model rating of { $action }: { $judgement }
coach-metric-label-win = Winning chance ({ $party })
coach-metric-label-points = Ranking value (expected Seeger-Fabian points)
coach-metric-label-points-defender = Ranking value (expected Seeger-Fabian points plus half of the declarer's lost game points)
coach-metric-label-declarer = Declarer's game points (incl. Kontra/Re)
coach-metric-label-thrown-in = Chance that all pass
coach-moment-trap = The model rates { $trap } clearly lower here.
coach-moment-trap-loss = Gap in the model's ranking value: about { $points } points.
coach-status-analysing = The coach is thinking …
coach-status-computing = The coach is still working on your question …
coach-status-idle = Ask the coach whenever you want help.
coach-status-waiting = The coach speaks up at your next decision.
coach-unavailable-cancelled = The analysis was cancelled. Ask again.
coach-unavailable-failed = No analysis is possible for this decision.
coach-checking = The coach is checking your move: { $action }
button-coach-proceed = Play it anyway
button-coach-skip-check = Play without check
button-coach-reconsider = Back to the decision
coach-warning-title = Are you sure?
coach-warning-text = { $action } is likely clearly weaker than the best option. Think again.
coach-warning-loss = Expected loss against the best move: about { $points } points (Seeger-Fabian).
coach-action-bid = Bid { $value }
coach-action-hold = Hold { $value }
coach-action-discard = Discard { $cards }
coach-action-announce = Announce { $game }
coach-decision-bid = Bidding: bid or pass
coach-decision-answer = Bidding: hold or pass
coach-decision-pickup = Play Hand or pick up the Skat
coach-decision-discard = Discarding
coach-decision-announce = Announce the game
coach-decision-card = Play a card
coach-decision-kontra = Kontra or not
coach-decision-re = Re or not
coach-role-bidder = no role yet
coach-role-declarer = you are the declarer
coach-role-defender = you are a defender
coach-objective-undecided = No game announced yet: each option is valued by how the following game is likely to end.
coach-objective-augen = Goal: at least { $target } Augen.
coach-objective-augen-defender = Goal: keep the declarer below { $target } Augen.
coach-objective-all-tricks = Goal: every trick (Schwarz announced).
coach-objective-all-tricks-defender = Goal: take at least one trick (Schwarz announced).
coach-objective-no-trick = Goal (Null): take no trick at all.
coach-objective-no-trick-defender = Goal (Null): force the declarer to take a trick.
coach-important = Important decision: an obvious option costs clearly.
coach-provisional = Provisional: the work budget did not cover every distribution – no reliable reading.
coach-section-facts = What you know for sure
coach-section-inferred = What is likely (estimated, not certain)
coach-section-options = Possible moves (not judged)
coach-section-basis = What the estimate rests on
coach-fact-void = { $player } has no { $class } left (did not follow suit).
coach-fact-trumps-out = { $count } trumps are still with the others or in the Skat.
coach-fact-augen-won = Augen in tricks: declarer { $declarer }, defenders { $defenders }.
coach-fact-augen-out = { $augen } Augen are still with the others or in the Skat.
coach-fact-skat-known = The Skat holds: { $cards }.
coach-fact-bid-shown = { $player } bid up to { $value }.
coach-fact-kontra-eligible = { $player } may still say Kontra.
coach-fact-trumpf-unseen = You have not seen { $count } trumps yet.
coach-fact-structure = { $game }: you hold { $count } trumps.
coach-fact-game-values = { $game }: possible game values { $value }.
coach-fact-commitment = The game must be worth at least { $value }.
coach-section-lesson = What to look at
coach-section-lesson-review = What is certain
coach-lesson-class-trumpf = trumps
coach-lesson-remaining-trumpf-question = Count: which trumps have you not seen yet, and where can they be?
coach-lesson-remaining-trumpf-total = { $kind ->
        [grand] In Grand only the { $count } Jacks are trumps.
       *[suit] In a { $game } game there are { $count } trumps: the 4 Jacks and the 7 { $game } cards.
    }
coach-lesson-remaining-trumpf-own = In your hand: { $count ->
        [0] none.
       *[other] { $count } ({ $cards }).
    }
coach-lesson-remaining-trumpf-played = Already played: { $count ->
        [0] none.
       *[other] { $count } ({ $cards }).
    }
coach-lesson-remaining-trumpf-skat = In the Skat you discarded: { $count } ({ $cards }). Nobody holds them.
coach-lesson-remaining-trumpf-revealed = Open in the declarer's hand: { $count } ({ $cards }).
coach-lesson-remaining-trumpf-unseen = Not seen: { $count ->
        [0] none. No trump is concealed any more.
       *[other] { $count } ({ $cards }), in a concealed hand{ $skat ->
            [unknown] {""} or in the Skat.
           *[known] . You know the Skat.
        }
    }
coach-lesson-remaining-trumpf-exception = { $skat ->
        [unknown] Note: this does not tell you who holds the unseen trumps. They can be spread over the concealed hands or lie in the Skat.
       *[known] Note: trumps in your own Skat are not the opponents' trumps. How the unseen ones are spread over the concealed hands is not known from this.
    }
coach-lesson-remaining-trumpf-exception-none = Note: this only tells you about trumps, not about the side suits.
coach-lesson-remaining-trumpf-transfer = At the next trump trick: how many trumps will still be unseen after it?
coach-lesson-proven-void-question = Did someone just fail to follow suit? What do you now know for sure about their cards?
coach-lesson-proven-void-evidence = Trick { $trick }: { $player } played { $card } to { $led } and so did not follow { $class }. A player who can follow suit must (ISkO) – { $player } has no { $follow ->
        [trumpf] { $class }
       *[suit] { $class }
    } left.
coach-lesson-proven-void-exception = { $case ->
        [trumpf] This only rules out trumps. Who wins a later trick does not follow from it.
        [null] Null has no trumps: a player who is void discards. Who takes a later trick does not follow from it.
       *[suit] This only rules out { $class }. A player who is void may trump or discard – whether they do and who wins the trick does not follow from it.
    }
coach-lesson-proven-void-transfer = When { $class } is led again: who can still follow?
coach-section-lesson-mechanism = What was possible in this trick
coach-lesson-trumpf-vulnerability-evidence = { $player } plays after your { $card } in this trick. { $certainty ->
        [certain] It is certain that { $player } holds no { $suit } and at least one trump (this follows from the cards you have seen and the proven voids), so { $player } can trump it.
       *[possible] { $player } can only trump it while holding no { $suit } and at least one trump. Both are possible, not certain: you have not seen { $count } cards of { $suit } and { $trumps } trumps.
    }
coach-lesson-trumpf-vulnerability-rule = Rule (ISkO): a player who can follow suit must. Only a player without { $suit } may trump – or discard instead.
coach-lesson-trumpf-vulnerability-exception = Note: being able to trump does not mean they will. A possible trump-in alone does not show that drawing trumps first would have been better; the rating of your move above is a separate model judgement.
coach-lesson-trumpf-vulnerability-transfer = Before you play an Ace or a Ten of a side suit: who plays after you, can they be out of that suit, and can they still hold trumps?
coach-lesson-partner-overtake-evidence = { $declarer } led { $best } as declarer; after your { $card } it still takes the trick. { $player } plays last.
coach-lesson-partner-overtake-ways = { $player } can beat it { $way ->
        [both] with a higher card ({ $cards }) or, holding no { $class }, with a trump
        [trump] only when holding no { $class }, with a trump
       *[follow] with a higher card ({ $cards })
    }. { $certainty ->
        [certain] That is certain: whatever the concealed cards, { $player } holds such a card.
       *[possible] That is possible, not certain: it depends on cards you have not seen.
    }
coach-lesson-partner-overtake-rule = { $case ->
        [trumpf] Rule (ISkO): { $player } must follow with a trump if possible; only a higher trump wins.
       *[suit] Rule (ISkO): { $player } must follow { $class } if possible; then only a higher { $class } card wins. Without { $class }, any trump wins.
    }
coach-lesson-partner-overtake-exception = Note: being able to beat it does not mean your partner will. This says nothing about later tricks or about which card you should have played.
coach-lesson-partner-overtake-transfer = When the declarer leads and your partner sits behind you: with which cards can your partner still beat the declarer's card?
coach-lesson-spitzen = { $case ->
        [mit] with { $count }
       *[ohne] without { $count }
    }
coach-lesson-preplay-model = The rating of your move above is a coarse model estimate: before card play the model simulates only a few deals, and in them everyone plays as if they saw all cards.
coach-lesson-hand-structure-question = For each suit and for Grand: which Jacks and how many trumps do you hold – and with or without how many?
coach-lesson-hand-structure-game = { $game }: { $kind ->
        [grand] { $count } Jacks
       *[suit] { $count } trumps
    } ({ $cards }), { $value }.
coach-lesson-hand-structure-discarded = Of these, in the Skat you discarded: { $count } ({ $cards }). They count for the matadors but cannot be played.
coach-lesson-hand-structure-range = { $case ->
        [fixed] The unknown Skat cannot change this count.
       *[range] With the unknown Skat: { $min } to { $max } matadors possible (with or without), up to { $count } { $kind ->
            [grand] Jacks
           *[suit] trumps
        }.
    }
coach-lesson-hand-structure-skat = { $skat ->
        [unknown] The Skat is unknown. Its two cards count for the declarer's matadors (also in a Hand game), so the count can still change.
        [held] You hold twelve cards. The matadors count all twelve, including the two you put away; only the ten you keep are played.
       *[discarded] The two cards you put away still count for the matadors but are not played.
    }
coach-lesson-hand-structure-exception = Note: these counts describe your cards. They do not say whether a game is won, which game to play, how high to bid or what to put away.
coach-lesson-hand-structure-transfer = With your next hand: for each suit and for Grand, which Jacks and how many trumps do you hold – and with or without how many?
coach-lesson-game-value-question = What is each of your possible games worth with your cards – and what can the unknown Skat still change?
coach-lesson-game-value-bid = { $case ->
        [call] Looking back at the position before your decision: the lowest available bid or hold committed you to { $value }. A higher jump bid commits you to its value.
       *[declarer] The bid is { $value }.
    } A game worth less than the actual bid is lost as overbid (ISkO), however many points you take.
coach-lesson-game-value-game = { $game }: { $value }
coach-lesson-game-value-entry = { $kind ->
        [plain] with pickup
        [hand] Hand
        [ouvert] Ouvert
       *[handouvert] Hand Ouvert
    } { $value } ({ $case ->
        [reached] reaches it
        [skat] only with a suitable Skat
        [play] only with Schneider or Schwarz made in play
       *[never] cannot reach it
    })
coach-lesson-game-value-skat = { $skat ->
        [unknown] The Skat is unknown: its cards count for the matadors, so a range goes from the lowest to the highest value any Skat can give.
       *[known] You know the Skat: the matadors and so these values are fixed.
    }
coach-lesson-game-value-levels = Values without Schneider or Schwarz in play. Each made in play adds one level; announced (Hand only) each adds two. Ouvert in a suit game or Grand needs Hand with Schneider and Schwarz announced; Null has fixed values.
coach-lesson-game-value-kontra = Kontra and Re double the points written down, not the game value: they do not change whether the bid is reached.
coach-lesson-game-value-exception = Note: reaching the bid only rules out losing by overbidding. It says nothing about whether you win, and no game or bid is named here as better.
coach-lesson-game-value-transfer = At the next bid value: which of your games still reach it for sure, and which only with a suitable Skat?
coach-holder-skat = The Skat
coach-class-trumps = trumps
coach-class-jacks = Jacks
coach-inferred-certain = { $holder } certainly holds { $count } { $class } (follows from the rules).
coach-inferred-hardly = { $holder } probably holds hardly any { $class }.
coach-inferred-about-one = { $holder } probably holds about one of the { $class }.
coach-inferred-several = { $holder } probably holds several { $class }.
coach-inferred-figures = (on average { $expected }, between { $min } and { $max })
coach-judgement-best = best
coach-judgement-equivalent = about as good
coach-judgement-worse = somewhat weaker
coach-judgement-weak = clearly weaker
coach-judgement-screened = only roughly checked
coach-judgement-not-evaluated = not evaluated
coach-party-own = your side
coach-party-declarer = you as declarer
coach-party-defenders = you defenders
coach-metric-win = Winning chance ({ $party }): { $chance }
coach-metric-score = expected list points (Seeger-Fabian): { $score } ± { $se }
coach-metric-game-points = expected game points of the declarer: { $points }
coach-reason-win-more = wins more often than { $other }
coach-reason-win-less = wins less often than { $other }
coach-reason-points-more = scores more points on average than { $other }
coach-reason-points-less = scores fewer points on average than { $other }
coach-reason-declarer-more = more game points for the declarer than { $other }
coach-reason-declarer-less = fewer game points for the declarer than { $other }
coach-reason-thrown-in-more = more often thrown in than { $other }
coach-reason-thrown-in-less = less often thrown in than { $other }
coach-reason-same = leads to the same result as { $other }
coach-reason-uncertain = The difference is within the uncertainty of the estimate.
coach-more-options = … and { $count } more options
coach-basis-exact = Every possible distribution of the hidden cards was evaluated (exact within the model).
coach-basis-known = Computed in the real deal: the coach sees every card (God Mode).
coach-basis-sampled = Estimate over randomly drawn distributions of the hidden cards.
coach-basis-sampled-figures = Estimate over { $finished } randomly drawn distributions of the hidden cards.
coach-assumption-uniform = Assumption: the hidden cards are equally likely anywhere the rules allow.
coach-assumption-bidding = Assumption: the opponents bid by rules of thumb; distributions that do not fit their bidding count less.
coach-assumption-double-dummy = Assumption: in card play everybody plays as if seeing every card.
coach-assumption-policy = Assumption: later decisions before card play follow the rules of thumb.
coach-assumption-known = Assumption: none about the hidden cards; the real distribution of every card is known.
coach-assumption-bid-limit = Assumption: a bid value means "go along up to here".
error-no-decision = You have no decision to take right now.
error-coach-off = The coach is switched off.
error-nothing-pending = No move is being held back.
error-action-pending = First decide about the move being held back.
error-stale-coach-result = The analysis no longer fits the current position.
mode-experimental = Experimental
mode-experimental-description = Exact card search with an additional point bound. During card play the coach shows progress after about 200 ms.
coach-preview-live = Preliminary evaluation from { $finished } of { $total } distributions. Search continues.
error-question-pending = A counting question is open: answer it, skip it or switch the quiz off first.
error-practice-attempt = A practice attempt is exactly one game. Return to the series or start a new attempt.
error-no-question = There is no open counting question.
menu-assist = Assistance
setting-aid = Counting aid
setting-aid-description = Shows what you can know for sure about the played cards, the trumps, the Augen and who could not follow suit. Hidden while a counting question is open.
setting-quiz = Counting questions
setting-quiz-description = Asks a counting question at your card-play decision after the third and the sixth trick. Your move waits; you can answer, skip or switch the quiz off.
aid-title = Counting aid
aid-exact = Every figure is exact and follows from what you have seen.
aid-withheld = Hidden while the counting question is open.
aid-played = Played so far (with the trick on the table): { $count } of 32 cards
aid-played-cards = Played cards: { $cards }
aid-trumpf = Trumpf outside: { $outside } of { $total } (yours { $own }, played { $played })
aid-trumpf-discard = Your own discard held { $discarded } Trumpf.
aid-trumpf-skat = The Skat is unknown to you; some of them may lie in it.
aid-trumpf-null = Null has no Trumpf.
aid-trumpf-later = Trumpf: known once the game is announced.
aid-augen = Augen won in tricks: declarer { $declarer }, defenders { $defenders }
aid-augen-later = Augen won: known once the declarer is known.
aid-augen-outstanding = Augen neither in your hand, nor played, nor in your own discard: { $count }
aid-augen-skat = Your own discard counts { $count } Augen (not part of the tricks).
aid-voids = Proven voids
aid-void-line = { $player }: { $classes }
aid-void-none = Nobody has failed to follow suit yet.
aid-void-trumpf = Trumpf
quiz-title = Counting question
quiz-q-trumpf = How many Trumpf are not in your hand, not played and not in your own discard? An unknown Skat counts as outside.
quiz-q-augen-declarer = How many Augen has the declarer won in tricks so far?
quiz-q-augen-defenders = How many Augen have the two defenders won in tricks so far, together?
quiz-q-played = How many { $class } cards have been played so far, including the trick on the table?
quiz-class-trumpf = Trumpf
quiz-hint = Your move waits. The counting aid and the coach stay hidden until you answer or skip.
quiz-value = Your answer: { $value }
quiz-minus-ten = − 10
quiz-minus-one = − 1
quiz-plus-one = + 1
quiz-plus-ten = + 10
quiz-submit = Answer
quiz-skip = Skip
quiz-off = Switch quiz off
quiz-feedback-title = Last counting question
quiz-correct = Correct: { $expected }.
quiz-incorrect = Not quite: you answered { $given }, the exact count is { $expected }.
quiz-skipped = Skipped. The exact count is { $expected }.
quiz-why-trumpf = { $total } Trumpf − { $own } in your hand − { $played } played − { $discarded } in your discard = { $outside }.
quiz-why-trumpf-skat = An unknown Skat may hold some of them.
quiz-why-augen = This side won { $augen } Augen, the other side { $other }.
quiz-why-played = { $played } of the { $size } { $class } cards are played, { $unplayed } are not yet.

# Focused retrieval (#48)
setting-retrieval = Focused retrieval: remaining Trumpf and proven voids
setting-retrieval-level = Support for focused retrieval
retrieval-visible = Counting aid visible
retrieval-request = Count mentally, reveal on request
retrieval-answer = Answer before reveal
retrieval-hint = Your move waits for an answer, skip or switch-off. You can always restore help; the attempt is then recorded as assisted.
retrieval-help = Show help
retrieval-assisted = Help visible – this attempt is assisted.
retrieval-withheld = Counting aid hidden for focused retrieval. Restore it permanently in settings at any time.
quiz-q-voids = How many other players are publicly proven to hold no more cards of the { $class } class?
quiz-why-voids = Other players publicly proven unable to follow the { $class } class: { $count }. Concealed hands are never guessed.
retrieval-enter = Enter a count

# Local game recordings, deliberately separate from live play.
record-save = Save game …
record-open = Open recording …
record-file-help = Local JSON game file (max. 1 MiB), not series resumption. Enter a full file path. Existing files are never overwritten.
record-path = File path:
record-current = Current game
record-completed = Completed game { $number }
record-no-game = No game available
record-export-warning = The file contains all concealed cards, the Skat and decisions.
record-export-consent = Deliberately write concealed information to the file
record-save-file = Save file
record-open-file = Open file
record-title = Recording — read only
record-isolation = The running series is paused. No game actions are available here; comments only on request for a recorded decision.
record-identity = Game { $number } · Seed { $seed } · Created with { $version }
record-incomplete = Incomplete recording: ends at an outstanding decision.
record-all-pass = All passed: no contract and no tricks.
record-finished = Completed. Null may end before the tenth trick; remaining cards appear when revealed.
record-first = First
record-previous = Previous
record-next = Next
record-last = Last
record-boundary = After { $count } actions
record-reveal = Reveal all concealed cards and the Skat
record-privileged = Full information — includes cards unknown at the time; no inference from a player's view.
record-hands = Remaining cards at this position
record-deal = Originally dealt cards
record-bidding = Bidding history
record-tricks = Completed tricks at this position
record-no-tricks = No completed tricks yet.
record-trick = Trick { $number } · Lead: { $leader } · Winner: { $winner }
record-current-trick = Trick in progress
record-close = Close recording
record-error-json = Invalid JSON file or malformed data fields. Please select another file.
record-error-format = This recording schema version is unsupported.
record-error-engine = This rules or dealing version is unsupported.
record-error-history = Invalid recording: legal history, cards or result do not agree.
record-error-large = Recording too large: at most 1 MiB and 512 actions allowed.
record-error-missing = File or target folder not found. Please check the path.
record-error-exists = The path already exists and was left unchanged. Please choose a new file name.
record-error-io = Could not read or fully write the file. Check path and permissions; after a write failure choose a new file name.
record-source = Game selection
record-practice-start = Practise this deal
record-practice-help = Starts a fresh practice attempt before bidding: the same cards, seats, rules and opponents. The recorded moves are not replayed; you decide anew. The series stays paused and unchanged.
record-practice-unavailable = No further attempt can start during a practice attempt.
record-practice-error = Practice unavailable: the recording is invalid or unsupported.
practice-banner = PRACTICE · Game { $number } of the recorded series · Attempt { $attempt }
practice-isolation = Not scored, not saved and does not change the series.
practice-return = Return to series
practice-restart = New attempt
practice-finished = Practice game over. Return to the series or start a new attempt above.
practice-not-scored = Practice: no score sheet entry. The series' score sheet is unchanged.
practice-limit = Maximum number of attempts reached. Return to the series and reopen the recording.
replay-coach-title = Comment on an earlier decision
replay-coach-help = Choose one of your recorded decisions. A comment is computed only on request, one at a time and separate from the running series.
replay-coach-decision = Decision
replay-coach-none-selected = No decision selected
replay-coach-option = Action { $number } · { $when } · { $action }
replay-coach-forced-option = { $label } (only option)
replay-coach-no-decisions = This recording contains no decision of yours yet. There is nothing to comment on.
replay-coach-perspective-historical = Perspective: what you knew then. Only what you could see before this move; concealed cards are estimated.
replay-coach-perspective-hindsight = HINDSIGHT: all cards known. A model comparison in the actual deal using cards unknown at the time; no proof that your move was a mistake then.
replay-coach-request = Request comment
replay-coach-retry = Compute again
replay-coach-hindsight = Hindsight: compare with all cards
replay-coach-historical = Back to what you knew then
replay-coach-cancel = Cancel computation
replay-coach-computing = Computing the comment …
replay-coach-idle = Not computed yet. The comment starts only on request.
replay-coach-before = The table before your recorded move
replay-coach-alternative = Compared with { $action } (model rating: { $judgement }).
replay-coach-forced = You only had { $action }. Without an alternative there is no comparison.
replay-coach-fallback = No checked lesson fits this decision. Only the model comparison is available, no strategic reason.
replay-coach-provisional = Provisional: the analysis did not finish. Small differences decide nothing here.
replay-coach-basis-known = Computed in the actual deal of this recording: all cards known (hindsight).
replay-coach-comment-historical = Comment from your view then: only your cards and the public history up to this move.
replay-coach-comment-hindsight = Hindsight comment: computed with all cards, including those concealed then.
record-reveal-note = Revealing changes only the display, not the comment's perspective.
replay-review-title = Game review: teachable moments
replay-review-help = On request the coach evaluates each of your decisions that had a choice, one after another and only with what you knew then, and selects up to three teachable moments. No card you could not see then is used.
replay-review-request = Review my decisions
replay-review-eligible = { $count } decisions with a choice; limit { $total } s in total, { $each } s per decision.
replay-review-running = Evaluating decision { $current } of { $eligible } …
replay-review-cancel = Cancel review
replay-review-continue = Continue review
replay-review-again = Review again
replay-review-busy = No single comment is computed while the game review runs.
replay-review-single-busy = No game review starts while a comment is being computed.
replay-review-incomplete = This recording ends before the game is over. A review of the whole game is not available; single comments are.
replay-review-no-decisions = This recording contains no decision of yours. There is nothing to review.
replay-review-complete = All { $eligible } decisions with a choice evaluated ({ $forced } without an alternative skipped).
replay-review-cancelled = Cancelled: { $analysed } of { $eligible } decisions with a choice evaluated. The moments come from these only, not from the whole game.
replay-review-time-limit = Time limit reached: { $analysed } of { $eligible } decisions with a choice evaluated. The moments come from these only, not from the whole game.
replay-review-unavailable = { $count } decisions could not be evaluated (limit reached or error); they are missing from the selection.
replay-review-provisional = { $count } evaluations are provisional; they are shown as neutral at most, never as mistakes.
replay-review-none = No evaluated decision had a checked lesson, so there is no moment; this says nothing about how well you played.
replay-review-fewer = { $count } moment(s): no more decisions had their own checked lesson. There is no required number of good or bad moments.
replay-review-show = Show the table before this move
replay-review-perspective = What you knew then: selection and comments use only your view at the time, never hindsight.
replay-review-partial = { $analysed } of { $eligible } decisions with a choice evaluated. The moments come from these only, not from the whole game.
replay-review-none-partial = None of the evaluated decisions had a checked lesson.

# Explicit guided Farbspiel dialogue (#131).
guided-start = Think through this decision together
guided-title = Think it through
guided-orient = Who is currently ahead in this trick?
guided-check = Who still plays after you in this trick?
guided-feed-role = You defend. Your partner is { $seat }. You play last to this trick; nobody follows you.
guided-feed-attempt = Consider any legal card for feeding Augen to the defending party. It may also overtake your partner.
guided-rule-feed-early = Your partner is ahead with a follow-class master: no higher card of that class remains outside your hand and the played cards. The declarer still follows and may trump a side master with a Trumpf; this does not secure the trick.
guided-feed-early-details = Feeding rule: candidates and ordering
guided-feed-early-order = Easy considers ALL legal cards: nontrump first, then MORE Augen; the LAST exact tie in legal order wins, with no rank tie-break. This is a heuristic, with no guarantee of optimal play or winning the game.
guided-rule-feed = Your partner is currently ahead and you play last. Easy considers ALL legal cards: nontrump first, then MORE Augen; the LAST exact tie in legal card order wins, with no rank tie-break. A card may overtake your partner. This is a heuristic, with no guarantee of optimal play, winning the game or who leads later.
guided-null-role = Null game: you are the declarer. There is no Trumpf; Buben follow their printed suit. Rank order: 7, 8, 9, 10, Bube, Dame, König, Ass. You play last to this trick; nobody follows you.
guided-null-no-winners = No legal card takes this trick; all leave the current winner ahead.
guided-null-winners = Cards that take this trick: { $cards }. These also remain playable.
guided-null-attempt = Which legal card leaves the current winning card ahead? A card of another suit cannot win this trick.
guided-rule-null = You are the Null declarer and play last. Easy considers only the cards that leave the current winner ahead and plays the highest-ranked of them, comparing rank within each card's own suit; on an exactly equal rank, the LAST such card in legal card order. A card of another suit never wins the led suit, however high it ranks. Any card that would take the trick remains playable. This is a heuristic, with no guarantee of optimal play, winning the game or who leads later, and it says nothing about who holds unseen cards.
guided-null-forced-attempt = Every legal card takes this trick. Which card does Easy choose by fewer Augen and lower Null rank?
guided-null-middle-forced-attempt = Every legal card overtakes the lead for now. Which card does Easy choose by fewer Augen and lower Null rank?
guided-rule-null-middle-forced = No legal card leaves the lead ahead. Easy chooses fewer Augen, then lower Null rank, with the first exact tie in legal order. The last defender may still overtake you. Every legal card remains playable; unseen cards are not located.
guided-null-middle-forced-order = Easy chooses the lowest cost: fewer Augen, then lower Null rank, first exact tie in legal order.
guided-rule-null-forced = You are the Null declarer and play last. No legal card leaves the current winner ahead, so you must take this trick. Easy then chooses by fewer Augen, then lower Null rank; the first exact tie in legal order. Every legal card remains playable and takes the trick; this says nothing about unseen cards.
guided-null-proves-void = This card proves a void in the led suit at this play. In Null there is no Trumpf; Buben follow their printed suit.
guided-under = Which card leaves your partner ahead for now?
guided-fallback-attempt = No legal card stays under your partner. Which card does Easy choose by nontrump, fewer Augen and lower rank?
guided-rule-fallback = No legal card stays under your partner. As a fallback, Easy chooses from all legal cards: nontrump first, then fewer Augen and lower rank; the first exact tie. The declarer still plays. This guarantees neither the trick nor the game; every legal card remains playable.
guided-decline-attempt = Easy does not take this trick while no overtaking card is a Trumpf or the highest card of its suit. Which card stays under the declarer by nontrump, fewer Augen and lower rank?
guided-rule-decline = The declarer is ahead. Easy overtakes only with a Trumpf, a highest card of the led suit, or in the last seat; here no overtaking card qualifies. Easy therefore chooses among the cards that leave the declarer ahead: nontrump first, then fewer Augen and lower rank; the first exact tie. This guarantees neither the trick nor the game and says nothing about unseen cards; every legal card remains playable.
guided-decline-last-attempt = No legal card takes the trick. Which card does Easy choose by nontrump, fewer Augen and lower rank?
guided-rule-decline-last = You are the declarer and play last. No legal card takes the trick. Easy chooses nontrump first, then fewer Augen and lower rank; the first exact tie. Every legal card remains playable.
guided-decline-early-attempt = Easy does not overtake this lead while no overtaking card is a Trumpf or the highest card of its suit. One player still follows. Which card keeps the lead untaken by nontrump, fewer Augen and lower rank?
guided-rule-decline-early = You are the declarer and an opponent's lead is currently ahead. One player still follows. Easy overtakes early only with a Trumpf or a highest card of the led suit; here no overtaking card qualifies. Easy therefore keeps a card that does not overtake: nontrump first, then fewer Augen and lower rank; the first exact tie. This guarantees neither the trick nor the game and says nothing about unseen cards; every legal card remains playable.
guided-win = Which card takes over this trick?
guided-act = Now play any legal card on the table.
guided-waiting = Let us observe the actual outcome of this trick.
guided-observe = Does this card prove failure to follow the led class?
guided-complete = The actual observation
guided-unsupported = No guided dialogue is verified for this phase or rule branch. Continue free play.
guided-unsupported-card = Card play · { $game } · { $position }: No guided dialogue is verified for this decision. Return to play and choose any legal card.
guided-unsupported-lead = you lead to the trick
guided-unsupported-second = you play second to the trick
guided-unsupported-last = you play last to the trick
guided-unsupported-revealed = The declarer's hand is revealed.
guided-wrong = That does not follow from the visible position. Try again or reveal the worked rule answer.
guided-retry = Try again
guided-none = Nobody else
guided-rule-card = The rule player plays { $card }.
guided-rule-under = Partner leads, one player follows, no follow-class master. Easy stays underneath: nontrump, fewer Augen, lower rank; first exact tie. This does not guarantee that your partner keeps the trick: the declarer after you may overtake or trump it.
guided-under-ouvert = The declarer's hand is open. Easy's rule does not separately examine it for this choice; check for yourself which cards can still follow.
guided-rule-win = You play last. Among winning cards, Easy prefers nontrump, then lower rank. This does not guarantee winning the game.
guided-candidates = This rule branch's candidates: { $cards }. All legal cards remain playable.
guided-early-attempt = Which legal card overtakes the current lead? One player still follows.
guided-rule-early = One player still follows. Easy chooses among ALL cards currently winning: non-trumps first, then lower rank; first exact tie. The condition applies to Easy's selected card, not to every candidate. Neither the trick nor strategic quality is guaranteed. Every legal card stays free.
guided-early-trumpf = Easy's selected card is trump. In Grand only the four Jacks are trumps; they never follow their printed suit. The remaining player can overtrump with a higher Jack.
guided-early-master = Easy's selected card is a master of its follow class: higher cards are known or in your hand. The remaining player can still trump if they need not follow the side suit. In Grand only the four Jacks are trumps; they never follow their printed suit.
guided-memory = Open card memory
guided-memory-hide = Close card memory
guided-memory-counts = Frozen knowledge: own cards { $own }, played { $played }, unseen { $unseen }.
guided-known-skat = Your known Skat: { $count } cards.
guided-unknown-skat = The Skat is unknown. No holder is assigned to unseen cards.
guided-trumpf = Trumpf outside: { $count } (neither your hand, played cards nor your known Skat).
guided-source = Trick { $trick }: { $seat } plays { $card }.
guided-void-yes = Yes, failed to follow
guided-void-no = No, followed
guided-proves-void = This card proves a void in the led follow class at this play. Buben belong to Trumpf in Farbspiel and Grand, never to their printed suit.
guided-only-card = This card followed the led class and is now known. It does not establish the holder of another unseen card.
guided-winner = Actual trick winner: { $seat }.
guided-unknown = I don't know – show me
guided-reveal = Deliberately reveal the rule answer
guided-return = Skip dialogue / return to free play
guided-figures = Request a separate model comparison
guided-model-title = Model comparison of the original decision – separate from Easy
guided-budget = Game { $game }, decision after { $actions } actions. Work budget: { $samples } distributions, { $nodes } search nodes per task.
guided-model-unavailable = No usable model comparison is available for this decision. The rule answer needs no estimate.
guided-basis-own = Basis: frozen own view, no later card knowledge.
guided-consider = Consider card: { $card }
guided-bid-current = Current public bid: { $bid }.
guided-bid-current-none = No bid has been made yet.
guided-bid-orient = Do you answer the current bid (hold), or must you name a bid yourself?
guided-bid-holds = I answer: hold
guided-bid-asks = I name a bid
guided-bid-threshold-hold = You decide whether to hold { $value }. Does Easy's ceiling for your own ten cards reach { $value }?
guided-bid-threshold-ask = The next standard bid is { $value }. Does Easy's ceiling for your own ten cards reach { $value }?
guided-bid-threshold-none = No standard bid above { $bid } exists. Can Easy's rule still bid?
guided-bid-within-yes = Yes, within the ceiling
guided-bid-within-no = No, above the ceiling
guided-bid-rule = Easy's rule: { $action }.
guided-bid-game = Candidate from your own ten cards: { $game }, ceiling { $ceiling }.
guided-bid-weak = No game reaches the rule's thresholds: ceiling 0, so Easy passes.
guided-bid-branch-suit = Suit game: the strongest suit by Easy's strength count; value = base value × (Spitzen + 1).
guided-bid-branch-grand = Grand: at least two Buben and enough strength; value = 24 × (Spitzen + 1).
guided-bid-branch-null = Null: at most one unsafe suit; fixed value 23.
guided-bid-within = Value at stake { $value } ≤ ceiling { $ceiling }.
guided-bid-above = Value at stake { $value } > ceiling { $ceiling }.
guided-bid-no-value = No higher standard value exists; only Pass is possible.
guided-bid-caveat = The ceiling is a conditional rule of thumb from your own ten cards, not a guaranteed contract value or a promise to win. Spitzen count from your hand only; the hidden Skat cannot be relied on to supply missing ones.
guided-bid-act = Now choose any legal bidding action.
guided-pre-orient-pickup = Skat pickup: from which branch does Easy's rule build the candidate for your own ten cards?
guided-pre-orient-discard = Discard: from which branch does Easy's rule build the candidate for your twelve cards?
guided-pre-grand = Grand (at least two Buben, strength 70 or more)
guided-pre-suit = Suit game (strongest suit at strength 60 or more)
guided-pre-null = Null (at most one unsafe suit)
guided-pre-weak = No threshold reached
guided-pre-threshold-pickup = Candidate: { $game }. Does Easy's strength count for your ten cards reach the Hand threshold { $threshold }?
guided-pre-hand-yes = Yes: Hand
guided-pre-hand-no = No: pick up the Skat
guided-pre-threshold-discard = Candidate: { $game }. Name one card Easy puts away as one of its two discards.
guided-pre-consider = Consider discarding: { $card }
guided-pre-game = Candidate: { $game }.
guided-pre-branch-grand = Grand branch: at least two Buben and Grand strength of 70 or more.
guided-pre-branch-suit = Suit branch: the strongest suit reaches strength 60.
guided-pre-branch-null = Null branch: no suit reaches 60, at most one unsafe Null suit.
guided-pre-branch-weak = No branch reaches its threshold: Easy keeps counting with the strongest suit.
guided-pre-strength-hand = Strength { $strength } ≥ Hand threshold { $threshold }: Easy plays Hand.
guided-pre-strength-pickup = Strength { $strength } < Hand threshold { $threshold }: Easy picks up the Skat.
guided-pre-strength-null = In Null the rule counts no strength (0): Easy always picks up the Skat.
guided-pre-strength-count = Strength count: Trumpf Buben 14 (Kreuz Bube 18), Trumpf Ace and Ten 11, other trumps 8; side Ace 10; side Ten 6 with its Ace or two more cards, else 1; empty side suit 3 (not in Grand).
guided-pre-keeps = Easy keeps trumps, Aces and Tens guarded by their Ace; otherwise short suits and loose Tens go first.
guided-pre-keeps-null = In Null, Easy puts away the highest cards first.
guided-pre-keep-side = { $card }: side card, suit length { $length }.
guided-pre-keep-null = { $card }: high Null card.
guided-pre-keep-trumpf = { $card }: trump, only because nothing weaker is left.
guided-pre-keep-ass = { $card }: Ace, only because nothing weaker is left.
guided-pre-keep-ten = { $card }: guarded Ten, only because nothing weaker is left.
guided-pre-tie = Ranked equally and also correct: { $cards }.
guided-pre-fallback = Easy's wanted action is not legal here; the first legal action is shown.
guided-pre-caveat-pickup = A conditional rule of thumb from your own ten cards, not a promise to win; the hidden Skat stays unknown.
guided-pre-caveat-discard = A conditional rule of thumb; the discarded Augen count for you. Not a promise to win.
guided-pre-act-pickup = Now choose freely: Hand or pick up the Skat.
guided-pre-act-discard = Now put away any two cards.
guided-unsupported-kontra = No guided dialogue is verified for this Kontra/Re position. Continue free play.
guided-unsupported-kontra-ouvert-round = Ouvert Kontra round before the reveal: no guided dialogue. Choose Kontra or No Kontra freely.
guided-unsupported-kontra-ouvert = Kontra and Re in a Null Ouvert game: no guided dialogue. Continue free play.
guided-kontra-public-kontra = Public: { $game }, no Kontra so far (factor ×1). You defend and may say Kontra before your first card.
guided-kontra-public-re = Public: { $game }, a defender said Kontra (factor ×2). As declarer you answer now: Re or No Re.
guided-kontra-orient-kontra = If you say Kontra, by which factor are the game points written down multiplied?
guided-kontra-orient-re = If you answer with Re, by which factor are the game points written down multiplied?
guided-kontra-factor-button = ×{ $factor }
guided-kontra-threshold-kontra = Factor ×{ $factor }. Easy says Kontra with at least { $threshold } Trumpf in its own hand. Do you hold at least { $threshold } Trumpf?
guided-kontra-threshold-re = Factor ×{ $factor }. Easy says Re from strength { $threshold } for its own cards (after a pickup including the discarded cards). Does your strength reach { $threshold }?
guided-kontra-threshold-kontra-grand = Factor ×{ $factor }. In Grand only the four Buben are Trumpf. Easy says Kontra with at least { $threshold } Trumpf in its own hand. Do you hold at least { $threshold } Trumpf?
guided-kontra-threshold-re-grand = Factor ×{ $factor }. In Grand only the four Buben are Trumpf. Easy says Re from strength { $threshold } for its own cards (after a pickup including the discarded cards). Does your strength reach { $threshold }?
guided-kontra-threshold-kontra-null = Factor ×{ $factor }. Null has no Trumpf. Easy's Kontra rule counts the Trumpf in its own hand. Does this rule supply an assessment of a Null game?
guided-kontra-threshold-re-null = Factor ×{ $factor }. Null has no Trumpf. Easy's Re rule compares a strength score built from Trumpf, Aces and Tens. Does this rule supply an assessment of a Null game?
guided-kontra-applies-yes = Yes, the rule applies in Null
guided-kontra-applies-no = No, the rule does not apply in Null
guided-kontra-yes = Yes, the condition holds
guided-kontra-no = No, the condition does not hold
guided-kontra-act-kontra = Now choose freely: Kontra or any legal card. Playing a card declines Kontra; after your first card Kontra is no longer offered.
guided-kontra-act-re = Now choose freely: Re or No Re.
guided-kontra-count-kontra-yes = Trumpf in your hand: { $count } ≥ { $threshold }: Easy says Kontra.
guided-kontra-count-kontra-no = Trumpf in your hand: { $count } < { $threshold }: Easy says no Kontra and plays a card.
guided-kontra-card = Easy's ordinary card rule plays { $card }; this lesson explains only the Kontra condition, not this card.
guided-kontra-count-re-yes = Strength { $count } ≥ Re threshold { $threshold }: Easy says Re.
guided-kontra-count-re-no = Strength { $count } < Re threshold { $threshold }: Easy says No Re.
guided-kontra-count-kontra-grand = Trumpf in your hand: { $count } of the four Buben < { $threshold }: in Grand Easy's Kontra condition never holds, so Easy says no Kontra and plays a card.
guided-kontra-count-kontra-null = Null has no Trumpf, so Easy's Trumpf count gives no assessment: Easy says no Kontra and plays a card.
guided-kontra-count-re-null = Null has no Trumpf and Easy's strength score has no Null value, so its Re condition never holds: Easy says No Re.
guided-kontra-ouvert-view = In an Ouvert game the declarer's hand is revealed to the defenders. Easy's rule below does not use that reveal: it uses only the cards and public facts listed here, and nothing beyond that view is inferred.
guided-kontra-fact-factor = Kontra doubles and Re quadruples the game points written down; neither changes whether the bid is reached.
guided-kontra-caveat-kontra = Five Trumpf are Easy's rule of thumb, not proof of a winning defence; the outcome of the game stays unknown.
guided-kontra-caveat-re = Strength is Easy's heuristic score, not a probability and not a rule of Skat; the outcome of the game stays unknown.
guided-kontra-caveat-kontra-grand = That Easy never says Kontra in Grand is a limit of its rule of thumb, not a rule of Skat: Kontra stays allowed, and the outcome of the game stays unknown.
guided-kontra-caveat-null = This rule of thumb supplies no Null-specific assessment. That is its limit, not a rule of Skat: Kontra and Re stay allowed, and the outcome of the game stays unknown.
guided-kontra-waiting = Let us observe the actual next public step.
guided-kontra-complete = The actual next step
guided-kontra-reply-re = { $seat } answers with Re: the factor is now ×{ $factor }.
guided-kontra-reply-kein-re = { $seat } answers with No Re: the factor stays ×{ $factor }.
guided-kontra-resumed = Card play resumes: { $seat } plays { $card }. Factor ×{ $factor }.
guided-kontra-declined = You played { $card } in trick { $trick }: no Kontra, the factor stays ×{ $factor }. Kontra is no longer offered to you in this game.
guided-ann-public-hand = Public: bid { $bid }, Hand – the Skat stays unknown.
guided-ann-public-pickup = Public: bid { $bid }, Skat picked up – you know your two discarded cards.
guided-ann-orient = Announcement: which branch supplies Easy's preferred game for your cards (after a pickup including your discarded cards)?
guided-ann-threshold = Preferred: { $game }. Does its plain game value (Spitzen from your known cards, plus Hand when playing Hand) reach the bid { $bid }?
guided-ann-fits-yes = Yes: the value is enough
guided-ann-fits-no = No: the value falls short
guided-ann-act = Now freely choose any legal announcement.
guided-ann-preferred = Preferred game: { $game }.
guided-ann-skat-hand = Hand: the Skat stays unknown and is not counted.
guided-ann-skat-known = Counted as well: your discarded cards { $cards }.
guided-ann-fits = Plain value { $value } ≥ bid { $bid }: Easy announces the preferred game.
guided-ann-short = Plain value { $value } < bid { $bid }: the preferred game falls short.
guided-ann-candidates = Suit games and Grand whose value reaches the bid: { $games }.
guided-ann-candidate = { $game } { $value } (strength { $strength })
guided-ann-strongest = Easy takes the strongest of them: { $game }.
guided-ann-tie = Equally strong: { $games }. Easy takes the later one in the order Kreuz, Pik, Herz, Karo, Grand: { $game }.
guided-ann-no-fit = None of Easy's candidates reaches the bid { $bid } with the plain value of your known cards: Easy falls back to Grand. The actual game value, and so whether the game ends up overbid, is still unknown: Spitzen in an unknown Skat and earned levels such as Schneider can raise it.
guided-ann-ouvert = Null without an unsafe suit in your ten cards: Easy plays ouvert.
guided-ann-null-plain = Null with { $count ->
        [one] one unsafe suit
       *[other] { $count } unsafe suits
    } in your ten cards: Easy plays plain Null.
guided-ann-plain = Null ouvert is not available here; plain Null is shown.
guided-ann-caveat = A conditional rule of thumb of Easy from your known cards and the public bid, not a promise to win and not a rating of the announcement.
# Learning history (M8 foundation, D-70)
learning-open = Learning history …
learning-title = Learning history
learning-memory-only = Not saved: no data folder is available, so this history only lasts until the app closes.
learning-file = Saved locally in: { $path }
learning-retention = Kept on this computer across restarts until you reset it. Contains counting events and bounded catalogue/live Abstich response evidence, no full deals or recordings.
learning-damaged-kept = The file { $path } was left unchanged. New events are kept only until the app closes and are not saved, so nothing in the file is overwritten.
learning-set-aside = Rename damaged file and start a new history
learning-set-aside-done = The previous file was kept as { $path }.
learning-save-failed = Could not save the learning history: { $reason }
learning-error-invalid = The learning history file is damaged or inconsistent.
learning-error-format = The learning history file has an unsupported format version (perhaps from a newer version).
learning-error-large = The learning history file is too large.
learning-error-io = The learning history file could not be read or written. Check the folder and permissions.
learning-descriptive = Counts of what was observed – not a measure of skill or understanding.
learning-empty = No counting questions observed yet. Switch on counting questions in the settings or open counting training in the Game menu; this overview fills as you answer, ask for help or skip.
learning-limited = Only { $count } question(s) observed so far – too few for any conclusion.
learning-observed = { $count } questions observed in { $attempts } games or attempts.
learning-full = The learning history is full; new events are not recorded. Reset it to record again.
learning-col-topic = Topic
learning-col-context = Where
learning-col-opportunities = Asked
learning-col-answers = Answers (exact)
learning-col-unaided = Without help (exact)
learning-col-help = Help shown
learning-col-skips = Skipped
learning-of-exact = { $count } ({ $exact })
learning-legend = Every column counts its own event. Answers given with visible or requested help never count as “without help”. Practice attempts, retries of the same deal and counting training on fixed positions are listed separately from series games; none of these rows is a new, unseen task.
learning-topic-trumpf = Remaining Trumpf
learning-topic-voids = Proven voids
learning-topic-augen = Augen won
learning-topic-played = Cards played
learning-context-series = Series
learning-context-practice = Practice
learning-context-retry = Practice retry
learning-reset = Reset learning history …
learning-reset-confirm = Delete the whole learning history permanently? Games, recordings and scores are not affected.
learning-reset-yes = Delete history
learning-reset-no = Cancel
learning-close = Close

# Focused counting training (D-71)
training-open = Counting training …
training-title = COUNTING TRAINING
training-banner = COUNTING TRAINING · { $topic } · Position { $position } of { $count } · Round { $presentation }
training-isolation = Outside the game: your running game waits unchanged. Nothing here counts for the score sheet or is saved as a game.
training-return = Back to the game
training-intro = Practise counting on fixed, legal positions. You only see what the given player saw: their hand, their own discard and the tricks so far.
training-start-trumpf = Practise remaining Trumpf
training-start-voids = Practise proven voids
training-note-trumpf = Farbspiel and Grand. Null has no Trumpf and is not included.
training-note-voids = Public evidence only: who did not follow a led group.
training-practice-only = Every position is a fixed practice position. Repeats are practice, not a new task and no proof of skill. Answers, help and skips go into the learning history.
training-topic = Topic:
training-question = Question
training-result = Result
training-was-assisted = Help was visible in this round; it counts as assisted.
training-retry = Try again
training-next = Next position
training-fixture-note = Fixed practice position, version { $version }. The learning history stores only position, version and your events, no cards.
training-seat-you-declarer = { $seat } (you, declarer)
training-seat-you = { $seat } (you)
training-seat-declarer = { $seat } (declarer)
training-void-seats = Publicly proven for: { $seats }
training-contract = Game: { $game } · bid to { $bid } · declarer: { $declarer }
training-role-declarer = You sit in { $seat } and are the declarer.
training-role-defender = You sit in { $seat } and defend with your partner against the declarer in { $declarer }.
training-tricks = Tricks so far
training-trick = Trick { $number }: { $leader } leads, { $winner } wins.
training-table = On the table
training-you-lead = Still empty – you lead.
training-your-turn = It is your turn. Count before you choose a card.
training-hand = Your hand
training-discard = Your own discard (known to you)
training-skat-unknown = You do not know the Skat.
learning-context-training = Counting training
learning-context-training-repeat = Counting training, repeat
learning-responses-title = Answers by observed result
learning-col-exact = Exact
learning-col-too-high = Too high
learning-col-too-low = Too low
learning-col-unrecorded = Wrong, value not stored
learning-col-skipped = Skipped
learning-split = { $unaided } + { $assisted }
learning-responses-legend = Each cell: without help + with help. “Too high” and “too low” compare your answer only with the exact count stored with it; answers recorded before this version stored no exact count, so a wrong one is listed as “value not stored” and its deviation stays unknown. These categories describe the answer you gave, not why you gave it, and say nothing about skill or play.
learning-compare-title = Earlier and recent answers
learning-compare-sparse = { $topic }: { $count } answered or skipped – a comparison needs at least { $needed }.
learning-col-window = Answers
learning-col-attempts = Attempts
learning-window-earlier = Earlier: no. { $first }–{ $last }
learning-window-recent = Most recent: no. { $first }–{ $last }
learning-attempt-range = #{ $low }–#{ $high }
learning-compare-legend = Per topic, the { $window } answers or skips before the last { $window } are set beside the last { $window }, numbered in the order they were saved. Mixed places and help are shown, not corrected for: a difference between the windows is no trend, progress or transfer to new deals.
learning-provenance-title = Question rules recorded with each question
learning-provenance-lesson = { $family } (version { $version }): { $count } asked
learning-provenance-unversioned = Saved before versions were stored: { $count } asked (version unknown)
learning-family-quiz = In-game counting quiz
learning-family-focused = In-game focused counting
learning-family-training = Counting training
learning-strategic-separate = This section describes counting answers only. Mechanism responses and selected reviewed moves are separate.

## Opponent figures (D-72): names of both pools, shown in their language

figure-de-anna = Anna
figure-de-sabine = Sabine
figure-de-monika = Monika
figure-de-annemarie = Annemarie
figure-de-adelheid = Adelheid
figure-de-hildegard = Hildegard
figure-de-thomas = Thomas
figure-de-michael = Michael
figure-de-peter = Peter
figure-de-stefan = Stefan
figure-de-kunibert = Kunibert
figure-de-friedrich-wilhelm = Friedrich-Wilhelm
figure-en-emma = Emma
figure-en-sarah = Sarah
figure-en-linda = Linda
figure-en-elizabeth = Elizabeth
figure-en-ethel = Ethel
figure-en-mildred = Mildred
figure-en-james = James
figure-en-david = David
figure-en-tom = Tom
figure-en-oliver = Oliver
figure-en-archibald = Archibald
figure-en-bartholomew = Bartholomew

## Table layout and concealed Augen (issue #134)

opponent-strength = Strength: { $level }
opponent-mode = Mode: { $mode }
opponent-portrait = Caricature of { $player }
augen-title = Card points
augen-reveal = Show card points (press and hold)
augen-hidden = Hidden. Press and hold the button to look; releasing hides them at once.
augen-none = No trick completed yet.
augen-declarer = Declarer { $player }: { $augen }
augen-defence = Defenders { $player }: { $augen }
augen-player = { $player }: { $augen }
augen-skat-excluded = Completed tricks only; the Skat is not counted.

focus-trumpf = Focus: Remaining Trumpf
focus-void = Focus: Proven voids
focus-side = Focus: Trumping a full side card
focus-partner = Focus: Partner overtaking
focus-position = Position { $number } of { $count }
focus-unseen = Not previously presented in this app history. An answer without help can be recorded as a first unaided response.
focus-practice = Practice. This presentation receives no credit as a first unseen unaided response.
focus-limits = History knows only exposure in this app file. Resetting, missing or unreadable history does not prove novelty. This is no evidence of learning efficacy or transfer at a real table.
focus-topics = Choose a topic
focus-leader = { $seat } led; cards are shown in play order.
focus-question-abstich = Suppose you play the legal card { $card }: Is a later opponent trumping this full side card possible, certain or absent?
focus-question-overtake = Suppose you play the legal card { $card }: Can your partner, playing later, overtake the declarer in this trick – possible, certain or absent?
focus-judgement-note = Judge only this mechanism. The question recommends no card. “Possible” means some, “certain” all distributions consistent with your view. Other legal cards remain valid.
focus-possible = Possible, but not certain
focus-certain = Certain
focus-absent = This mechanism is absent
focus-count = Exact count
focus-correct = The answer to this question is correct.
focus-incorrect = The answer to this question disagrees with the bounded analysis.
focus-skipped = Skipped; feedback is now visible.
focus-unscored = Unscored: skipped or insufficient analysis.
focus-unavailable = Position or history unavailable; no reliable score or novelty credit.
focus-abstich-evidence = { $seat } can trump only with no { $suit } and at least one Trumpf. There are { $cards } unseen cards of that suit and { $trumps } unseen Trumpf; their holders are unknown.
focus-abstich-certain-evidence = In every distribution consistent with your view, { $seat } holds no { $suit } and at least one Trumpf, so it can trump. There are { $cards } unseen cards of that suit and { $trumps } unseen Trumpf; this follows from counting, not from knowing who holds what.
focus-overtake-evidence = Partner in { $seat }: { $cards } unseen higher cards of the led class are feasible. { $trumping }
focus-trumping-possible = Trumping when void in the led suit is also consistent with the view.
focus-no-trumping = An additional way by trumping is not established here.
focus-boundary-note = An absent mechanism means neither safe nor optimal. A possibility does not predict which cards are actually held or played.
focus-history = Catalogue practice: observed answers
focus-unaided = Without visible help
focus-unseen-correct = First in this file, correct without help
focus-history-row = Attempt { $attempt } · { $task } v{ $version } · { $result } · { $support } · { $exposure }
focus-visible = Analysis visible
focus-request = Think first, reveal on request

mechanism-title = Mechanism responses: observed patterns
mechanism-limits = Conditional Abstich / partner overtaking answers only. No strategic mastery or inferred thoughts. These catalogue rows exclude ordinary games and same-deal practice; live Abstich responses have their own section. Counting stays separate; outside-app exposure is unknown.
mechanism-empty = No catalogue mechanism responses presented in this history.
mechanism-abstich = Conditional Abstich
mechanism-overtake = Partner overtaking
mechanism-first = First app-local presentation
mechanism-repeat = Repeated app-local practice
mechanism-exposure-unknown = Earlier exposure unknown
mechanism-assisted = Help available
mechanism-unaided = Without help
mechanism-eligible = App-local unseen changed case
mechanism-practice = Practice; no unseen-case eligibility
mechanism-denominators = Presented { $presented } · answered { $answered } · scored { $scored } · skipped { $skipped } · unknown { $unknown }
mechanism-patterns = Correct { $correct } · certainty overstated { $overstated } · possibility denied { $denied } · false presence { $false } · certainty understated { $understated }
mechanism-unscored = Unanswered { $unanswered } · unsupported answers { $unsupported }
mechanism-sparse = Fewer than 20 settled observations in this exact task/context: no comparison yet.
mechanism-earlier = Earlier 10 settlements in this task/context
mechanism-recent = Recent 10 settlements: descriptive comparison, no learning-effect claim

mechanism-training = Catalogue training
mechanism-training-repeat = Catalogue training, repeated task

# Reserved learner validation. No task answers or reserved position previews.
validation-entry = Prepare learner validation
validation-title = Learner validation · protocol 1
validation-intro = Four reserved tasks per session. Help, repeats and prior exposure are recorded separately. Skip, unclear and exit are always allowed. Stop on failed saves, wrong versions, broken positions or premature answers. Follow-up is planned after 7–14 calendar days; the actual interval is recorded.
validation-build = Build: { $build }
validation-initial = Deliberately start initial session
validation-delayed = Deliberately start delayed session
validation-prior = Have you seen this exact position before, including outside this app?
validation-none = Never seen before
validation-seen = Seen before
validation-unknown = Unknown
validation-own-words = Own words (optional)
validation-clarity = Clarity / unclear terms (optional)
validation-unclear = Task unclear
validation-save-stop = Validation stopped: this task cannot be reliably saved to durable storage. Return and report the failure. Do not open another task.
validation-exposure-note = Local history cannot establish external novelty. Unknown, missing, migrated or reset data cannot establish new unaided performance. Own words and clarity are assessed separately; these are no mastery or efficacy claims.
validation-complete = These four tasks are settled. Return to the game. Another attempt is practice only.
validation-counts = Opportunities: { $opportunities }; answers: { $answers }; scored: { $scored }; correct: { $correct }; wrong: { $wrong }; unsupported: { $unsupported }; skipped: { $skip }; unclear: { $unclear }; unanswered: { $unanswered }.
validation-support-counts = Opportunities with help: { $help }; assisted: { $assisted }; repeats: { $repeat }; exposure unknown: { $unknown }; first unaided scored answers with reported novelty: { $eligible }, correct: { $correct }.
validation-interval = Actual interval between first initial and delayed entries: { $seconds } seconds. Both UTC timestamps are in local history. This does not assert compliance with the planned calendar interval.
validation-demo = Demonstration using a public practice task – no reserved position or learner result.
validation-unexpected-aid = Unexpected additional aid seen – stop session
validation-stopped = Session stopped without an answer. Return to the game and record the reason locally. After unexpected aid, continue only after technical triage.
validation-position = Task { $number } of 4
validation-first = First recorded presentation. Only the first unaided scored answer with an explicit report of no prior exposure can be considered. Local history alone does not establish novelty.
validation-unclear-result = Recorded as unclear – not scored.

guided-null-middle-role = You play second in Null; one defender still follows. Buben and Zehnen follow their printed suit, without Trumpf.
guided-rule-null-middle = A current loser cannot win this trick for you. An overtake by the last defender only changes the winning defender; Easy’s discard is a heuristic with no game-win guarantee.
guided-null-middle-details = Rule details: candidates and ties
guided-null-middle-order = Easy takes the highest rank among current losers; on equal rank, the LAST card in legal order. Rank is within the card’s own suit: another suit never wins the led suit.
guided-null-middle-no-winners = None of your legal cards overtakes the lead.
guided-null-middle-winners = Cards that currently overtake the lead: { $cards }. A defender still follows.

guided-lead-orient = The table is empty and you lead. What is your role?
guided-lead-declarer = I lead as declarer
guided-lead-defender = I lead as defender
guided-lead-master = Which legal side card has no higher rank outside your hand and played cards?
guided-lead-attempt = Which legal card would you try for Easy's largest-Augen master lead?
guided-lead-definition = Own higher cards do not block a master; Grand Buben are Trumpf, never side-suit followers.
guided-lead-suit-definition = Own higher cards do not block a master; Buben and the announced suit are Trumpf, never side-suit followers.
guided-lead-risk = A void follower may trump. A side master does not guarantee the trick or game.
guided-rule-lead = Easy: { $card }. Most Augen; LAST exact tie in core legal order (heuristic).
guided-lead-details = Predicate and empty-table details
guided-lead-conservative = Known discarded higher cards still block a master; discarded Buben still count as outstanding Trumpf. This does not locate an unseen card.
guided-lead-empty = At an empty table, every legal lead starts ahead. This predicts no future winner.
guided-lead-source = Trick { $trick }: { $seat } has the lead; no card or current winner yet.
guided-lead-observe = Look at both actual replies. Does the last reply prove a void in the led suit?
guided-lead-candidates-risk = Candidates: { $cards }. A void follower may trump; no trick or game win guaranteed.
guided-lead-fixture-label = Fixture component: { $case } / { $stage } (not an ordinary route)
guided-draw-trumpf = Which legal card is Trumpf? In Grand it is the four Buben; in Farbspiel the trump suit also counts.
guided-draw-trumpf-attempt = Which legal Trumpf would you try for Easy's highest-Trumpf lead?
guided-draw-trumpf-definition = You lead as declarer with Trumpf in hand. Easy leads Trumpf when the policy counts Trumpf outside your hand and the played cards.
guided-draw-trumpf-risk = This does not show that a defender holds Trumpf, nor any reply, trick or game result.
guided-rule-draw-trumpf = Easy: { $card }. Highest Trumpf; last exact tie in legal order (heuristic, not optimal play).
guided-draw-trumpf-candidates-risk = Trumpf candidates: { $cards }. Own Skat, unseen cards and defenders stay possible locations; nothing is guaranteed.
guided-draw-trumpf-conservative = Own and played Trumpf are excluded from the count; known discarded Trumpf still counts. An unknown Skat is a possible location, not a proven holder.
guided-draw-trumpf-observe = Look at both actual replies. Does the last reply prove that hand cannot follow what you led (Trumpf or the led suit)?

# Null opening lead (#176)
guided-null-lead-candidates = Which cards may you lead onto this empty table?
guided-null-lead-attempt = Which lead comes first in Easy's cost ordering?
guided-null-lead-ranks = Null has no Trumpf. Printed suits apply, including Buben; rank: 7, 8, 9, 10, J, Q, K, A.
guided-null-lead-limit = Your aim is to avoid tricks. Neither defender's reply is known.
guided-rule-null-lead = Easy minimises (Trumpf, Augen, Null power), keeping the FIRST exact tie in legal order. Augen are a legacy sorting cost, not Null scoring or strategic value.
guided-null-lead-details = Ordering and empty-table limits
guided-null-lead-order = Power counts weaker cards of the printed suit (0–7). A zero-Augen 9 can precede a positive-Augen 10 in another suit: this is not universally the lowest printed rank.
guided-null-lead-empty = Every legal lead is a current-table winner only because the table is empty. This guarantees neither taking nor avoiding the completed trick or winning the game.

guided-suit-early-trumpf = Easy’s selected card is trump: all four Jacks and the announced suit are trumps; a Jack never follows its printed suit. The remaining player may overtrump with a higher trump.
guided-suit-early-master = Easy’s selected card is a master of its follow class: higher cards are known or in your hand. The remaining player may still trump if void in the side suit; all Jacks and the announced suit are trumps.

opponent-engine = Opponent engine:
engine-classic = Classic

live-abstich-start = Consider a hypothetical Abstich
live-abstich-title = Live Abstich question
live-abstich-note = Deliberate observation of your explicit answer. Selecting a candidate does not play or recommend it. Different candidates are separate hypothetical questions at the same decision, not extra game decisions.
live-abstich-select = Choose any legal candidate:
live-abstich-question = If I play { $card }, can { $player }, acting later, trump it in this trick?
live-abstich-unsupported = No supported question for this selection or mode. Empty analysis is not evidence of absence. The game remains unchanged; return to play or select another legal card.
live-abstich-help = Reveal deduction (with help)
live-abstich-assisted = With revealed help (sticky)
live-abstich-return = Return to unchanged game
live-abstich-history = Live Abstich: explicit responses
live-abstich-selection-bias = Deliberately entered and selected for positively supported risk. These counts cannot estimate how often risks or errors occur in ordinary play. No judgement of chosen cards, strategy, mastery or transfer. Candidates at one decision count as separate hypothetical questions.
live-abstich-source = Context: ordinary closed-display series · source: permitted player view, shared Abstich analysis · question/evidence v1. Expected absence and false-presence scores are unsupported. No novelty claim.
live-abstich-empty = No live Abstich observations in this available history. Sparse observations support no trend.
live-abstich-totals = Presented { $presented } · answered { $answered } · supported scored { $scored }
live-abstich-categories = Correct { $correct } · overstated certainty { $overstated } · denied possibility { $denied } · understated certainty { $understated }
live-abstich-remaining = Skipped { $skipped } · unanswered { $unanswered } · unsupported { $unsupported }
live-response-correct = Correct response
live-response-overstated = Overstated certainty
live-response-denied = Denied possibility
live-response-understated = Understated certainty
live-response-skipped = Skipped; no answer scored
live-response-unsupported = Unsupported; no answer scored
live-abstich-definition = Possible means at least one but not all permitted hidden distributions; Certain means all. Neither predicts actual play or reveals a hidden card holder. This question recommends no card.

setting-debrief-model = Experimental: prioritize debriefing with model estimates
setting-debrief-model-description = Changes only the order. Every decision receives Classic analysis. Model estimates are not facts or proof of a mistake. Without a usable model, Classic ordering stays active.
coach-debriefing-partial = Moments already checked. The selection may change as more analyses finish.

mode-hybrid = Hybrid (experimental)
mode-hybrid-description = Experimental proposals are verified by Classic search; Easy verifies with one sample. If the model is unavailable, the selected Classic level takes over. Quality and speed are unmeasured.
guided-null-defender-role = You defend in Null. The declarer is ahead and your partner still plays. There is no Trumpf; Buben follow their printed suit.
guided-null-partner-role = Your partner is ahead; the declarer still plays. In Null there is no Trumpf; Buben follow their printed suit.
guided-null-partner-attempt = Which legal card comes first by Augen, then ascending Null rank? Break ties by legal order.
guided-rule-null-partner = Easy compares all legal cards by Augen, then by ascending Null rank, taking the first tie. This is a simple fallback, not a guarantee of the trick or game.
guided-rule-null-defender = Easy chooses the highest-ranked card that leaves the declarer ahead for now; on equal rank, the last in legal order. Your partner can still overtake. This heuristic does not guarantee winning the game.

guided-null-defender-last-role = You defend in Null and play last. The declarer is ahead. There is no Trumpf; Buben follow their printed suit.
guided-rule-null-defender-last = Easy chooses the highest-ranked card that leaves the declarer ahead, with the last equal rank in legal order. This heuristic does not guarantee winning the game.
guided-null-defender-forced-attempt = Every legal card overtakes the declarer. Which does Easy choose by fewer Augen and lower Null rank?
guided-rule-null-defender-forced-middle = Every legal card overtakes the declarer for now. Easy chooses fewer Augen, then lower Null rank, with the first exact tie in legal order. Your partner still plays and may overtake. Every legal card remains playable; this does not determine the game outcome.

practice-compare = Compare with original
compare-title = Practice attempt vs. original game
compare-isolation = Read-only and in memory only: nothing is scored, saved or added to the learning history.
compare-no-ranking = Your own decisions side by side, each from what you could see then. This does not say which line was better; for that, use the ordinary coach analysis, which shows its source and uncertainty.
compare-all-same = Every own decision matched the original game.
compare-identical = { $count ->
    [one] 1 identical decision (same position, same action)
   *[other] { $count } identical decisions (same position, same action)
}
compare-decision = Decision { $number }
compare-original = Original game
compare-attempt = Practice attempt
compare-chose = Chosen: { $action }
compare-none = No decision at this point in this line.

guided-kontra-ouvert-before = At this decision the declarer’s hand is hidden. After the Ouvert Kontra round it is open to the defenders.
guided-kontra-ouvert-question = Does Easy assess your hand before deciding in this Ouvert round?
guided-kontra-ouvert-act = Choose freely: Kontra or No Kontra.
guided-kontra-ouvert-rule = Easy always chooses No Kontra in this round without assessing the hand. This is a limitation of Easy, not a rule of Skat; the outcome is unknown.
guided-kontra-ouvert-declined = You chose No Kontra before the reveal.
guided-kontra-ouvert-yes = Yes
guided-kontra-ouvert-no = No

guided-null-defender-lead-role = You defend in Null and lead. The declarer must take a trick to lose; neither reply is known.
guided-null-defender-lead-limit = Both replies are unknown. This heuristic guarantees neither the trick nor the game.

guided-feed-ouvert-view = The declarer's hand is open. Easy chooses from your legal cards and the current trick; the open hand does not change this rule.
guided-null-ouvert-defender-lead-role = You defend in Null Ouvert and lead. The declarer's hand is visible; your partner's hand is concealed.
guided-null-ouvert-defender-lead-limit = The visible hand does not determine either reply. This heuristic guarantees neither the trick nor the game.

strategic-title = Reviewed moves: classification of the selected moments
strategic-limits = Only the at most three moments a review selected per recording are counted. They are neither frequencies nor error rates of your play and no evidence of skill or learning effect.
strategic-empty = No reviewed recording in this history yet.
strategic-reviews = Reviews { $reviews } (of these incomplete { $partial }) · moments { $moments }
strategic-counts = Moments { $total } · Decided well { $good } · weaker choice { $bad } · neither { $neutral }
strategic-sparse = Fewer than 20 moments of this topic: no comparison yet.
strategic-earlier = Earlier 10 moments of this topic
strategic-recent = Latest 10 moments: descriptive comparison, no evidence of a learning effect
guided-lead-fallback-attempt = Easy finds no side-suit card without a higher unseen card and has no own Trumpf left to draw. Which legal card is lowest by nontrump, fewer Augen and lower rank?
guided-lead-fallback-definition = A lead card starts the trick ahead. Your own higher cards do not block a master; this view counts only your hand and the cards already played.
guided-lead-fallback-risk = Easy's choice here is a heuristic. It says nothing about unseen cards and guarantees neither the trick nor the game.
guided-rule-lead-fallback = You lead and no legal side-suit card is a master by the counted cards, and no Trumpf is left to draw. Easy leads the lowest legal card: nontrump first, then fewer Augen and lower rank; the first exact tie. This guarantees neither the trick nor the game; every legal card remains playable.
guided-lead-fallback-ouvert = The declarer's hand is open. Easy's rule does not use it for this lead; check for yourself which cards can still follow.
mode-pre-play = Learned pre-play (experimental)
mode-pre-play-description = Experimental imitation model of Classic Strong decides bidding, Skat, discarding, announcements and card play, with the selected Easy/Medium/Strong temperature. It is clearly weaker than Classic Strong. Kontra/Re follow the Classic rules of thumb; any decision the model cannot take uses the selected Classic level. Retrying model loading requires an app restart.

godot-setting-restart-confirm = Start a new series? The current game and series progress will be lost.
