# SPDX-License-Identifier: EUPL-1.2
extends Control

var app := SkatApp.new()
var failed := false

func _ready() -> void:
	var config := {"seed": 1, "series": {"Fixed": 3}, "kontra_re": false,
		"opponents": {"Levels": ["Easy", "Easy"]}}
	var error := app.start(JSON.stringify(config))
	if error != "":
		_report_error(error)
		return
	_show()

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
		$Layout/Status.text = "Fehler: " + JSON.stringify(parsed["error"])
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
	return ["Kreuz", "Pik", "Herz", "Karo"][index / 8] + " " + \
		["7", "8", "9", "10", "Bube", "Dame", "König", "Ass"][index % 8]

func _label(action: Variant) -> String:
	if action is Dictionary:
		if action.has("Play"):
			return "Spielen: " + _card(int(action["Play"]))
		if action.has("Discard"):
			var cards: Array[String] = []
			for i in range(32):
				if int(action["Discard"]) & (1 << i):
					cards.append(_card(i))
			return "Drücken: " + ", ".join(cards)
	return JSON.stringify(action)

func _show() -> void:
	var view = JSON.parse_string(app.view())
	if not view is Dictionary:
		return
	var hand: Array[String] = []
	for card in view["hand"]:
		hand.append(_card(int(card)))
	$Layout/Table.text = "Spiel %d · %s\nHand: %s\nStich: %s" % [
		view["game_number"], view["phase"], ", ".join(hand), JSON.stringify(view["current_trick"])]
	$Layout/Status.text = "Gegner am Zug …" if view["opponent_to_move"] else "Aktion wählen"
	for child in $Layout/Scroll/Actions.get_children():
		child.queue_free()
	if not view["opponent_to_move"]:
		for action in view["legal_actions"]:
			_add_button(_label(action), {"Act": action})
		if view["can_next_game"]:
			_add_button("Nächstes Spiel", "NextGame")
		elif view["legal_actions"].is_empty():
			$Layout/Status.text = "Serie beendet" if view["series_finished"] else "Keine Aktion verfügbar"

func _add_button(label: String, command: Variant) -> void:
	var button := Button.new()
	button.text = label
	button.pressed.connect(_act.bind(command))
	$Layout/Scroll/Actions.add_child(button)
