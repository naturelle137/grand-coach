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
	$Layout/RecordingFiles/SaveRecording.pressed.connect(_recording_consent)
	$Layout/RecordingFiles/OpenRecording.pressed.connect(_recording_open_dialog)
	$RecordingConsent.confirmed.connect(func(): _recording_save_dialog.call_deferred())
	$RecordingFile.file_selected.connect(_recording_selected)
	$Replay.close_requested.connect(_replay_close)
	$Replay/Content/Navigation/Previous.pressed.connect(_replay_step.bind(-1))
	$Replay/Content/Navigation/Next.pressed.connect(_replay_step.bind(1))
	_start_series()

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

func _start_series() -> void:
	var config := {"seed": 1, "series": {"Fixed": 3}, "kontra_re": kontra_re,
		"display": "Open" if open_display else "Closed",
		"opponents": {"Levels": levels}}
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
	if pending_setting.has("engine"):
		engine = pending_setting["engine"]
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
	var lines: Array[String] = [texts.tr_key("label-open-cards")]
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
