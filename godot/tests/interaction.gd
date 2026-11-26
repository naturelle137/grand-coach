# SPDX-License-Identifier: EUPL-1.2
extends SceneTree

func _initialize() -> void:
	run.call_deferred()

func run() -> void:
	var table = load("res://main.tscn").instantiate()
	root.add_child(table)
	var saw_bid := false
	var saw_play := false
	var saw_next := false
	var captured := false
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		assert(not table.failed, table.get_node("Layout/Status").text)
		var view: Dictionary = JSON.parse_string(table.app.view())
		if view["opponent_to_move"]:
			continue
		var actions: Array = view["legal_actions"]
		var buttons = table.get_node("Layout/Scroll/Actions").get_children().filter(
			func(button): return not button.is_queued_for_deletion())
		assert(buttons.size() == actions.size() + int(view["can_next_game"]))
		if view["series_finished"]:
			assert(saw_bid and saw_play and saw_next and actions.is_empty())
			print("Godot interaction: bidding, cards, next game and finished series")
			quit(0)
			return
		if actions.is_empty():
			if view["can_next_game"]:
				saw_next = true
				buttons.back().pressed.emit()
				continue
			break
		var selected := actions.size() - 1
		for i in range(actions.size()):
			var action = actions[i]
			if action is Dictionary and (action.has("Bid") or action.has("Play")):
				selected = i
				if action.has("Bid"):
					saw_bid = true
				if action.has("Play"):
					saw_play = true
		if not captured and OS.has_environment("GODOT_SCREENSHOT") and actions[selected] is Dictionary and actions[selected].has("Play"):
			root.get_texture().get_image().save_png(OS.get_environment("GODOT_SCREENSHOT"))
			captured = true
		buttons[selected].pressed.emit()
	push_error("Interaction did not reach bidding and card play")
	quit(1)
