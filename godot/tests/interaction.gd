# SPDX-License-Identifier: EUPL-1.2
extends SceneTree

func _initialize() -> void:
	run.call_deferred()

func run() -> void:
	var table = load("res://main.tscn").instantiate()
	root.add_child(table)
	for lang in ["de", "en"]:
		assert(table.texts.load_language(lang))
		assert(table._label({"Bid": 18}, 18, false) == table.texts.tr_key("button-bid", {"value": 18}))
		assert(table._label("Hold", 18, false) == table.texts.tr_key("button-hold", {"value": 18}))
		assert(table._label("Pass", 18, false) == table.texts.tr_key("button-pass"))
		var announcement := {"game": {"Farbe": "Kreuz"}, "schneider": true, "schwarz": false, "ouvert": false}
		var label: String = table._label({"Announce": announcement}, 18, true)
		assert(table.texts.tr_key("suit-kreuz") in label and table.texts.tr_key("contract-hand") in label)
		assert(table.texts.tr_key("contract-schneider") in label)
		assert(table._trick([]) == "")
		assert(table._trick([{"player": 1, "card": 0}]) == "%s: %s" % [table.texts.tr_key("player-left"), table._card(0)])
	var saw_bid := false
	var saw_play := false
	var saw_next := false
	var captured := false
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		assert(not table.failed, table.get_node("Layout/Status").text)
		var view: Dictionary = JSON.parse_string(table.app.view())
		if not captured and OS.has_environment("GODOT_SCREENSHOT") and not view["current_trick"].is_empty():
			assert(table._trick(view["current_trick"]) in table.get_node("Layout/Table").text)
			root.get_texture().get_image().save_png(OS.get_environment("GODOT_SCREENSHOT"))
			captured = true
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
		buttons[selected].pressed.emit()
	push_error("Interaction did not reach bidding and card play")
	quit(1)
