# SPDX-License-Identifier: EUPL-1.2
extends Control

var app := SkatApp.new()
var failed := false
var texts := preload("res://i18n.gd").new()
var faces := {}
var settings_path := "user://settings.cfg"
var kontra_re := false
var open_display := false
var levels := ["Easy", "Easy"]
var pending_setting := {}
var discard_selection: Array[int] = []
const LEVELS := ["Rookie", "Easy", "Medium", "Strong"]

func _ready() -> void:
	var language := "en" if OS.get_locale_language() == "en" else "de"
	var settings := ConfigFile.new()
	if settings.load(settings_path) == OK:
		var saved = settings.get_value("display", "language", language)
		if saved in ["de", "en"]:
			language = saved
		kontra_re = settings.get_value("game", "kontra_re", false) == true
		open_display = settings.get_value("game", "open_display", false) == true
		for i in range(2):
			var level = settings.get_value("game", "level_%d" % i, "Easy")
			if level in LEVELS:
				levels[i] = level
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

func _level_changed(index: int, player: int) -> void:
	$Layout/Opponents.get_child(player).select(LEVELS.find(levels[player]))
	pending_setting = {"player": player, "level": LEVELS[index]}
	_confirm_restart()

func _confirm_restart() -> void:
	$ConfirmRestart.dialog_text = texts.tr_key("godot-setting-restart-confirm")
	$ConfirmRestart.popup_centered()

func _apply_setting() -> void:
	if pending_setting.has("open_display"):
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

func _discard_masks(actions: Array) -> Array[int]:
	var masks: Array[int] = []
	for action in actions:
		if action is Dictionary and action.has("Discard"):
			masks.append(int(action["Discard"]))
	return masks

func _selection_mask() -> int:
	var mask := 0
	for card in discard_selection:
		mask |= 1 << card
	return mask

func _toggle_discard(card: int) -> void:
	if card in discard_selection:
		discard_selection.erase(card)
	else:
		if discard_selection.size() == 2:
			discard_selection.remove_at(0)
		discard_selection.append(card)
	_show()

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
			var game = announcement["game"]
			var parts: Array[String] = []
			if game is Dictionary:
				parts.append(texts.tr_key("suit-" + str(game["Farbe"]).to_lower()))
			else:
				parts.append(texts.tr_key("game-" + str(game).to_lower()))
			if hand_game == true:
				parts.append(texts.tr_key("contract-hand"))
			for modifier in ["schneider", "schwarz", "ouvert"]:
				if announcement[modifier]:
					parts.append(texts.tr_key("contract-" + modifier))
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
	var hand: Array[String] = []
	for card in view["hand"]:
		hand.append(_card(int(card)))
	$Layout/Table.text = "%d · %s\n%s\n%s: %s" % [
		view["game_number"], texts.tr_key("phase-" + str(view["phase"]).to_lower()),
		", ".join(hand), texts.tr_key("trick-current"), _trick(view["current_trick"])]
	$Layout/OpenCards.text = _open_cards(view["open_cards"])
	$Layout/OpenCards.visible = view["open_cards"] != null
	_clear($Layout/Hand)
	_clear($Layout/Trick)
	var masks: Array[int] = []
	if not view["opponent_to_move"]:
		masks = _discard_masks(view["legal_actions"])
	if masks.is_empty():
		discard_selection.clear()
	for card in view["hand"]:
		var node := _card_node(int(card), _card(int(card)))
		node.disabled = true
		if int(card) in discard_selection:
			node.modulate = Color(1.0, 0.85, 0.3)
		$Layout/Hand.add_child(node)
	for played in view["current_trick"]:
		var node := _card_node(int(played["card"]), "%s: %s" % [texts.tr_key(["player-human", "player-left", "player-right"][int(played["player"])]), _card(int(played["card"]))])
		node.disabled = true
		$Layout/Trick.add_child(node)
	$Layout/Status.text = texts.tr_key("status-waiting", {"player": texts.tr_key(["player-human", "player-left", "player-right"][int(view["acting"])])}) if view["opponent_to_move"] and view["acting"] != null else ""
	_clear($Layout/Scroll/Actions)
	if not view["opponent_to_move"]:
		for action in view["legal_actions"]:
			if action is Dictionary and action.has("Play"):
				var card := $Layout/Hand.get_node_or_null("Card%d" % int(action["Play"]))
				if card != null:
					card.disabled = false
					card.pressed.connect(_act.bind({"Act": action}))
			if action is Dictionary and action.has("Discard"):
				continue
			_add_button(_label(action, int(view["bid"]), view["hand_game"]), {"Act": action})
		if not masks.is_empty():
			var usable := 0
			for mask in masks:
				usable |= mask
			for card in view["hand"]:
				if usable & (1 << int(card)):
					var node := $Layout/Hand.get_node("Card%d" % int(card))
					node.disabled = false
					node.pressed.connect(_toggle_discard.bind(int(card)))
			$Layout/Status.text = texts.tr_key("godot-discard-hint")
			var confirm := _add_button(texts.tr_key("godot-discard-confirm"), {"Act": {"Discard": _selection_mask()}})
			confirm.name = "ConfirmDiscard"
			confirm.disabled = not _selection_mask() in masks
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

func _add_button(label: String, command: Variant) -> Button:
	var button := Button.new()
	button.text = label
	button.pressed.connect(_act.bind(command))
	$Layout/Scroll/Actions.add_child(button)
	return button
