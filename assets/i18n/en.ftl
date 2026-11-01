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
option-see-all-description = God Mode: shows the opponents' cards and the Skat face up. Switch it off to play without looking.
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
        [call] Bidding or holding here commits you to at least { $value }.
       *[declarer] The bid is { $value }.
    } A game worth less than that is lost as overbid (ISkO), however many points you take.
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
