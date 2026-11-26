# SPDX-License-Identifier: EUPL-1.2
# Basic table: shows the permitted view and plays the first legal action on
# a click. All game behaviour comes from skat-app through SkatApp (D-12).
extends Control

var app := SkatApp.new()

func _ready() -> void:
	var config := {"seed": 1, "series": {"Fixed": 3}, "kontra_re": false,
		"opponents": {"Levels": ["Easy", "Easy"]}}
	var err := app.start(JSON.stringify(config))
	if err != "":
		push_error(err)
	_show()

func _process(_delta: float) -> void:
	var events := app.poll_opponent()
	if events != "":
		_show()
	elif app.view() != "" and JSON.parse_string(app.view())["opponent_to_move"]:
		app.start_opponent()

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed:
		var view = JSON.parse_string(app.view())
		if view["opponent_to_move"]:
			return
		var legal: Array = view["legal_actions"]
		var command = {"Act": legal[0]} if legal.size() > 0 else "NextGame"
		var result := app.execute(JSON.stringify(command))
		if result.begins_with("{\"error\""):
			push_error(result)
		_show()

func _show() -> void:
	var view = JSON.parse_string(app.view())
	$Table.text = "Game %d  phase %s\nHand: %s" % [
		view["game_number"], view["phase"], JSON.stringify(view["hand"])]
