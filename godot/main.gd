# SPDX-License-Identifier: EUPL-1.2
extends Control

var app := SkatApp.new()
var failed := false
var texts := preload("res://i18n.gd").new()
var faces := {}
var settings_path := "user://settings.cfg"
var kontra_re := false

func _ready() -> void:
	var language := "en" if OS.get_locale_language() == "en" else "de"
	var settings := ConfigFile.new()
	if settings.load(settings_path) == OK:
		var saved = settings.get_value("display", "language", language)
		if saved in ["de", "en"]:
			language = saved
		kontra_re = settings.get_value("game", "kontra_re", false) == true
	if not texts.load_language(language):
		texts.load_language("de")
	$Layout/Language.add_item(texts.tr_key("language-de"))
	$Layout/Language.add_item(texts.tr_key("language-en"))
	$Layout/Language.select(1 if texts.language == "en" else 0)
	$Layout/Language.item_selected.connect(_language_changed)
	$Layout/KontraRe.button_pressed = kontra_re
	$Layout/KontraRe.toggled.connect(_kontra_re_changed)
	_start_series()

func _start_series() -> void:
	var config := {"seed": 1, "series": {"Fixed": 3}, "kontra_re": kontra_re,
		"opponents": {"Levels": ["Easy", "Easy"]}}
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

## Kontra/Re applies to a new series, as in the egui setup (D-03).
func _kontra_re_changed(enabled: bool) -> void:
	kontra_re = enabled
	_save_setting("game", "kontra_re", enabled)
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
	var hand: Array[String] = []
	for card in view["hand"]:
		hand.append(_card(int(card)))
	$Layout/Table.text = "%d · %s\n%s\n%s: %s" % [
		view["game_number"], texts.tr_key("phase-" + str(view["phase"]).to_lower()),
		", ".join(hand), texts.tr_key("trick-current"), _trick(view["current_trick"])]
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

func _add_button(label: String, command: Variant) -> void:
	var button := Button.new()
	button.text = label
	button.pressed.connect(_act.bind(command))
	$Layout/Scroll/Actions.add_child(button)
