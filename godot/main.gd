# SPDX-License-Identifier: EUPL-1.2
extends Control

var app := SkatApp.new()
var failed := false
var texts := preload("res://i18n.gd").new()
var faces := {}
var figure_keys: Array = []
var cast_keys := ["", "", ""]  # figure key per seat; "" for the human
var portraits := {}
var settings_path := "user://settings.cfg"
var kontra_re := false
var open_display := false
var levels := ["Easy", "Easy"]
var engine := "Classic"
var mode := "Levels"
var show_figures := false
var counting_aid := false
var quiz := false
var coach_level := "OnRequest"
var coach_warnings := false
var learning_path := "user://learning-history.json"
var quiz_id := ""
var pending_setting := {}
var replay_active := false
var replay_index := 0
var replay_total := 0
var recording_save := false
var training_active := false
var discard_selection: Array[int] = []
const COACH_LEVELS := ["Off", "OnRequest", "Important", "EveryMove"]
const COACH_LEVEL_KEYS := ["coach-level-off", "coach-level-on-request", "coach-level-important", "coach-level-every-move"]
const LEVELS := ["Rookie", "Easy", "Medium", "Strong"]
const MODES := ["Levels", "Goldfish", "Mephisto", "GodMode"]
const MODE_KEYS := ["mode-levels", "mode-goldfish", "mode-mephisto", "mode-god"]
const ENGINES := ["Classic", "Neural", "PrePlay", "Hybrid"]
const LEARNING_TOPICS := {"TrumpfOutside": "learning-topic-trumpf", "ProvenVoids": "learning-topic-voids", "AugenWon": "learning-topic-augen", "ClassPlayed": "learning-topic-played"}
const ENGINE_KEYS := ["engine-classic", "mode-neural", "mode-pre-play", "mode-hybrid"]

func _ready() -> void:
	var language := "en" if OS.get_locale_language() == "en" else "de"
	var settings := ConfigFile.new()
	if settings.load(settings_path) == OK:
		var saved = settings.get_value("display", "language", language)
		if saved in ["de", "en"]:
			language = saved
		kontra_re = settings.get_value("game", "kontra_re", false) == true
		open_display = settings.get_value("game", "open_display", false) == true
		counting_aid = settings.get_value("assist", "counting_aid", false) == true
		quiz = settings.get_value("assist", "quiz", false) == true
		show_figures = settings.get_value("assist", "figures", false) == true
		var saved_coach = settings.get_value("coach", "level", "OnRequest")
		if saved_coach in COACH_LEVELS:
			coach_level = saved_coach
		coach_warnings = settings.get_value("coach", "warnings", false) == true
		var saved_engine = settings.get_value("game", "engine", "Classic")
		if saved_engine in ENGINES:
			engine = saved_engine
		var saved_mode = settings.get_value("game", "mode", "Levels")
		if saved_mode in MODES:
			mode = saved_mode
		if mode != "Levels":
			engine = "Classic"  # learned engines need individual levels
		for i in range(2):
			var level = settings.get_value("game", "level_%d" % i, "Easy")
			if level in LEVELS:
				levels[i] = level
		_coerce_rookie()
	if not texts.load_language(language):
		texts.load_language("de")
	$Layout/Language.add_item(texts.tr_key("language-de"))
	$Layout/Language.add_item(texts.tr_key("language-en"))
	$Layout/Language.select(1 if texts.language == "en" else 0)
	$Layout/Language.item_selected.connect(_language_changed)
	$Layout/KontraRe.button_pressed = kontra_re
	$Layout/KontraRe.toggled.connect(_kontra_re_changed)
	$Layout/OpenDisplay.button_pressed = open_display
	$Layout/OpenDisplay.toggled.connect(_open_display_changed)
	$Layout/CountingAid.button_pressed = counting_aid
	$Layout/CountingAid.toggled.connect(_assist_changed)
	$Layout/Quiz.button_pressed = quiz
	$Layout/Quiz.toggled.connect(_assist_changed)
	$Layout/ShowFigures.button_pressed = show_figures
	$Layout/ShowFigures.toggled.connect(_figures_changed)
	$Layout/QuizBox/Controls/Answer.pressed.connect(_answer_quiz)
	$Layout/QuizBox/Controls/Skip.pressed.connect(_act.bind("SkipQuiz"))
	$Layout/QuizBox/Controls/ShowHelp.pressed.connect(_act.bind("RestoreQuizHelp"))
	$Layout/QuizBox/Controls/Off.pressed.connect(_quiz_off)
	for key in COACH_LEVEL_KEYS:
		$Layout/CoachLevel.add_item(texts.tr_key(key))
	$Layout/CoachLevel.select(COACH_LEVELS.find(coach_level))
	$Layout/CoachLevel.item_selected.connect(_coach_changed)
	$Layout/CoachWarnings.button_pressed = coach_warnings
	$Layout/CoachWarnings.toggled.connect(_coach_changed)
	$Layout/CoachBox/Controls/Proceed.pressed.connect(_act.bind("ProceedAnyway"))
	$Layout/CoachBox/Controls/Reconsider.pressed.connect(_act.bind("Reconsider"))
	$Layout/DialogueEntry/Guided.pressed.connect(_act.bind("StartGuided"))
	$Layout/DialogueEntry/Abstich.pressed.connect(_act.bind("StartLiveAbstich"))
	for key in MODE_KEYS:
		$Layout/Mode.add_item(texts.tr_key(key))
	$Layout/Mode.select(MODES.find(mode))
	$Layout/Mode.item_selected.connect(_mode_changed)
	for key in ENGINE_KEYS:
		$Layout/Engine.add_item(texts.tr_key(key))
	$Layout/Engine.select(ENGINES.find(engine))
	$Layout/Engine.item_selected.connect(_engine_changed)
	$ConfirmRestart.confirmed.connect(_apply_setting)
	for i in range(2):
		var selector = $Layout/Opponents.get_child(i)
		for level in LEVELS:
			selector.add_item(level)
		selector.select(LEVELS.find(levels[i]))
		selector.item_selected.connect(_level_changed.bind(i))
	$Layout/PracticeControls/Practice.pressed.connect(_practice)
	$Layout/PracticeControls/Return.pressed.connect(_return_to_series)
	$Layout/PracticeControls/HistoryButton.pressed.connect(_history)

	$Layout/RecordingFiles/SaveRecording.pressed.connect(_recording_consent)
	$Layout/RecordingFiles/OpenRecording.pressed.connect(_recording_open_dialog)
	$RecordingConsent.confirmed.connect(func(): _recording_save_dialog.call_deferred())
	$RecordingFile.file_selected.connect(_recording_selected)
	$Replay.close_requested.connect(_replay_close)
	$Replay/Content/Navigation/Previous.pressed.connect(_replay_step.bind(-1))
	$Replay/Content/Navigation/Next.pressed.connect(_replay_step.bind(1))
	$Layout/LearningButtons/Training.pressed.connect(_training_open)
	$TrainingWindow.close_requested.connect(_training_close)
	$TrainingWindow/Content/Close.pressed.connect(_training_close)
	$TrainingWindow/Content/Topics/Trumpf.pressed.connect(_training_start.bind("TrumpfOutside"))
	$TrainingWindow/Content/Topics/Voids.pressed.connect(_training_start.bind("ProvenVoids"))
	$TrainingWindow/Content/Controls/Answer.pressed.connect(_training_answer)
	$TrainingWindow/Content/Controls/Help.pressed.connect(_training_command.bind("Help"))
	$TrainingWindow/Content/Controls/Skip.pressed.connect(_training_command.bind("Skip"))
	$TrainingWindow/Content/Controls/Retry.pressed.connect(_training_command.bind("Retry"))
	$TrainingWindow/Content/Controls/Next.pressed.connect(_training_command.bind("Next"))
	_start_series()

# Focused counting training outside the game: skat-app's TrainingSession,
# seen from one seat. The running game waits unchanged.
func _training_open() -> void:
	training_active = true
	app.stop_training()
	_training_show()
	$TrainingWindow.popup_centered.call_deferred()

func _training_close() -> void:
	training_active = false
	app.stop_training()
	$TrainingWindow.hide()

# A refused training command leaves training and game unchanged; it is only shown.
func _training_error(result: String) -> bool:
	var parsed = JSON.parse_string(result)
	if parsed is Dictionary and parsed.has("error"):
		$Layout/Status.text = JSON.stringify(parsed["error"])
		return true
	return false

func _training_start(topic: String) -> void:
	if not _training_error(app.start_training(JSON.stringify(topic), JSON.stringify("AnswerBeforeReveal"))):
		_training_show()

func _training_command(command: Variant) -> void:
	if not _training_error(app.training_command(JSON.stringify(command))):
		_training_show()

func _training_answer() -> void:
	_training_command({"Answer": int($TrainingWindow/Content/Controls/Value.value)})

func _training_show() -> void:
	var window := $TrainingWindow
	window.title = texts.tr_key("training-title")
	window.get_node("Content/Close").text = texts.tr_key("training-return")
	window.get_node("Content/Topics/Trumpf").text = texts.tr_key("training-start-trumpf")
	window.get_node("Content/Topics/Voids").text = texts.tr_key("training-start-voids")
	var controls := window.get_node("Content/Controls")
	controls.get_node("Answer").text = texts.tr_key("quiz-submit")
	controls.get_node("Help").text = texts.tr_key("retrieval-help")
	controls.get_node("Skip").text = texts.tr_key("quiz-skip")
	controls.get_node("Retry").text = texts.tr_key("training-retry")
	controls.get_node("Next").text = texts.tr_key("training-next")
	var raw: String = app.training_view()
	var view = JSON.parse_string(raw) if raw != "" else null
	controls.visible = view is Dictionary
	var text := window.get_node("Content/Scroll/Text")
	if not view is Dictionary:
		text.text = "\n".join([texts.tr_key("training-isolation"), texts.tr_key("training-intro"),
			texts.tr_key("training-note-trumpf"), texts.tr_key("training-note-voids"), texts.tr_key("training-practice-only")])
		return
	var settled = view["feedback"] != null
	controls.get_node("Answer").disabled = settled
	controls.get_node("Skip").disabled = settled
	controls.get_node("Value").editable = not settled
	controls.get_node("Help").disabled = settled or view["help"] != null
	controls.get_node("Retry").disabled = not settled
	controls.get_node("Next").disabled = not settled
	var seat: String = texts.tr_key("seat-" + str(view["seat"]).to_lower())
	var declarer: String = texts.tr_key("seat-" + str(view["declarer"]).to_lower())
	var contract: Dictionary = view["contract"]
	var game := " · ".join(_contract_parts(contract["game"], contract["hand"], contract))
	var hand: Array[String] = []
	for card in view["hand"]:
		hand.append(_card(int(card)))
	var tricks: Array[String] = []
	for trick in view["tricks"]:
		var played: Array[String] = []
		for entry in trick["cards"]:
			played.append(texts.tr_key("seat-" + str(entry[0]).to_lower()) + " " + _card(int(entry[1])))
		tricks.append(", ".join(played))
	var lines: Array[String] = [
		texts.tr_key("training-banner", {"topic": texts.tr_key(LEARNING_TOPICS[view["topic"]]), "position": int(view["position"][0]), "count": int(view["position"][1]), "presentation": int(view["presentation"])}),
		texts.tr_key("training-isolation"),
		texts.tr_key("training-contract", {"game": game, "bid": int(view["bid"]), "declarer": declarer}),
		texts.tr_key("training-role-declarer" if view["seat"] == view["declarer"] else "training-role-defender", {"seat": seat, "declarer": declarer}),
		texts.tr_key("training-hand") + ": " + ", ".join(hand)]
	if view["own_skat"] != null:
		var skat: Array[String] = []
		for card in view["own_skat"]:
			skat.append(_card(int(card)))
		lines.append(texts.tr_key("training-discard") + ": " + ", ".join(skat))
	else:
		lines.append(texts.tr_key("training-skat-unknown"))
	lines.append(texts.tr_key("training-tricks") + ": " + "; ".join(tricks))
	lines.append(texts.tr_key("training-table") + ": " + (texts.tr_key("training-you-lead") if view["current_trick"].is_empty() else ", ".join(view["current_trick"].map(func(e): return texts.tr_key("seat-" + str(e[0]).to_lower()) + " " + _card(int(e[1]))))))
	lines.append(texts.tr_key("training-question") + ": " + _question_text(view["question"]))
	if view["help"] != null:
		lines.append_array(_derivation_lines(view["help"]))
		lines.append(texts.tr_key("retrieval-assisted"))
	if settled:
		lines.append(texts.tr_key("training-result"))
		lines.append_array(_feedback_lines(view["feedback"]))
		if view["assisted"]:
			lines.append(texts.tr_key("training-was-assisted"))
	lines.append(texts.tr_key("training-fixture-note", {"version": int(view["task"]["version"])}))
	text.text = "\n".join(lines)

func _practice() -> void:
	var error := app.practice()
	if error != "":
		_report_error(error)
		return
	_show()

func _return_to_series() -> void:
	app.return_to_series()
	_show()

func _history() -> void:
	var summary: Dictionary = JSON.parse_string(app.learning_summary())
	var lines: Array[String] = [texts.tr_key("learning-descriptive")]
	var totals: Dictionary = summary["totals"]
	lines.append(texts.tr_key("learning-observed", {"count": int(totals["opportunities"]), "attempts": int(summary["attempts"])}))
	lines.append("%s | %s | %s | %s | %s | %s | %s" % [texts.tr_key("learning-col-topic"), texts.tr_key("learning-col-context"), texts.tr_key("learning-col-opportunities"), texts.tr_key("learning-col-answers"), texts.tr_key("learning-col-unaided"), texts.tr_key("learning-col-help"), texts.tr_key("learning-col-skips")])
	var topics := {"TrumpfOutside": "trumpf", "ProvenVoids": "voids", "AugenWon": "augen", "ClassPlayed": "played"}
	var contexts := {"Series": "series", "Practice": "practice", "PracticeRetry": "retry", "Training": "training", "TrainingRepeat": "training-repeat"}
	for row in summary["rows"]:
		var c: Dictionary = row["counts"]
		lines.append("%s | %s | %s | %s (%s) | %s (%s) | %s | %s" % [texts.tr_key("learning-topic-" + topics[row["topic"]]), texts.tr_key("learning-context-" + contexts[row["context"]]), int(c["opportunities"]), int(c["answers"]), int(c["correct"]), int(c["unaided_answers"]), int(c["unaided_correct"]), int(c["help"]), int(c["skips"])])
	if summary["full"]:
		lines.append(texts.tr_key("learning-full"))
	lines.append(texts.tr_key("learning-legend"))
	$History.title = texts.tr_key("learning-title")
	$History/Scroll/Summary.text = "\n".join(lines)
	$History.popup_centered(Vector2i(1000, 500))

func _recording_consent() -> void:
	$RecordingConsent.dialog_text = texts.tr_key("record-export-warning")
	$RecordingConsent.popup_centered()

func _recording_save_dialog() -> void:
	recording_save = true
	$RecordingFile.file_mode = FileDialog.FILE_MODE_SAVE_FILE
	$RecordingFile.popup_centered_ratio()

func _recording_open_dialog() -> void:
	recording_save = false
	$RecordingFile.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	$RecordingFile.popup_centered_ratio()

func _recording_selected(path: String) -> void:
	var error: String = app.save_recording(path) if recording_save else app.open_recording(path)
	if error != "":
		var parsed = JSON.parse_string(error)
		var detail = parsed.get("error", {}) if parsed is Dictionary else {}
		var kind = detail.get("Recording", "Io") if detail is Dictionary else "Io"
		if kind is Dictionary:
			kind = "InvalidRecording"
		var keys := {"InvalidJson": "json", "UnsupportedFormat": "format", "UnsupportedEngine": "engine", "InvalidRecording": "history", "TooLarge": "large", "Missing": "missing", "AlreadyExists": "exists", "Io": "io"}
		$RecordingError.dialog_text = texts.tr_key("record-error-" + keys.get(kind, "io"))
		$RecordingError.popup_centered.call_deferred()
	elif not recording_save:
		replay_active = true
		replay_index = 0
		_replay_show()
		$Replay.popup_centered.call_deferred()

func _replay_close() -> void:
	replay_active = false
	$Replay.hide()

func _replay_step(delta: int) -> void:
	replay_index = clampi(replay_index + delta, 0, replay_total)
	_replay_show()

func _replay_show() -> void:
	var replay: Dictionary = JSON.parse_string(app.replay(replay_index))
	replay_total = int(replay["total"])
	var view: Dictionary = replay["view"]
	var cards: Array[String] = []
	for card in replay["hand"]:
		cards.append(_card(int(card)))
	var bids: Array[String] = []
	for bid in view["bidding"]:
		var call = bid["call"]
		var label: String = texts.tr_key("button-pass")
		if call is Dictionary:
			var kind: String = call.keys()[0]
			label = texts.tr_key("button-" + kind.to_lower(), {"value": int(call[kind])})
		bids.append(texts.tr_key("seat-" + str(bid["seat"]).to_lower()) + ": " + label)
	var tricks: Array[String] = []
	for trick in view["tricks"]:
		tricks.append(_replay_cards(trick["cards"]))
	$Replay.title = texts.tr_key("record-title")
	$Replay/Content/Scroll/Position.text = "\n".join([texts.tr_key("record-boundary", {"count": replay_index}), texts.tr_key("phase-" + str(view["phase"]).to_lower()), ", ".join(cards), texts.tr_key("record-bidding") + ": " + ", ".join(bids), texts.tr_key("record-tricks") + ": " + "; ".join(tricks), texts.tr_key("record-current-trick") + ": " + _replay_cards(view["current_trick"]["cards"])])
	if view["contract"] is Dictionary:
		var contract: Dictionary = view["contract"]
		$Replay/Content/Scroll/Position.text += "\n" + texts.tr_key("label-announce") + " " + " · ".join(_contract_parts(contract["game"], contract["hand"], contract))
	if view["own_skat"] != null:
		var skat: Array[String] = []
		for i in range(32):
			if int(view["own_skat"]) & (1 << i):
				skat.append(_card(i))
		$Replay/Content/Scroll/Position.text += "\n" + texts.tr_key("label-own-skat") + " " + ", ".join(skat)
	$Replay/Content/Navigation/Previous.text = texts.tr_key("record-previous")
	$Replay/Content/Navigation/Next.text = texts.tr_key("record-next")
	$Replay/Content/Navigation/Previous.disabled = replay_index == 0
	$Replay/Content/Navigation/Next.disabled = replay_index == replay_total

func _replay_cards(cards: Array) -> String:
	var names: Array[String] = []
	for card in cards:
		if card != null:
			names.append(_card(int(card)))
	return ", ".join(names)

# Disclosure of what the selected mode lets the human or the opponents see.
# God Mode is stated apart from the open-display option (D-62).
func _mode_note() -> String:
	match mode:
		"Goldfish":
			return texts.tr_key("mode-goldfish-description")
		"Mephisto":
			return texts.tr_key("option-open-display-mephisto")
		"GodMode":
			return "%s %s" % [texts.tr_key("mode-god-description"), texts.tr_key("option-open-display-god")]
	return ""

func _start_series() -> void:
	var config := {"seed": 1, "series": {"Fixed": 3}, "kontra_re": kontra_re,
		"display": "Open" if open_display else "Closed",
		"opponents": {"Levels": levels} if mode == "Levels" else mode}
	if engine != "Classic":
		config["engine"] = engine
	failed = false
	var error := app.start(JSON.stringify(config))
	if error != "":
		_report_error(error)
		return
	app.use_learning_file(ProjectSettings.globalize_path(learning_path))
	_send_assist()
	_send_coach()
	_show()

# Assistance (D-64) is set on every new session; changing it never changes
# the game, so it needs no restart.
func _send_assist() -> void:
	var result := app.execute(JSON.stringify({"SetAssist": {
		"counting_aid": counting_aid, "quiz": quiz}}))
	_report_error(result)

func _figures_changed(enabled: bool) -> void:
	show_figures = enabled
	_save_setting("assist", "figures", show_figures)
	_send_coach()
	_show()

func _assist_changed(_enabled: bool) -> void:
	counting_aid = $Layout/CountingAid.button_pressed
	quiz = $Layout/Quiz.button_pressed
	_save_setting("assist", "counting_aid", counting_aid)
	_save_setting("assist", "quiz", quiz)
	_send_assist()
	_show()

# Coach mode and warnings (D-08, D-09) are set on every new session; changing
# them never changes the game, so it needs no restart. A held action is
# withdrawn by skat-app when warnings go off.
func _send_coach() -> void:
	var result := app.execute(JSON.stringify({"SetCoach": {
		"level": coach_level, "warnings": coach_warnings,
		"show_figures": show_figures, "focus": null}}))
	_report_error(result)

func _coach_changed(_value: Variant) -> void:
	coach_level = COACH_LEVELS[$Layout/CoachLevel.selected]
	coach_warnings = $Layout/CoachWarnings.button_pressed
	_save_setting("coach", "level", coach_level)
	_save_setting("coach", "warnings", coach_warnings)
	_send_coach()
	_show()

func _quiz_off() -> void:
	$Layout/Quiz.button_pressed = false

func _answer_quiz() -> void:
	_act({"AnswerQuiz": int($Layout/QuizBox/Controls/Value.value)})

func _save_setting(section: String, key: String, value: Variant) -> void:
	var settings := ConfigFile.new()
	var error := settings.load(settings_path)
	if error == OK or error == ERR_FILE_NOT_FOUND:
		settings.set_value(section, key, value)
		error = settings.save(settings_path)
	if error != OK:
		push_warning("Could not save settings: %s" % error_string(error))

func _language_changed(index: int) -> void:
	if texts.load_language(["de", "en"][index]):
		_save_setting("display", "language", texts.language)
		_show()

func _kontra_re_changed(enabled: bool) -> void:
	$Layout/KontraRe.set_pressed_no_signal(kontra_re)
	pending_setting = {"kontra_re": enabled}
	_confirm_restart()

func _open_display_changed(enabled: bool) -> void:
	$Layout/OpenDisplay.set_pressed_no_signal(open_display)
	pending_setting = {"open_display": enabled}
	_confirm_restart()

func _mode_changed(index: int) -> void:
	$Layout/Mode.select(MODES.find(mode))
	pending_setting = {"mode": MODES[index]}
	_confirm_restart()

func _engine_changed(index: int) -> void:
	$Layout/Engine.select(ENGINES.find(engine))
	pending_setting = {"engine": ENGINES[index]}
	_confirm_restart()

# Learned engines have no Rookie level (as in the egui app).
func _coerce_rookie() -> void:
	if engine == "Classic":
		return
	for i in range(2):
		if levels[i] == "Rookie":
			levels[i] = "Easy"
			$Layout/Opponents.get_child(i).select(LEVELS.find("Easy"))
			_save_setting("game", "level_%d" % i, "Easy")

func _level_changed(index: int, player: int) -> void:
	$Layout/Opponents.get_child(player).select(LEVELS.find(levels[player]))
	pending_setting = {"player": player, "level": LEVELS[index]}
	if engine != "Classic" and LEVELS[index] == "Rookie":
		pending_setting["level"] = "Easy"
	_confirm_restart()

func _confirm_restart() -> void:
	$ConfirmRestart.dialog_text = texts.tr_key("godot-setting-restart-confirm")
	$ConfirmRestart.popup_centered()

func _apply_setting() -> void:
	if pending_setting.has("mode"):
		mode = pending_setting["mode"]
		$Layout/Mode.select(MODES.find(mode))
		_save_setting("game", "mode", mode)
		if mode != "Levels" and engine != "Classic":
			engine = "Classic"
			$Layout/Engine.select(0)
			_save_setting("game", "engine", engine)
	elif pending_setting.has("engine"):
		engine = pending_setting["engine"]
		if engine != "Classic" and mode != "Levels":
			mode = "Levels"
			$Layout/Mode.select(0)
			_save_setting("game", "mode", mode)
		$Layout/Engine.select(ENGINES.find(engine))
		_save_setting("game", "engine", engine)
		for i in range(2):
			if levels[i] == "Rookie" and engine != "Classic":
				levels[i] = "Easy"
				$Layout/Opponents.get_child(i).select(LEVELS.find("Easy"))
				_save_setting("game", "level_%d" % i, "Easy")
	elif pending_setting.has("open_display"):
		open_display = pending_setting["open_display"]
		$Layout/OpenDisplay.set_pressed_no_signal(open_display)
		_save_setting("game", "open_display", open_display)
	elif pending_setting.has("kontra_re"):
		kontra_re = pending_setting["kontra_re"]
		$Layout/KontraRe.set_pressed_no_signal(kontra_re)
		_save_setting("game", "kontra_re", kontra_re)
	else:
		var player: int = pending_setting["player"]
		levels[player] = pending_setting["level"]
		$Layout/Opponents.get_child(player).select(LEVELS.find(levels[player]))
		_save_setting("game", "level_%d" % player, levels[player])
	pending_setting.clear()
	_start_series()

func _process(_delta: float) -> void:
	if failed or replay_active or training_active:
		return
	var coach_events := app.poll_coach()
	if coach_events != "":
		if not _report_error(coach_events):
			_show()
	elif coach_warnings:
		app.start_coach()
	var events := app.poll_opponent()
	if events != "":
		if not _report_error(events):
			_show()
	else:
		var view = JSON.parse_string(app.view())
		if view is Dictionary and view["opponent_to_move"]:
			app.start_opponent()

func _report_error(result: String) -> bool:
	var parsed = JSON.parse_string(result)
	if parsed is Dictionary and parsed.has("error"):
		$Layout/Status.text = JSON.stringify(parsed["error"])
		failed = true
		return true
	return false

func _integers(value: Variant) -> Variant:
	if value is float:
		return int(value)
	if value is Array:
		return value.map(_integers)
	if value is Dictionary:
		var converted := {}
		for key in value:
			converted[key] = _integers(value[key])
		return converted
	return value

func _act(command: Variant) -> void:
	var result := app.execute(JSON.stringify(_integers(command)))
	if not _report_error(result):
		_show()

func _card(index: int) -> String:
	return texts.tr_key("card-name", {
		"suit": texts.tr_key(["suit-kreuz", "suit-pik", "suit-herz", "suit-karo"][index / 8]),
		"rank": texts.tr_key(["rank-sieben", "rank-acht", "rank-neun", "rank-zehn",
			"rank-bube", "rank-dame", "rank-koenig", "rank-ass"][index % 8])})

func _face(index: int) -> Texture2D:
	if not faces.has(index):
		var name := "%s-%s" % [["kreuz", "pik", "herz", "karo"][index / 8],
			["sieben", "acht", "neun", "zehn", "bube", "dame", "koenig", "ass"][index % 8]]
		var path := ProjectSettings.globalize_path("res://").path_join("../assets/cards/german/%s.svg" % name)
		var image := Image.new()
		faces[index] = null
		if image.load_svg_from_string(FileAccess.get_file_as_string(path), 0.4) == OK:
			faces[index] = ImageTexture.create_from_image(image)
	return faces[index]

func _clear(box: Node) -> void:
	for child in box.get_children():
		box.remove_child(child)
		child.queue_free()

func _card_node(index: int, label: String) -> TextureButton:
	var node := TextureButton.new()
	node.texture_normal = _face(index)
	node.tooltip_text = label
	node.name = "Card%d" % index
	return node

func _trick(cards: Array) -> String:
	var played: Array[String] = []
	for card in cards:
		played.append("%s: %s" % [texts.tr_key(["player-human", "player-left", "player-right"][int(card["player"])]), _card(int(card["card"]))])
	return ", ".join(played)

func _label(action: Variant, bid: int, hand_game: Variant) -> String:
	if action is String:
		var keys := {"Hold": "button-hold", "Pass": "button-pass", "Hand": "button-hand",
			"TakeSkat": "button-take-skat", "Kontra": "button-kontra",
			"KeinKontra": "button-kein-kontra", "Re": "button-re", "KeinRe": "button-kein-re"}
		if keys.has(action):
			return texts.tr_key(keys[action], {"value": bid})
	if action is Dictionary:
		if action.has("Bid"):
			return texts.tr_key("button-bid", {"value": int(action["Bid"])})
		if action.has("Announce"):
			var announcement: Dictionary = action["Announce"]
			var parts := _contract_parts(announcement["game"], hand_game == true, announcement)
			return texts.tr_key("coach-action-announce", {"game": " · ".join(parts)})
		if action.has("Play"):
			return _card(int(action["Play"]))
		if action.has("Discard"):
			var cards: Array[String] = []
			for i in range(32):
				if int(action["Discard"]) & (1 << i):
					cards.append(_card(i))
			return texts.tr_key("coach-action-discard", {"cards": ", ".join(cards)})
	return JSON.stringify(action)

func _player(index: int) -> String:
	if cast_keys[index] != "":
		return texts.tr_key("figure-" + cast_keys[index])
	return texts.tr_key(["player-human", "player-left", "player-right"][index])

func _contract_parts(game: Variant, hand: bool, levels: Dictionary) -> Array[String]:
	var parts: Array[String] = []
	if game is Dictionary:
		parts.append(texts.tr_key("suit-" + str(game["Farbe"]).to_lower()))
	else:
		parts.append(texts.tr_key("game-" + str(game).to_lower()))
	if hand:
		parts.append(texts.tr_key("contract-hand"))
	for modifier in ["schneider", "schwarz", "ouvert"]:
		if levels[modifier]:
			parts.append(texts.tr_key("contract-" + modifier))
	return parts

# Score sheet as rows of cells: the values come from skat-app's view; only
# formatting happens here.
func _sheet_rows(view: Dictionary) -> Array:
	var rows := []
	var head := [texts.tr_key("sheet-number"), texts.tr_key("sheet-game")]
	for i in range(3):
		head.append(_player(i))
	rows.append(head)
	for record in view["sheet"]["records"]:
		var game: String = texts.tr_key("sheet-eingepasst")
		if record["result"] != null:
			var contract: Dictionary = record["contract"]
			var score := int(record["result"]["score"])
			game = "%s %s %s%d" % [_player(int(record["declarer"])),
				" ".join(_contract_parts(contract["game"], contract["hand"], contract)),
				"+" if record["result"]["won"] else "−", absi(score)]
		var row := [str(int(record["number"])), game]
		for total in record["totals"]:
			row.append(str(int(total)))
		rows.append(row)
	var foot := [texts.tr_key("sheet-total"), ""]
	for player in view["players"]:
		foot.append(str(int(player["total"])))
	rows.append(foot)
	return rows

func _series_text(view: Dictionary) -> String:
	var series = view["series_result"]
	if not series is Dictionary:
		return ""
	var lines: Array[String] = [texts.tr_key("series-finished", {"games": int(series["games"])})]
	for rank in range(series["ranking"].size()):
		var p := int(series["ranking"][rank])
		lines.append(texts.tr_key("series-rank", {"rank": rank + 1, "player": _player(p),
			"total": int(series["totals"][p]), "won": int(series["won"][p]), "lost": int(series["lost"][p])}))
	return "\n".join(lines)

# Figures of the series in the language's pool: skat-app picks them, the
# adapter names the catalogue, GDScript only loads name and portrait.
func _update_cast(view: Dictionary) -> void:
	if figure_keys.is_empty():
		figure_keys = JSON.parse_string(app.figure_keys())
	var pool: Array = view["cast"]["german" if texts.language == "de" else "english"]
	cast_keys = ["", str(figure_keys[int(pool[0])]), str(figure_keys[int(pool[1])])]

func _portrait(key: String) -> Texture2D:
	if not portraits.has(key):
		var path := ProjectSettings.globalize_path("res://").path_join("../assets/portraits/%s.svg" % key)
		var image := Image.new()
		portraits[key] = null
		if image.load_svg_from_string(FileAccess.get_file_as_string(path), 0.5) == OK:
			portraits[key] = ImageTexture.create_from_image(image)
	return portraits[key]

func _show_cast() -> void:
	var box: HBoxContainer = $Layout/Cast
	_clear(box)
	for seat in [1, 2]:
		var picture := TextureRect.new()
		picture.texture = _portrait(cast_keys[seat])
		picture.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		picture.custom_minimum_size = Vector2(56, 56)
		box.add_child(picture)
		var name_label := Label.new()
		name_label.text = _player(seat)
		box.add_child(name_label)

func _show_sheet(view: Dictionary) -> void:
	_update_cast(view)
	var grid: GridContainer = $Layout/Sheet
	_clear(grid)
	$Layout/SheetTitle.text = texts.tr_key("sheet-title")
	if view["practice"]:
		return
	for row in _sheet_rows(view):
		for cell in row:
			var label := Label.new()
			label.text = cell
			grid.add_child(label)
	_show_cast()
	$Layout/Series.text = _series_text(view)
	$Layout/Series.visible = view["series_result"] != null

func _show() -> void:
	var view = JSON.parse_string(app.view())
	if not view is Dictionary:
		return
	$Layout/PracticeControls/Practice.text = texts.tr_key("practice-restart" if view["practice"] else "record-practice-start")
	$Layout/PracticeControls/Practice.tooltip_text = texts.tr_key("record-practice-help")
	$Layout/PracticeNotice.visible = view["practice"]
	$Layout/PracticeNotice.text = texts.tr_key("practice-isolation")
	$Layout/PracticeControls/Return.visible = view["practice"]
	$Layout/PracticeControls/Return.text = texts.tr_key("practice-return")
	$Layout/PracticeControls/HistoryButton.text = texts.tr_key("learning-open")
	$Layout/RecordingFiles/SaveRecording.text = texts.tr_key("record-save")
	$Layout/RecordingFiles/OpenRecording.text = texts.tr_key("record-open")
	$Layout/LearningButtons/Training.text = texts.tr_key("training-open")
	if training_active:
		_training_show()
	$Layout/RecordingFiles/SaveRecording.disabled = not (view["can_next_game"] or view["series_finished"])
	$Layout/Language.tooltip_text = texts.tr_key("setting-language")
	$Layout/KontraRe.text = texts.tr_key("option-kontra-re")
	$Layout/OpenDisplay.text = texts.tr_key("option-open-display")
	$Layout/OpenDisplay.tooltip_text = texts.tr_key("option-open-display-description")
	for i in range(2):
		var selector = $Layout/Opponents.get_child(i)
		selector.tooltip_text = texts.tr_key(["opponent-left", "opponent-right"][i])
		for j in range(LEVELS.size()):
			selector.set_item_text(j, "%s %s" % [selector.tooltip_text, texts.tr_key("level-" + LEVELS[j].to_lower())])
	$Layout/Engine.tooltip_text = texts.tr_key("opponent-engine")
	$Layout/CoachLevel.tooltip_text = texts.tr_key("setting-coach-level")
	for j in range(COACH_LEVELS.size()):
		$Layout/CoachLevel.set_item_text(j, texts.tr_key(COACH_LEVEL_KEYS[j]))
	$Layout/CoachWarnings.text = texts.tr_key("setting-coach-warnings")
	$Layout/CoachWarnings.tooltip_text = texts.tr_key("setting-coach-warnings-description")
	$Layout/Mode.tooltip_text = texts.tr_key("mode-levels")
	for j in range(MODES.size()):
		$Layout/Mode.set_item_text(j, texts.tr_key(MODE_KEYS[j]))
	$Layout/ModeNote.text = _mode_note()
	$Layout/ModeNote.visible = $Layout/ModeNote.text != ""
	for i in range(2):
		$Layout/Opponents.get_child(i).disabled = mode != "Levels"
	$Layout/CountingAid.text = texts.tr_key("setting-aid")
	$Layout/CountingAid.tooltip_text = texts.tr_key("setting-aid-description")
	$Layout/Quiz.text = texts.tr_key("setting-quiz")
	$Layout/ShowFigures.text = texts.tr_key("setting-coach-figures")
	$Layout/Quiz.tooltip_text = texts.tr_key("setting-quiz-description")
	for j in range(ENGINES.size()):
		$Layout/Engine.set_item_text(j, texts.tr_key(ENGINE_KEYS[j]))
	var hand: Array[String] = []
	for card in view["hand"]:
		hand.append(_card(int(card)))
	$Layout/Table.text = "%d · %s\n%s\n%s: %s" % [
		view["game_number"], texts.tr_key("phase-" + str(view["phase"]).to_lower()),
		", ".join(hand), texts.tr_key("trick-current"), _trick(view["current_trick"])]
	if view["own_skat"] is Array:
		var skat_cards: Array[String] = []
		for card in view["own_skat"]:
			skat_cards.append(_card(int(card)))
		$Layout/Table.text += "\n%s %s" % [texts.tr_key("label-own-skat"), ", ".join(skat_cards)]
	_show_sheet(view)
	var holding := _show_assist(view["assist"])
	holding = _show_coach(view) or holding
	# An open Abstich question refuses game actions until the learner returns.
	holding = _show_dialogue(view, holding) or holding
	$Layout/OpenCards.text = _open_cards(view["open_cards"])
	$Layout/OpenCards.visible = view["open_cards"] != null
	_clear($Layout/Hand)
	_clear($Layout/Trick)
	for card in view["hand"]:
		var node := _card_node(int(card), _card(int(card)))
		node.disabled = true
		$Layout/Hand.add_child(node)
	for played in view["current_trick"]:
		var node := _card_node(int(played["card"]), "%s: %s" % [texts.tr_key(["player-human", "player-left", "player-right"][int(played["player"])]), _card(int(played["card"]))])
		node.disabled = true
		$Layout/Trick.add_child(node)
	$Layout/Status.text = texts.tr_key("status-waiting", {"player": texts.tr_key(["player-human", "player-left", "player-right"][int(view["acting"])])}) if view["opponent_to_move"] and view["acting"] != null else ""
	_clear($Layout/Scroll/Actions)
	if not view["opponent_to_move"] and not holding:
		var discards := {}
		for action in view["legal_actions"]:
			if action is Dictionary and action.has("Discard"):
				discards[int(action["Discard"])] = action
		_show_discard(discards)
		var announcements: Array = []
		for action in view["legal_actions"]:
			if action is Dictionary and action.has("Announce"):
				announcements.append(action)
		_show_announcements(announcements, view["hand_game"])
		for action in view["legal_actions"]:
			if action is Dictionary and (action.has("Discard") or action.has("Announce")):
				continue
			if action is Dictionary and action.has("Play"):
				var card := $Layout/Hand.get_node_or_null("Card%d" % int(action["Play"]))
				if card != null:
					card.disabled = false
					card.pressed.connect(_act.bind({"Act": action}))
			_add_button(_label(action, int(view["bid"]), view["hand_game"]), {"Act": action})
		if view["can_next_game"]:
			_add_button(texts.tr_key("button-next-game"), "NextGame")
		elif view["legal_actions"].is_empty():
			$Layout/Status.text = texts.tr_key("series-finished", {"games": int(view["game_number"])}) if view["series_finished"] else ""

# Ansage: one picker over exactly the legal Announce actions of the view.
func _show_announcements(announcements: Array, hand_game: Variant) -> void:
	if announcements.is_empty():
		return
	var title := Label.new()
	title.name = "AnnounceLabel"
	title.text = texts.tr_key("label-announce")
	$Layout/Scroll/Actions.add_child(title)
	var picker := OptionButton.new()
	picker.name = "AnnouncePicker"
	for action in announcements:
		picker.add_item(_label(action, 0, hand_game))
	$Layout/Scroll/Actions.add_child(picker)
	var confirm := Button.new()
	confirm.name = "ConfirmAnnounce"
	confirm.text = texts.tr_key("button-announce")
	confirm.pressed.connect(func(): _act({"Act": announcements[picker.selected]}))
	$Layout/Scroll/Actions.add_child(confirm)

# Drücken: the player marks two hand cards and confirms; the resulting mask
# must be one of the legal Discard actions of the view.
func _show_discard(discards: Dictionary) -> void:
	if discards.is_empty():
		discard_selection.clear()
		return
	for card in discard_selection.duplicate():
		if $Layout/Hand.get_node_or_null("Card%d" % card) == null:
			discard_selection.erase(card)
	for node in $Layout/Hand.get_children():
		if node.is_queued_for_deletion():
			continue
		node.disabled = false
		node.toggle_mode = true
		node.set_pressed_no_signal(discard_selection.has(node.name.trim_prefix("Card").to_int()))
		node.modulate = Color(1, 1, 1, 0.55) if node.button_pressed else Color.WHITE
		node.toggled.connect(_discard_toggled.bind(node.name.trim_prefix("Card").to_int(), discards))
	var mask := _discard_mask()
	var button := Button.new()
	button.name = "ConfirmDiscard"
	button.text = texts.tr_key("coach-action-discard", {"cards": ", ".join(discard_selection.map(_card))})
	button.disabled = not discards.has(mask)
	if not button.disabled:
		button.pressed.connect(_act.bind({"Act": discards[mask]}))
	$Layout/Scroll/Actions.add_child(button)

func _discard_mask() -> int:
	var mask := 0
	for card in discard_selection:
		mask |= 1 << card
	return mask

func _discard_toggled(on: bool, card: int, discards: Dictionary) -> void:
	discard_selection.erase(card)
	if on:
		discard_selection.append(card)
	_discard_refresh(discards)

func _discard_refresh(discards: Dictionary) -> void:
	for node in $Layout/Hand.get_children():
		var pressed: bool = discard_selection.has(node.name.trim_prefix("Card").to_int())
		node.modulate = Color(1, 1, 1, 0.55) if pressed else Color.WHITE
	for child in $Layout/Scroll/Actions.get_children():
		if child.name == "ConfirmDiscard" and not child.is_queued_for_deletion():
			var mask := _discard_mask()
			child.text = texts.tr_key("coach-action-discard", {"cards": ", ".join(discard_selection.map(_card))})
			child.disabled = not discards.has(mask)
			for c in child.pressed.get_connections():
				child.pressed.disconnect(c["callable"])
			if not child.disabled:
				child.pressed.connect(_act.bind({"Act": discards[mask]}))

func _open_cards(open: Variant) -> String:
	if not open is Dictionary:
		return ""
	# God Mode is labelled apart from the open display.
	var lines: Array[String] = [texts.tr_key("mode-god" if mode == "GodMode" else "label-open-cards")]
	for i in range(3):
		var names: Array[String] = []
		for card in open["hands"][i]:
			names.append(_card(int(card)))
		lines.append("%s: %s" % [texts.tr_key(["player-human", "player-left", "player-right"][i]), ", ".join(names)])
	var skat: Array[String] = []
	for card in open["skat"]:
		skat.append(_card(int(card)))
	lines.append("Skat: %s" % ", ".join(skat))
	return "\n".join(lines)

func _add_button(label: String, command: Variant) -> void:
	var button := Button.new()
	button.text = label
	button.pressed.connect(_act.bind(command))
	$Layout/Scroll/Actions.add_child(button)

# Counting aid and quiz: every figure comes from skat-app's permitted view;
# only wording happens here. Returns true while a question holds the move.
func _show_assist(assist: Dictionary) -> bool:
	var counting = assist["counting"]
	var aid_lines: Array[String] = []
	if counting is Dictionary:
		aid_lines = _aid_lines(counting["Shown"])
		aid_lines.push_front(texts.tr_key("aid-exact"))
		aid_lines.push_front(texts.tr_key("aid-title"))
	elif counting == "Withheld":
		aid_lines = [texts.tr_key("aid-title"), texts.tr_key("aid-withheld")]
	$Layout/Aid.text = "\n".join(aid_lines)
	$Layout/Aid.visible = not aid_lines.is_empty()
	var quiz_view: Dictionary = assist["quiz"]
	var question = quiz_view["question"]
	var box := $Layout/QuizBox
	box.visible = question != null
	if question != null:
		var id := JSON.stringify(quiz_view["id"])
		if id != quiz_id:
			quiz_id = id
			box.get_node("Controls/Value").value = 0
		var lines: Array[String] = [texts.tr_key("quiz-title"), _question_text(question),
			texts.tr_key("quiz-hint")]
		if quiz_view["help"] != null:
			lines.append_array(_derivation_lines(quiz_view["help"]))
			lines.append(texts.tr_key("retrieval-assisted"))
		box.get_node("Question").text = "\n".join(lines)
		box.get_node("Controls/Answer").text = texts.tr_key("quiz-submit")
		box.get_node("Controls/Skip").text = texts.tr_key("quiz-skip")
		box.get_node("Controls/ShowHelp").text = texts.tr_key("retrieval-help")
		box.get_node("Controls/ShowHelp").disabled = quiz_view["help"] != null
		box.get_node("Controls/Off").text = texts.tr_key("quiz-off")
	var feedback = quiz_view["feedback"]
	$Layout/QuizFeedback.visible = feedback != null
	if feedback != null:
		var lines: Array[String] = [texts.tr_key("quiz-feedback-title")]
		lines.append_array(_feedback_lines(feedback))
		$Layout/QuizFeedback.text = "\n".join(lines)
	return question != null

# Warning before a weak move: the coach part of the permitted view holds the
# action; the player proceeds or reconsiders. Returns true while it is held.
func _show_coach(view: Dictionary) -> bool:
	var pending = view["coach"]["pending"]
	var box := $Layout/CoachBox
	box.visible = pending != null
	if pending == null:
		return false
	var state: String = pending.keys()[0]
	var action = pending[state]["action"]
	var label := _label(action, int(view["bid"]), view["hand_game"])
	var proceed := box.get_node("Controls/Proceed")
	if state == "Warning":
		box.get_node("Message").text = "%s\n%s" % [texts.tr_key("coach-warning-title"),
			texts.tr_key("coach-warning-text", {"action": label})]
		proceed.text = texts.tr_key("button-coach-proceed")
		proceed.visible = true
	else:
		box.get_node("Message").text = texts.tr_key("coach-checking", {"action": label})
		proceed.visible = false
	box.get_node("Controls/Reconsider").text = texts.tr_key("button-coach-reconsider")
	return true

func _class_name(follow: Variant) -> String:
	if follow is Dictionary:
		return texts.tr_key("suit-" + str(follow["Farbe"]).to_lower())
	return texts.tr_key("quiz-class-trumpf")

func _suit_name(index: int) -> String:
	return texts.tr_key(["suit-kreuz", "suit-pik", "suit-herz", "suit-karo"][index])

func _aid_lines(aid: Dictionary) -> Array[String]:
	var lines: Array[String] = [texts.tr_key("aid-played", {"count": aid["played"].size()})]
	if not aid["played"].is_empty():
		var cards: Array[String] = []
		for card in aid["played"]:
			cards.append(_card(int(card)))
		lines.append(texts.tr_key("aid-played-cards", {"cards": ", ".join(cards)}))
	var trumpf = aid["trumpf"]
	if trumpf is Dictionary:
		lines.append(texts.tr_key("aid-trumpf", {"outside": int(trumpf["outside"]),
			"total": int(trumpf["total"]), "own": int(trumpf["own"]), "played": int(trumpf["played"])}))
		if int(trumpf["discarded"]) > 0:
			lines.append(texts.tr_key("aid-trumpf-discard", {"discarded": int(trumpf["discarded"])}))
		if trumpf["skat_unknown"]:
			lines.append(texts.tr_key("aid-trumpf-skat"))
	elif aid["game"] != null:
		lines.append(texts.tr_key("aid-trumpf-null"))
	else:
		lines.append(texts.tr_key("aid-trumpf-later"))
	var augen: Dictionary = aid["augen"]
	if augen["declarer"] != null and augen["defenders"] != null:
		lines.append(texts.tr_key("aid-augen", {"declarer": int(augen["declarer"]),
			"defenders": int(augen["defenders"])}))
	else:
		lines.append(texts.tr_key("aid-augen-later"))
	lines.append(texts.tr_key("aid-augen-outstanding", {"count": int(augen["outstanding"])}))
	if augen["skat"] != null:
		lines.append(texts.tr_key("aid-augen-skat", {"count": int(augen["skat"])}))
	lines.append(texts.tr_key("aid-voids"))
	var any_void := false
	for player in [1, 2]:
		var voids: Dictionary = aid["voids"][player]
		var classes: Array[String] = []
		if voids["trumpf"]:
			classes.append(texts.tr_key("aid-void-trumpf"))
		for suit in range(4):
			if voids["suits"][suit]:
				classes.append(_suit_name(suit))
		if not classes.is_empty():
			any_void = true
			lines.append(texts.tr_key("aid-void-line", {"player": _player(player),
				"classes": ", ".join(classes)}))
	if not any_void:
		lines.append(texts.tr_key("aid-void-none"))
	return lines

func _question_text(question: Variant) -> String:
	if question is String:
		return texts.tr_key("quiz-q-trumpf")
	if question.has("AugenWon"):
		return texts.tr_key("quiz-q-augen-declarer" if question["AugenWon"]["declarer"] else "quiz-q-augen-defenders")
	if question.has("ClassPlayed"):
		return texts.tr_key("quiz-q-played", {"class": _class_name(question["ClassPlayed"]["class"])})
	return texts.tr_key("quiz-q-voids", {"class": _class_name(question["ProvenVoids"]["class"])})

# The exact answer as skat-app derived it; nothing is computed from hidden cards.
func _expected(derivation: Dictionary) -> int:
	if derivation.has("TrumpfOutside"):
		return int(derivation["TrumpfOutside"]["outside"])
	if derivation.has("AugenWon"):
		return int(derivation["AugenWon"]["augen"])
	if derivation.has("ClassPlayed"):
		return int(derivation["ClassPlayed"]["played"])
	return derivation["ProvenVoids"]["seats"].count(true)

func _derivation_lines(derivation: Dictionary) -> Array[String]:
	var lines: Array[String] = []
	if derivation.has("TrumpfOutside"):
		var t: Dictionary = derivation["TrumpfOutside"]
		lines.append(texts.tr_key("quiz-why-trumpf", {"total": int(t["total"]), "own": int(t["own"]),
			"played": int(t["played"]), "discarded": int(t["discarded"]), "outside": int(t["outside"])}))
		if t["skat_unknown"]:
			lines.append(texts.tr_key("quiz-why-trumpf-skat"))
	elif derivation.has("AugenWon"):
		var a: Dictionary = derivation["AugenWon"]
		lines.append(texts.tr_key("quiz-why-augen", {"augen": int(a["augen"]), "other": int(a["other"])}))
	elif derivation.has("ClassPlayed"):
		var c: Dictionary = derivation["ClassPlayed"]
		lines.append(texts.tr_key("quiz-why-played", {"class": _class_name(c["class"]),
			"size": int(c["size"]), "played": int(c["played"]), "unplayed": int(c["unplayed"])}))
	else:
		var v: Dictionary = derivation["ProvenVoids"]
		lines.append(texts.tr_key("quiz-why-voids", {"class": _class_name(v["class"]),
			"count": _expected(derivation)}))
	return lines

func _feedback_lines(feedback: Dictionary) -> Array[String]:
	var expected := _expected(feedback["derivation"])
	var outcome = feedback["outcome"]
	var lines: Array[String] = []
	if outcome is Dictionary:
		lines.append(texts.tr_key("quiz-incorrect", {"given": int(outcome["Incorrect"]["given"]), "expected": expected}))
	elif outcome == "Correct":
		lines.append(texts.tr_key("quiz-correct", {"expected": expected}))
	else:
		lines.append(texts.tr_key("quiz-skipped", {"expected": expected}))
	lines.append_array(_derivation_lines(feedback["derivation"]))
	return lines

# Guided dialogue and live Abstich question (D-64, #131, #13): the session owns
# every step, answer and piece of evidence; this only words the permitted views
# and sends the learner's explicit responses. Neither ever plays a card.
func _show_dialogue(view: Dictionary, holding: bool) -> bool:
	var guided = JSON.parse_string(app.guided())
	var abstich = JSON.parse_string(app.live_abstich())
	var plays: Array = view["legal_actions"].filter(func(a): return a is Dictionary and a.has("Play"))
	var entry: bool = not view["opponent_to_move"] and not holding \
		and not view["legal_actions"].is_empty() and guided == null and abstich == null
	$Layout/DialogueEntry.visible = entry
	$Layout/DialogueEntry/Guided.text = texts.tr_key("guided-start")
	$Layout/DialogueEntry/Abstich.text = texts.tr_key("live-abstich-start")
	$Layout/DialogueEntry/Abstich.disabled = plays.size() < 2
	var box := $Layout/Dialogue
	_clear(box.get_node("Buttons"))
	box.visible = guided != null or abstich != null
	if guided is Dictionary:
		_show_guided(guided, view)
	elif abstich is Dictionary:
		_show_abstich(abstich, view)
	return abstich is Dictionary

func _dialogue_button(label: String, command: Variant, pressed := false) -> void:
	var button := Button.new()
	button.text = label
	button.toggle_mode = pressed
	button.button_pressed = pressed
	button.pressed.connect(_act.bind(command))
	$Layout/Dialogue/Buttons.add_child(button)

func _seat_name(seat: Variant) -> String:
	return texts.tr_key("seat-" + str(seat).to_lower())

func _guided_command(guided: Dictionary, action: Variant) -> Variant:
	return {"Guided": [guided["id"], action]}

func _guided_prompt(guided: Dictionary, view: Dictionary) -> String:
	var task = guided.get("task")
	var step: String = guided["step"]
	var ctx := _null_context(view)
	var lead = guided.get("lead")
	var is_null: bool = ctx["null"]
	if step == "Orient":
		return texts.tr_key("guided-lead-orient" if lead != null else "guided-orient")
	if step == "Check":
		if lead != null and is_null: return texts.tr_key("guided-null-lead-candidates")
		if guided.get("lead_trumpf", false): return texts.tr_key("guided-draw-trumpf")
		if lead != null: return texts.tr_key("guided-lead-master")
		return texts.tr_key("guided-check")
	if step == "Observe":
		if guided.get("lead_trumpf", false): return texts.tr_key("guided-draw-trumpf-observe")
		if lead != null: return texts.tr_key("guided-lead-observe")
	if step == "Narrow":
		if lead != null and is_null: return texts.tr_key("guided-null-lead-attempt")
		if guided.get("lead_trumpf", false): return texts.tr_key("guided-draw-trumpf-attempt")
		if guided.get("lead_fallback", false): return texts.tr_key("guided-lead-fallback-attempt")
		if lead != null: return texts.tr_key("guided-lead-attempt")
	if step == "Narrow" or step == "Threshold":
		if task == "UnderPartner":
			return texts.tr_key("guided-under")
		if task == "Fallback":
			for flag in [["overtake_forced", "guided-forced-early-attempt"],
					["decline_last", "guided-decline-last-attempt"],
					["decline_early", "guided-decline-early-attempt"],
					["decline", "guided-decline-attempt"]]:
				if guided.get(flag[0], false):
					return texts.tr_key(flag[1])
			return texts.tr_key("guided-fallback-attempt")
		if task == "FeedPartner":
			return texts.tr_key("guided-feed-attempt")
		if task == "WinEarly":
			return texts.tr_key("guided-early-attempt")
		if task == "Null" and ctx["partner_lead"]:
			return texts.tr_key("guided-null-partner-attempt")
		if task == "Null" and guided.get("null_forced", false):
			if ctx["defender"]:
				return texts.tr_key("guided-null-defender-forced-attempt")
			return texts.tr_key("guided-null-middle-forced-attempt" if ctx["trick"] == 1
				else "guided-null-forced-attempt")
		if task == "Null":
			return texts.tr_key("guided-null-attempt")
		return texts.tr_key("guided-win")
	return texts.tr_key({"Act": "guided-act", "Waiting": "guided-waiting",
		"Observe": "guided-observe", "Complete": "guided-complete"}.get(step, "guided-unsupported"))

# Public Null/seat facts of the table the dialogue words (no state of its own).
func _null_context(view: Dictionary) -> Dictionary:
	var contract = view["contract"]
	var is_null: bool = contract is Dictionary and str(contract["game"]) == "Null"
	var trick: Array = view["current_trick"]
	var declarer = view["declarer"]
	var defender: bool = null and declarer != null and int(declarer) != 0
	return {"null": is_null, "defender": defender, "trick": trick.size(),
		"declarer_human": declarer != null and int(declarer) == 0,
		"partner_lead": defender and trick.size() == 1 and int(trick[0]["player"]) != int(declarer),
		"ouvert": contract is Dictionary and contract["ouvert"] == true}

func _card_list(cards: Array) -> String:
	var names: Array[String] = []
	for card in cards:
		names.append(_card(int(card)))
	return ", ".join(names)

# Rule answer after the deliberate reveal; mirrors the egui wording per branch,
# the collapsed details included as plain lines.
func _guided_rule(guided: Dictionary, view: Dictionary) -> String:
	var rule = guided["rule"]
	var ctx := _null_context(view)
	var branch: String = rule["branch"]
	var lead = guided.get("lead")
	var last: bool = rule["last"]
	var losers_empty: bool = rule["losers"].is_empty()
	var lead_rule: bool = branch in ["LeadMaster", "DrawTrumpf"]
	var key := "guided-rule-win"
	if branch == "DrawTrumpf": key = "guided-rule-draw-trumpf"
	elif branch == "LeadMaster": key = "guided-rule-lead"
	elif branch == "Fallback" and lead != null: key = "guided-rule-lead-fallback"
	elif branch == "UnderPartner": key = "guided-rule-under"
	elif branch == "Fallback":
		if guided.get("overtake_forced", false): key = "guided-rule-forced-early"
		elif guided.get("decline_last", false): key = "guided-rule-decline-last"
		elif guided.get("decline_early", false): key = "guided-rule-decline-early"
		elif not rule["partner_ahead"]: key = "guided-rule-decline"
		else: key = "guided-rule-fallback"
	elif branch == "FeedPartner": key = "guided-rule-feed" if last else "guided-rule-feed-early"
	elif branch == "WinEarly": key = "guided-rule-early"
	elif branch == "Null":
		var defender: bool = ctx["defender"]
		var partner: bool = ctx["partner_lead"]
		if lead != null: key = "guided-rule-null-lead"
		elif defender and not last and losers_empty and not partner: key = "guided-rule-null-defender-forced-middle"
		elif defender and last and losers_empty and not partner: key = "guided-rule-null-defender-forced-last"
		elif defender and last: key = "guided-rule-null-defender-last"
		elif partner: key = "guided-rule-null-partner"
		elif defender: key = "guided-rule-null-defender"
		elif losers_empty: key = "guided-rule-null-forced" if last else "guided-rule-null-middle-forced"
		else: key = "guided-rule-null" if last else "guided-rule-null-middle"
	var lines: Array[String] = []
	var cards := _card_list(rule["candidates"])
	if lead_rule:
		lines.append(texts.tr_key(key, {"card": _card(int(rule["card"]))}))
		lines.append(texts.tr_key("guided-draw-trumpf-candidates-risk" if branch == "DrawTrumpf"
			else "guided-lead-candidates-risk", {"cards": cards}))
	else:
		lines.append(texts.tr_key("guided-rule-card", {"card": _card(int(rule["card"]))}))
		lines.append(texts.tr_key(key))
	if branch == "UnderPartner" and ctx["ouvert"]:
		lines.append(texts.tr_key("guided-under-ouvert"))
	if branch == "Fallback" and lead != null and ctx["ouvert"]:
		lines.append(texts.tr_key("guided-lead-fallback-ouvert"))
	var condition = guided.get("early_condition")
	if condition != null:
		var contract = view["contract"]
		var suit: bool = contract is Dictionary and contract["game"] is Dictionary
		lines.append(texts.tr_key(("guided-suit-early-" if suit else "guided-early-")
			+ ("trumpf" if str(condition) == "Trumpf" else "master")))
	if branch == "DrawTrumpf":
		lines.append_array([texts.tr_key("guided-draw-trumpf-conservative"), texts.tr_key("guided-lead-empty")])
	elif branch == "LeadMaster":
		lines.append_array([texts.tr_key("guided-lead-conservative"), texts.tr_key("guided-lead-empty")])
	elif branch == "Null" and lead != null:
		lines.append_array([texts.tr_key("guided-null-lead-order"), texts.tr_key("guided-null-lead-empty")])
	elif branch == "FeedPartner" and not last:
		lines.append(texts.tr_key("guided-feed-early-order"))
	elif branch == "Null" and not last and not ctx["partner_lead"]:
		lines.append(texts.tr_key("guided-null-middle-forced-order" if losers_empty else "guided-null-middle-order"))
	if not lead_rule:
		lines.append(texts.tr_key("guided-candidates", {"cards": cards}))
	if branch == "Null" and lead == null and not ctx["defender"]:
		var winners := _card_list(rule["winners"])
		if rule["winners"].is_empty():
			lines.append(texts.tr_key("guided-null-no-winners" if last else "guided-null-middle-no-winners"))
		else:
			lines.append(texts.tr_key("guided-null-winners" if last else "guided-null-middle-winners", {"cards": winners}))
	return "\n".join(lines)

func _source_line(source: Variant) -> String:
	if source is Dictionary and source.has("Trick"):
		var trick: Dictionary = source["Trick"]
		return texts.tr_key("guided-source", {"trick": int(trick["index"]) + 1,
			"seat": _seat_name(trick["seat"]), "card": _card(int(trick["card"]))})
	return ""

func _show_guided(guided: Dictionary, view: Dictionary) -> void:
	var lines: Array[String] = [texts.tr_key("guided-title")]
	if guided.get("reizen") is Dictionary:
		_show_reizen(guided, lines)
	elif guided.get("preplay") is Dictionary:
		_show_preplay(guided, view, lines)
	elif guided.get("kontra") is Dictionary:
		_show_kontra(guided, view, lines)
	else:
		_show_card_play(guided, view, lines)
	$Layout/Dialogue/Text.text = "\n".join(lines)

func _retry_lines(guided: Dictionary, lines: Array[String]) -> void:
	if guided["correct"] == false:
		lines.append(texts.tr_key("guided-wrong"))
		_dialogue_button(texts.tr_key("guided-retry"), _guided_command(guided, "Retry"))

func _reveal_buttons(guided: Dictionary, steps: Array) -> void:
	if guided["step"] in steps:
		_dialogue_button(texts.tr_key("guided-unknown"), _guided_command(guided, "Reveal"))
		_dialogue_button(texts.tr_key("guided-reveal"), _guided_command(guided, "Reveal"))

func _game_name(game: Variant) -> String:
	return texts.tr_key("suit-" + str(game["Farbe"]).to_lower()) if game is Dictionary \
		else texts.tr_key("game-" + str(game).to_lower())

func _skat_line(skat: Variant) -> String:
	if int(skat) == 0:
		return texts.tr_key("guided-ann-skat-hand")
	var cards: Array[String] = []
	for i in range(32):
		if int(skat) & (1 << i):
			cards.append(_card(i))
	return texts.tr_key("guided-ann-skat-known", {"cards": ", ".join(cards)})

# Card play: follow-trick, lead and Null branches, observation and memory.
func _show_card_play(guided: Dictionary, view: Dictionary, lines: Array[String]) -> void:
	var step: String = guided["step"]
	var ctx := _null_context(view)
	var is_null: bool = ctx["null"]
	var lead = guided.get("lead")
	var contract = view["contract"]
	var position: int = ctx["trick"]
	var declarer = view["declarer"]
	var active: bool = step in ["Orient", "Check", "Narrow", "Act"]
	if step == "Unsupported" and guided.get("kontra_exclusion") != null:
		lines.append(texts.tr_key({"OuvertRound": "guided-unsupported-kontra-ouvert-round",
			"Ouvert": "guided-unsupported-kontra-ouvert",
			"Restricted": "guided-unsupported-kontra"}[str(guided["kontra_exclusion"])]))
		_dialogue_button(texts.tr_key("guided-return"), _guided_command(guided, "Return"))
		return
	if guided.get("feed_partner") != null and active and declarer != null and int(declarer) != 0 \
			and position == 2 and contract is Dictionary and not is_null:
		lines.append(texts.tr_key("guided-feed-role", {"seat": _seat_name(guided["feed_partner"])}))
	if contract is Dictionary and contract["ouvert"] == true and not is_null and step == "Act" \
			and guided["rule"] != null and guided["rule"]["branch"] == "FeedPartner":
		lines.append(texts.tr_key("guided-feed-ouvert-view"))
	if ctx["defender"] and active:
		if lead != null:
			lines.append(texts.tr_key("guided-null-ouvert-defender-lead-role" if ctx["ouvert"]
				else "guided-null-defender-lead-role"))
		elif position == 2:
			lines.append(texts.tr_key("guided-null-defender-last-role"))
		elif ctx["partner_lead"]:
			lines.append(texts.tr_key("guided-null-partner-role"))
		else:
			lines.append(texts.tr_key("guided-null-defender-role"))
	if is_null and ctx["declarer_human"] and (position == 2 or (position == 1 and step == "Orient")):
		lines.append(texts.tr_key("guided-null-middle-role" if position == 1 else "guided-null-role"))
	if step == "Unsupported" and view["phase"] == "Spiel" and contract is Dictionary \
			and not guided["legal_cards"].is_empty():
		lines.append(texts.tr_key("guided-unsupported-card", {
			"game": " · ".join(_contract_parts(contract["game"], contract["hand"] == true, contract)),
			"position": texts.tr_key(["guided-unsupported-lead", "guided-unsupported-second",
				"guided-unsupported-last"][mini(position, 2)])}))
		if contract["ouvert"] == true and view["declarer_cards"] != null:
			lines.append(texts.tr_key("guided-unsupported-revealed"))
	else:
		lines.append(_guided_prompt(guided, view))
	_retry_lines(guided, lines)
	if lead != null and step == "Orient":
		for answer in [["guided-lead-declarer", true], ["guided-lead-defender", false]]:
			_dialogue_button(texts.tr_key(answer[0]),
				_guided_command(guided, {"Answer": {"Lead": {"seat": lead, "declarer": answer[1]}}}))
	if lead != null and step in ["Check", "Narrow"]:
		var farbe: bool = contract is Dictionary and contract["game"] is Dictionary
		if is_null: lines.append(texts.tr_key("guided-null-lead-ranks"))
		elif guided.get("lead_fallback", false): lines.append(texts.tr_key("guided-lead-fallback-definition"))
		elif guided.get("lead_trumpf", false): lines.append(texts.tr_key("guided-draw-trumpf-definition"))
		elif farbe: lines.append(texts.tr_key("guided-lead-suit-definition"))
		else: lines.append(texts.tr_key("guided-lead-definition"))
		if is_null:
			if ctx["defender"]:
				lines.append(texts.tr_key("guided-null-ouvert-defender-lead-limit" if ctx["ouvert"]
					else "guided-null-defender-lead-limit"))
			else:
				lines.append(texts.tr_key("guided-null-lead-limit"))
		elif guided.get("lead_fallback", false): lines.append(texts.tr_key("guided-lead-fallback-risk"))
		elif guided.get("lead_trumpf", false): lines.append(texts.tr_key("guided-draw-trumpf-risk"))
		else: lines.append(texts.tr_key("guided-lead-risk"))
	if lead == null and step in ["Orient", "Check"]:
		for seat in ["Vorhand", "Mittelhand", "Hinterhand"]:
			_dialogue_button(_seat_name(seat), _guided_command(guided, {"Answer": {"Seat": seat}}))
		if step == "Check":
			_dialogue_button(texts.tr_key("guided-none"), _guided_command(guided, {"Answer": {"Seat": null}}))
	if step == "Narrow" or (lead != null and step == "Check"):
		for card in guided["legal_cards"]:
			_dialogue_button(texts.tr_key("guided-consider", {"card": _card(int(card))}),
				_guided_command(guided, {"Answer": {"Card": int(card)}}))
	if guided.get("evidence") != null and guided["correct"] == false and step == "Orient":
		lines.append(_source_line(guided["evidence"]))
	if guided["rule"] != null and step == "Act":
		lines.append(_guided_rule(guided, view))
		var open: bool = guided.get("memory") != null
		_dialogue_button(texts.tr_key("guided-memory-hide" if open else "guided-memory"),
			_guided_command(guided, {"Memory": not open}))
	var memory = guided.get("memory")
	if memory is Dictionary:
		lines.append(texts.tr_key("guided-memory-counts", {"own": _card_count(memory["own"]),
			"played": _card_count(memory["played"]), "unseen": _card_count(memory["unseen"])}))
		lines.append(texts.tr_key("guided-known-skat", {"count": _card_count(memory["skat"])})
			if memory["skat"] != null else texts.tr_key("guided-unknown-skat"))
		if memory["trumpf"] != null:
			lines.append(texts.tr_key("guided-trumpf", {"count": int(memory["trumpf"]["outside"])}))
	if not guided.get("lead_replies", []).is_empty():
		var replies: Array[String] = []
		for source in guided["lead_replies"]:
			replies.append(_source_line(source))
		lines.append("; ".join(replies))
	if lead == null and step in ["Observe", "Complete"] and guided["observed_source"] != null:
		lines.append(_source_line(guided["observed_source"]))
	if step == "Observe":
		for answer in [["guided-void-yes", true], ["guided-void-no", false]]:
			_dialogue_button(texts.tr_key(answer[0]), _guided_command(guided, {"Answer": {"Void": answer[1]}}))
	var observation = guided["observation"]
	if observation != null:
		var deduction := texts.tr_key("guided-null-proves-void" if observation["proves_void"] and is_null
			else ("guided-proves-void" if observation["proves_void"] else "guided-only-card"))
		var winner := ""
		if observation["winner"] != null:
			winner = texts.tr_key("guided-winner", {"seat": _seat_name(observation["winner"])})
		lines.append((deduction + " " + winner).strip_edges() if lead != null else deduction)
		if lead == null and winner != "":
			lines.append(winner)
	_reveal_buttons(guided, ["Orient", "Check", "Narrow", "Observe"])
	_figures_part(guided, view, lines)
	_dialogue_button(texts.tr_key("guided-return"), _guided_command(guided, "Return"))

# Separately requested model comparison of the original decision (opt-in
# "figures" setting); the numbers come from skat-app, never from here.
func _figures_part(guided: Dictionary, view: Dictionary, lines: Array[String]) -> void:
	if guided["step"] == "Complete" and show_figures and not guided.get("figures_requested", false):
		_dialogue_button(texts.tr_key("guided-figures"), _guided_command(guided, "Figures"))
	if not guided.get("figures_requested", false):
		return
	lines.append(texts.tr_key("guided-model-title"))
	var hints = guided.get("figures")
	if hints == null:
		lines.append(texts.tr_key("guided-model-unavailable"))
		return
	var id: Dictionary = guided["id"]
	var budget = guided.get("budget")
	lines.append(texts.tr_key("guided-budget", {"samples": int(budget["samples"]) if budget != null else 0,
		"nodes": int(budget["nodes_per_task"]) if budget != null else 0,
		"game": int(id["game"]), "actions": int(id["actions"])}))
	lines.append(texts.tr_key("coach-basis-real-deal" if str(guided["basis"]) == "RealDeal"
		else "guided-basis-own"))
	var party := texts.tr_key({"Bidder": "coach-party-own", "Declarer": "coach-party-declarer",
		"Defender": "coach-party-defenders"}.get(str(hints["role"]), "coach-party-own"))
	for figure in hints["figures"] if hints["figures"] != null else []:
		lines.append(_label(figure["action"], int(view["bid"]), view["hand_game"]))
		var e = figure["estimate"]
		if e == null:
			lines.append(texts.tr_key("coach-judgement-not-evaluated"))
			continue
		var text := texts.tr_key("coach-metric-win", {"party": party,
			"chance": "%.0f %%" % (float(e["win_chance"]) * 100.0)}) + " · " \
			+ texts.tr_key("coach-metric-score", {"score": "%+.1f" % float(e["expected_score"]),
			"se": "%.1f" % float(e["score_std_error"])})
		if str(hints["role"]) != "Declarer":
			text += " · " + texts.tr_key("coach-metric-game-points",
				{"points": "%+.1f" % float(e["expected_game_points"])})
		lines.append(text)

# Skataufnahme, Drücken and Ansage: candidate branch, threshold, card or value
# attempt, then Easy's actual Hand/pickup, discard or announcement.
func _show_preplay(guided: Dictionary, view: Dictionary, lines: Array[String]) -> void:
	var p: Dictionary = guided["preplay"]
	var step: String = guided["step"]
	var phase: String = str(p["phase"])
	var pickup: bool = phase == "SkatAufnahme"
	var ansage: bool = phase == "Ansage"
	var game = p.get("game")
	var name: String = _game_name(game) if game != null else ""
	if p.get("bid") != null:
		lines.append(texts.tr_key("guided-ann-public-hand" if p.get("hand_game") == true
			else "guided-ann-public-pickup", {"bid": int(p["bid"])}))
	if step == "Orient":
		lines.append(texts.tr_key("guided-ann-orient" if ansage else
			("guided-pre-orient-pickup" if pickup else "guided-pre-orient-discard")))
	elif step == "Threshold" and game != null:
		if ansage:
			lines.append(texts.tr_key("guided-ann-threshold", {"game": name, "bid": int(p["bid"])}))
		elif pickup:
			lines.append(texts.tr_key("guided-pre-threshold-pickup",
				{"game": name, "threshold": int(p["threshold"])}))
		else:
			lines.append(texts.tr_key("guided-pre-threshold-discard", {"game": name}))
	elif step == "Act":
		lines.append(texts.tr_key("guided-ann-act" if ansage else
			("guided-pre-act-pickup" if pickup else "guided-pre-act-discard")))
	else:
		lines.append(texts.tr_key("guided-unsupported"))
	_retry_lines(guided, lines)
	var answers: Array = []
	if step == "Orient":
		for branch in ["Grand", "Suit", "Null", "Weak"]:
			answers.append([texts.tr_key("guided-pre-" + branch.to_lower()), {"Game": branch}])
	elif step == "Threshold" and pickup:
		answers = [[texts.tr_key("guided-pre-hand-yes"), {"Within": true}],
			[texts.tr_key("guided-pre-hand-no"), {"Within": false}]]
	elif step == "Threshold" and ansage:
		answers = [[texts.tr_key("guided-ann-fits-yes"), {"Within": true}],
			[texts.tr_key("guided-ann-fits-no"), {"Within": false}]]
	elif step == "Threshold":
		for card in p["cards"]:
			answers.append([texts.tr_key("guided-pre-consider", {"card": _card(int(card))}),
				{"Card": int(card)}])
	for answer in answers:
		_dialogue_button(answer[0], _guided_command(guided, {"Answer": answer[1]}))
	var rule = guided.get("pre_rule")
	if rule is Dictionary:
		_pre_rule_lines(rule, ansage, pickup, view, lines)
	_reveal_buttons(guided, ["Orient", "Threshold"])
	_dialogue_button(texts.tr_key("guided-return"), _guided_command(guided, "Return"))

func _pre_rule_lines(rule: Dictionary, ansage: bool, pickup: bool, view: Dictionary,
		lines: Array[String]) -> void:
	var kind: String = rule.keys()[0]
	var r: Dictionary = rule[kind]
	var game = r["preferred"] if kind == "Announce" else r["game"]
	lines.append(texts.tr_key("guided-bid-rule", {"action": _label(r["action"], int(view["bid"]), view["hand_game"])}))
	if r["fallback"]:
		lines.append(texts.tr_key("guided-pre-fallback"))
	lines.append(texts.tr_key("guided-ann-preferred" if ansage else "guided-pre-game", {"game": _game_name(game)}))
	lines.append(texts.tr_key("guided-pre-branch-" + str(r["branch"]).to_lower()))
	if kind == "Pickup":
		if str(r["branch"]) == "Null":
			lines.append(texts.tr_key("guided-pre-strength-null"))
		else:
			lines.append(texts.tr_key("guided-pre-strength-hand" if int(r["strength"]) >= int(r["threshold"])
				else "guided-pre-strength-pickup", {"strength": int(r["strength"]), "threshold": int(r["threshold"])}))
			lines.append(texts.tr_key("guided-pre-strength-count"))
	elif kind == "Discard":
		lines.append(texts.tr_key("guided-pre-keeps-null" if str(r["branch"]) == "Null" else "guided-pre-keeps"))
		var put := 0
		for reason in r["reasons"]:
			var card := int(reason[0])
			put |= 1 << card
			var keep = reason[1]
			var args := {"card": _card(card)}
			if keep is Dictionary:
				args["length"] = int(keep["Side"]["suit"])
				lines.append(texts.tr_key("guided-pre-keep-side", args))
			else:
				lines.append(texts.tr_key({"Null": "guided-pre-keep-null", "Trumpf": "guided-pre-keep-trumpf",
					"Ass": "guided-pre-keep-ass", "GuardedTen": "guided-pre-keep-ten"}[str(keep)], args))
		var ties: Array[String] = []
		for i in range(32):
			if int(r["eligible"]) & (1 << i) and not put & (1 << i):
				ties.append(_card(i))
		if not ties.is_empty():
			lines.append(texts.tr_key("guided-pre-tie", {"cards": ", ".join(ties)}))
	else:
		_announce_lines(r, lines)
	lines.append(texts.tr_key("guided-pre-caveat-pickup" if pickup else
		("guided-ann-caveat" if ansage else "guided-pre-caveat-discard")))

# Easy's announcement: preferred value against the bid, the fitting candidates
# with the strength tie-break, the Null ouvert condition and every fallback.
func _announce_lines(r: Dictionary, lines: Array[String]) -> void:
	lines.append(_skat_line(r["skat"]))
	var values := {"value": int(r["value"]), "bid": int(r["bid"])}
	lines.append(texts.tr_key("guided-ann-fits" if r["fits"] else "guided-ann-short", values))
	if r["no_fit"]:
		lines.append(texts.tr_key("guided-ann-no-fit", {"bid": int(r["bid"])}))
	elif not r["fits"]:
		var fitting: Array[String] = []
		var top = null
		for c in r["candidates"]:
			if c["game"] == r["game"] and top == null:
				top = c
			if c["fits"]:
				fitting.append(texts.tr_key("guided-ann-candidate", {"game": _game_name(c["game"]),
					"value": int(c["value"]), "strength": int(c["strength"])}))
		lines.append(texts.tr_key("guided-ann-candidates", {"games": ", ".join(fitting)}))
		var tied: Array[String] = []
		for c in r["candidates"]:
			if c["fits"] and top != null and int(c["strength"]) == int(top["strength"]):
				tied.append(_game_name(c["game"]))
		lines.append(texts.tr_key("guided-ann-tie", {"games": ", ".join(tied), "game": _game_name(r["game"])})
			if tied.size() > 1 else texts.tr_key("guided-ann-strongest", {"game": _game_name(r["game"])}))
	if r["null_risks"] != null:
		lines.append(texts.tr_key("guided-ann-ouvert") if int(r["null_risks"]) == 0
			else texts.tr_key("guided-ann-null-plain", {"count": int(r["null_risks"])}))
	if r["plain"]:
		lines.append(texts.tr_key("guided-ann-plain"))

# Kontra and Re: public factor and threshold attempt, then Easy's actual call
# and the next public transition after the learner's own call or card.
func _show_kontra(guided: Dictionary, view: Dictionary, lines: Array[String]) -> void:
	var k: Dictionary = guided["kontra"]
	var step: String = guided["step"]
	var re: bool = str(k["call"]) == "Re"
	var game = k["game"]
	var is_null: bool = str(game) == "Null"
	var grand: bool = str(game) == "Grand"
	var ouvert_round: bool = k["ouvert_round"]
	var contract = view["contract"]
	var by_game := func(farbe: String, g: String, n: String) -> String:
		return n if is_null else (g if grand else farbe)
	var pick := func(kontra: String, re_key: String) -> String:
		return re_key if re else kontra
	if not step in ["Waiting", "Complete"]:
		lines.append(texts.tr_key("guided-kontra-public-re" if re else "guided-kontra-public-kontra",
			{"game": _game_name(game)}))
	if ouvert_round:
		lines.append(texts.tr_key("guided-kontra-ouvert-before"))
	elif contract is Dictionary and contract["ouvert"] == true:
		lines.append(texts.tr_key("guided-kontra-ouvert-view"))
	var figures := {"factor": int(k["factor"]) if k["factor"] != null else 0,
		"threshold": int(k["threshold"]) if k["threshold"] != null else 0}
	if step == "Orient":
		lines.append(texts.tr_key("guided-kontra-orient-re" if re else "guided-kontra-orient-kontra"))
	elif step == "Threshold" and ouvert_round:
		lines.append(texts.tr_key("guided-kontra-ouvert-question"))
	elif step == "Threshold":
		lines.append(texts.tr_key(pick.call(
			by_game.call("guided-kontra-threshold-kontra", "guided-kontra-threshold-kontra-grand",
				"guided-kontra-threshold-kontra-null"),
			by_game.call("guided-kontra-threshold-re", "guided-kontra-threshold-re-grand",
				"guided-kontra-threshold-re-null")), figures))
	elif step == "Act":
		lines.append(texts.tr_key("guided-kontra-ouvert-act" if ouvert_round else
			("guided-kontra-act-re" if re else "guided-kontra-act-kontra")))
	elif step == "Waiting":
		lines.append(texts.tr_key("guided-kontra-waiting"))
	elif step == "Complete":
		lines.append(texts.tr_key("guided-kontra-complete"))
	else:
		lines.append(texts.tr_key("guided-unsupported-kontra"))
	_retry_lines(guided, lines)
	var answers: Array = []
	if step == "Orient":
		for factor in [1, 2, 4]:
			answers.append([texts.tr_key("guided-kontra-factor-button", {"factor": factor}), {"Factor": factor}])
	elif step == "Threshold" and ouvert_round:
		answers = [[texts.tr_key("guided-kontra-ouvert-yes"), {"Applies": true}],
			[texts.tr_key("guided-kontra-ouvert-no"), {"Applies": false}]]
	elif step == "Threshold" and is_null:
		answers = [[texts.tr_key("guided-kontra-applies-yes"), {"Applies": true}],
			[texts.tr_key("guided-kontra-applies-no"), {"Applies": false}]]
	elif step == "Threshold":
		answers = [[texts.tr_key("guided-kontra-yes"), {"Within": true}],
			[texts.tr_key("guided-kontra-no"), {"Within": false}]]
	for answer in answers:
		_dialogue_button(answer[0], _guided_command(guided, {"Answer": answer[1]}))
	var rule = guided.get("kontra_rule")
	if rule is Dictionary:
		lines.append(texts.tr_key("guided-bid-rule", {"action": _label(rule["action"], int(view["bid"]), view["hand_game"])}))
		if re and not is_null:
			lines.append(_skat_line(rule["skat"]))
		var wants: bool = rule["wants"]
		var key := ""
		if is_null: key = "guided-kontra-count-re-null" if re else "guided-kontra-count-kontra-null"
		elif not re and not wants and grand: key = "guided-kontra-count-kontra-grand"
		elif not re: key = "guided-kontra-count-kontra-" + ("yes" if wants else "no")
		else: key = "guided-kontra-count-re-" + ("yes" if wants else "no")
		lines.append(texts.tr_key("guided-kontra-ouvert-rule") if ouvert_round else
			texts.tr_key(key, {"count": int(rule["count"]), "threshold": int(rule["threshold"])}))
		if rule["card"] != null:
			lines.append(texts.tr_key("guided-kontra-card", {"card": _card(int(rule["card"]["card"]))}))
		lines.append(texts.tr_key("guided-kontra-fact-factor" if not re or is_null
			else "guided-pre-strength-count"))
		if not ouvert_round:
			lines.append(texts.tr_key(pick.call(
				by_game.call("guided-kontra-caveat-kontra", "guided-kontra-caveat-kontra-grand",
					"guided-kontra-caveat-null"),
				by_game.call("guided-kontra-caveat-re", "guided-kontra-caveat-re", "guided-kontra-caveat-null"))))
	var observation = guided.get("kontra_observation")
	if observation != null:
		lines.append_array(_kontra_observation_lines(observation))
	_reveal_buttons(guided, ["Orient", "Threshold"])
	_figures_part(guided, view, lines)
	_dialogue_button(texts.tr_key("guided-return"), _guided_command(guided, "Return"))

func _level_factor(level: Variant) -> int:
	return {"None": 1, "Kontra": 2, "Re": 4}.get(str(level), 1)

func _kontra_observation_lines(observation: Variant) -> Array[String]:
	if str(observation) == "DeclinedCall":
		return [texts.tr_key("guided-kontra-ouvert-declined")]
	var kind: String = observation.keys()[0]
	var o: Dictionary = observation[kind]
	var factor := _level_factor(o["level"])
	match kind:
		"Reply":
			return [texts.tr_key("guided-kontra-reply-re" if o["re"] else "guided-kontra-reply-kein-re",
				{"seat": _seat_name(o["seat"]), "factor": factor})]
		"Resumed":
			return [texts.tr_key("guided-kontra-resumed", {"seat": _seat_name(o["seat"]),
				"card": _card(int(o["card"])), "factor": factor})]
	var out: Array[String] = [texts.tr_key("guided-kontra-declined", {"card": _card(int(o["card"])),
		"trick": int(o["trick"]) + 1, "factor": factor})]
	if o["winner"] != null:
		out.append(texts.tr_key("guided-winner", {"seat": _seat_name(o["winner"])}))
	return out

# `CardSet` crosses the wire as a bit mask (one bit per card); only its size is shown.
# Reizen dialogue: orientation (hold or name a bid), Easy's ceiling attempt, the
# rule and then any legal bidding action in the table's own buttons.
func _show_reizen(guided: Dictionary, lines: Array[String]) -> void:
	var r: Dictionary = guided["reizen"]
	var step: String = guided["step"]
	var current := int(r["current"])
	lines.append(texts.tr_key("guided-bid-current-none") if current == 0
		else texts.tr_key("guided-bid-current", {"bid": current}))
	var stake = r.get("stake")
	if step == "Orient":
		lines.append(texts.tr_key("guided-bid-orient"))
	elif step == "Threshold" and stake != null:
		lines.append(texts.tr_key("guided-bid-threshold-hold" if r.get("holds") == true
			else "guided-bid-threshold-ask", {"value": int(stake)}))
	elif step == "Threshold":
		lines.append(texts.tr_key("guided-bid-threshold-none", {"bid": current}))
	elif step == "Act":
		lines.append(texts.tr_key("guided-bid-act"))
	else:
		lines.append(texts.tr_key("guided-unsupported"))
	if guided["correct"] == false:
		lines.append(texts.tr_key("guided-wrong"))
		_dialogue_button(texts.tr_key("guided-retry"), _guided_command(guided, "Retry"))
	var answers: Array = []
	if step == "Orient":
		answers = [["guided-bid-holds", {"Holds": true}], ["guided-bid-asks", {"Holds": false}]]
	elif step == "Threshold":
		answers = [["guided-bid-within-yes", {"Within": true}], ["guided-bid-within-no", {"Within": false}]]
	for answer in answers:
		_dialogue_button(texts.tr_key(answer[0]), _guided_command(guided, {"Answer": answer[1]}))
	var rule = guided.get("bid_rule")
	if rule is Dictionary:
		lines.append(texts.tr_key("guided-bid-rule", {"action": _label(rule["action"], current, null)}))
		var branch: String = str(rule["branch"])
		if branch == "Weak":
			lines.append(texts.tr_key("guided-bid-weak"))
		else:
			var game = rule["game"]
			var name: String = texts.tr_key("suit-" + str(game["Farbe"]).to_lower()) \
				if game is Dictionary else texts.tr_key("game-" + str(game).to_lower())
			lines.append(texts.tr_key("guided-bid-game", {"game": name, "ceiling": int(rule["ceiling"])}))
			lines.append(texts.tr_key("guided-bid-branch-" + branch.to_lower()))
		var value = rule.get("stake")
		if value == null:
			lines.append(texts.tr_key("guided-bid-no-value"))
		else:
			lines.append(texts.tr_key("guided-bid-within" if int(value) <= int(rule["ceiling"])
				else "guided-bid-above", {"value": int(value), "ceiling": int(rule["ceiling"])}))
		lines.append(texts.tr_key("guided-bid-caveat"))
	if step in ["Orient", "Threshold"]:
		_dialogue_button(texts.tr_key("guided-unknown"), _guided_command(guided, "Reveal"))
		_dialogue_button(texts.tr_key("guided-reveal"), _guided_command(guided, "Reveal"))
	_dialogue_button(texts.tr_key("guided-return"), _guided_command(guided, "Return"))

func _card_count(mask: Variant) -> int:
	var bits := int(mask)
	var count := 0
	while bits != 0:
		count += bits & 1
		bits >>= 1
	return count

func _show_abstich(abstich: Dictionary, view: Dictionary) -> void:
	var lines: Array[String] = [texts.tr_key("live-abstich-title"), texts.tr_key("live-abstich-note")]
	var identity = abstich["id"]
	if abstich["unsupported"]:
		lines.append(texts.tr_key("live-abstich-unsupported"))
	if not abstich["unsupported"] or identity != null:
		lines.append(texts.tr_key("live-abstich-select"))
		for card in abstich["candidates"]:
			_dialogue_button(_card(int(card)), {"SelectLiveAbstich": [abstich["boundary"], int(card)]},
				identity != null and int(identity["card"]) == int(card))
	if identity != null and identity["opponent"] != null:
		var player := 0
		for info in view["players"]:
			if info["seat"] == identity["opponent"]:
				player = int(info["player"])
		lines.append(texts.tr_key("live-abstich-question",
			{"card": _card(int(identity["card"])), "player": _player(player)}))
		lines.append(texts.tr_key("live-abstich-definition"))
	var response = abstich["response"]
	if response != null:
		lines.append(texts.tr_key({"Correct": "live-response-correct",
			"OverstatedCertainty": "live-response-overstated", "DeniedPossibility": "live-response-denied",
			"UnderstatedCertainty": "live-response-understated",
			"Skipped": "live-response-skipped"}.get(str(response), "live-response-unsupported")))
	var evidence = abstich["evidence"]
	if evidence != null:
		lines.append(texts.tr_key("focus-possible" if evidence["certainty"] == "Possible" else "focus-certain"))
		lines.append(texts.tr_key("focus-abstich-evidence", {"seat": _seat_name(evidence["seat"]),
			"suit": _suit_name(int(evidence["card"]) / 8), "cards": _card_count(evidence["suit_unseen"]),
			"trumps": _card_count(evidence["trumpf_unseen"])}))
		lines.append(texts.tr_key("focus-boundary-note"))
	if abstich["assisted"]:
		lines.append(texts.tr_key("live-abstich-assisted"))
	if identity != null and identity["opponent"] != null and response == null:
		for answer in [["focus-possible", "Possible"], ["focus-certain", "Certain"], ["focus-absent", "Absent"]]:
			_dialogue_button(texts.tr_key(answer[0]), {"LiveAbstich": [identity, {"Answer": answer[1]}]})
		_dialogue_button(texts.tr_key("live-abstich-help"), {"LiveAbstich": [identity, "Help"]})
		_dialogue_button(texts.tr_key("quiz-skip"), {"LiveAbstich": [identity, "Skip"]})
	_dialogue_button(texts.tr_key("live-abstich-return"), "CloseLiveAbstich")
	$Layout/Dialogue/Text.text = "\n".join(lines)
