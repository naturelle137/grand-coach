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
var counting_aid := false
var quiz := false
var learning_path := "user://learning-history.json"
var quiz_id := ""
var pending_setting := {}
var replay_active := false
var replay_index := 0
var replay_total := 0
var recording_save := false
var discard_selection: Array[int] = []
const LEVELS := ["Rookie", "Easy", "Medium", "Strong"]
const MODES := ["Levels", "Goldfish", "Mephisto", "GodMode"]
const MODE_KEYS := ["mode-levels", "mode-goldfish", "mode-mephisto", "mode-god"]
const ENGINES := ["Classic", "Neural", "PrePlay", "Hybrid"]
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
	$Layout/QuizBox/Controls/Answer.pressed.connect(_answer_quiz)
	$Layout/QuizBox/Controls/Skip.pressed.connect(_act.bind("SkipQuiz"))
	$Layout/QuizBox/Controls/ShowHelp.pressed.connect(_act.bind("RestoreQuizHelp"))
	$Layout/QuizBox/Controls/Off.pressed.connect(_quiz_off)
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
	_start_series()

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
	_show()

# Assistance (D-64) is set on every new session; changing it never changes
# the game, so it needs no restart.
func _send_assist() -> void:
	var result := app.execute(JSON.stringify({"SetAssist": {
		"counting_aid": counting_aid, "quiz": quiz}}))
	_report_error(result)

func _assist_changed(_enabled: bool) -> void:
	counting_aid = $Layout/CountingAid.button_pressed
	quiz = $Layout/Quiz.button_pressed
	_save_setting("assist", "counting_aid", counting_aid)
	_save_setting("assist", "quiz", quiz)
	_send_assist()
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
	if failed or replay_active:
		return
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
	var bids: Array = view["legal_actions"].filter(func(a):
		return str(a) == "Pass" or (a is Dictionary and a.has("Bid")))
	var entry: bool = not view["opponent_to_move"] and not holding \
		and not (plays.is_empty() and bids.is_empty()) and guided == null and abstich == null
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

func _guided_prompt(guided: Dictionary) -> String:
	var task = guided.get("task")
	var step: String = guided["step"]
	if step == "Orient":
		return texts.tr_key("guided-orient")
	if step == "Check":
		return texts.tr_key("guided-check")
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
		return texts.tr_key("guided-win")
	return texts.tr_key({"Act": "guided-act", "Waiting": "guided-waiting",
		"Observe": "guided-observe", "Complete": "guided-complete"}.get(step, "guided-unsupported"))

# Rule answer after the deliberate reveal; mirrors the egui wording per branch.
func _guided_rule(guided: Dictionary) -> String:
	var rule = guided["rule"]
	var key := "guided-rule-win"
	match rule["branch"]:
		"UnderPartner": key = "guided-rule-under"
		"FeedPartner": key = "guided-rule-feed" if rule["last"] else "guided-rule-feed-early"
		"WinEarly": key = "guided-rule-early"
		"Fallback":
			if guided.get("overtake_forced", false): key = "guided-rule-forced-early"
			elif guided.get("decline_last", false): key = "guided-rule-decline-last"
			elif guided.get("decline_early", false): key = "guided-rule-decline-early"
			elif not rule["partner_ahead"]: key = "guided-rule-decline"
			else: key = "guided-rule-fallback"
	var cards: Array[String] = []
	for card in rule["candidates"]:
		cards.append(_card(int(card)))
	return "%s\n%s\n%s" % [texts.tr_key("guided-rule-card", {"card": _card(int(rule["card"]))}),
		texts.tr_key(key), texts.tr_key("guided-candidates", {"cards": ", ".join(cards)})]

func _source_line(source: Variant) -> String:
	if source is Dictionary and source.has("Trick"):
		var trick: Dictionary = source["Trick"]
		return texts.tr_key("guided-source", {"trick": int(trick["index"]) + 1,
			"seat": _seat_name(trick["seat"]), "card": _card(int(trick["card"]))})
	return ""

func _show_guided(guided: Dictionary, view: Dictionary) -> void:
	var contract = view["contract"]
	var null_game: bool = contract is Dictionary and str(contract["game"]) == "Null"
	# Reizen, Skataufnahme, Drücken, Ansage, Kontra/Re, lead and Null dialogues
	# are not worded in this frontend yet; the learner returns to the table.
	var covered: bool = guided.get("preplay") == null \
		and guided.get("kontra") == null and guided.get("kontra_exclusion") == null \
		and guided.get("lead") == null and not null_game and guided["step"] != "Unsupported"
	var lines: Array[String] = [texts.tr_key("guided-title")]
	if guided.get("reizen") is Dictionary:
		_show_reizen(guided, lines)
		$Layout/Dialogue/Text.text = "\n".join(lines)
		return
	if not covered:
		lines.append(texts.tr_key("guided-unsupported"))
		_dialogue_button(texts.tr_key("guided-return"), _guided_command(guided, "Return"))
		$Layout/Dialogue/Text.text = "\n".join(lines)
		return
	var step: String = guided["step"]
	lines.append(_guided_prompt(guided))
	if guided["correct"] == false:
		lines.append(texts.tr_key("guided-wrong"))
		_dialogue_button(texts.tr_key("guided-retry"), _guided_command(guided, "Retry"))
	if guided["rule"] != null and step == "Act":
		lines.append(_guided_rule(guided))
	for source in [guided["observed_source"]] if step in ["Observe", "Complete"] else []:
		lines.append(_source_line(source))
	var observation = guided["observation"]
	if observation != null:
		lines.append(texts.tr_key("guided-proves-void" if observation["proves_void"] else "guided-only-card"))
		if observation["winner"] != null:
			lines.append(texts.tr_key("guided-winner", {"seat": _seat_name(observation["winner"])}))
	if step in ["Orient", "Check"]:
		for seat in ["Vorhand", "Mittelhand", "Hinterhand"]:
			_dialogue_button(_seat_name(seat), _guided_command(guided, {"Answer": {"Seat": seat}}))
		if step == "Check":
			_dialogue_button(texts.tr_key("guided-none"), _guided_command(guided, {"Answer": {"Seat": null}}))
	if step == "Narrow" or step == "Threshold":
		for card in guided["legal_cards"]:
			_dialogue_button(texts.tr_key("guided-consider", {"card": _card(int(card))}),
				_guided_command(guided, {"Answer": {"Card": int(card)}}))
	if step == "Observe":
		for answer in [["guided-void-yes", true], ["guided-void-no", false]]:
			_dialogue_button(texts.tr_key(answer[0]), _guided_command(guided, {"Answer": {"Void": answer[1]}}))
	if step in ["Orient", "Check", "Narrow", "Observe"]:
		_dialogue_button(texts.tr_key("guided-reveal"), _guided_command(guided, "Reveal"))
	_dialogue_button(texts.tr_key("guided-return"), _guided_command(guided, "Return"))
	$Layout/Dialogue/Text.text = "\n".join(lines)

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
