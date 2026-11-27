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
var pending_setting := {}
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
	_start_series()

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
	_show()

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
	if failed:
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
	for j in range(ENGINES.size()):
		$Layout/Engine.set_item_text(j, texts.tr_key(ENGINE_KEYS[j]))
	var hand: Array[String] = []
	for card in view["hand"]:
		hand.append(_card(int(card)))
	$Layout/Table.text = "%d · %s\n%s\n%s: %s" % [
		view["game_number"], texts.tr_key("phase-" + str(view["phase"]).to_lower()),
		", ".join(hand), texts.tr_key("trick-current"), _trick(view["current_trick"])]
	_show_sheet(view)
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
	for child in $Layout/Scroll/Actions.get_children():
		child.queue_free()
	if not view["opponent_to_move"]:
		for action in view["legal_actions"]:
			if action is Dictionary and action.has("Play"):
				var card := $Layout/Hand.get_node_or_null("Card%d" % int(action["Play"]))
				if card != null:
					card.disabled = false
					card.pressed.connect(_act.bind({"Act": action}))
			_add_button(_label(action, int(view["bid"]), view["hand_game"]), {"Act": action})
		if view["can_next_game"]:
			_add_button(texts.tr_key("button-next-game"), "NextGame")
		elif view["legal_actions"].is_empty():
			$Layout/Status.text = texts.tr_key("series-finished", {"games": view["game_number"]}) if view["series_finished"] else ""

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
