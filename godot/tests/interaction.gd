# SPDX-License-Identifier: EUPL-1.2
extends SceneTree

func _initialize() -> void:
	run.call_deferred()

func run() -> void:
	await settings_persistence()
	for lang in ["de", "en"]:
		await play_series(lang)
	print("Godot interaction: German and English bidding, cards, next game and finished series")
	quit(0)

func settings_persistence() -> void:
	var path := "user://interaction-settings.cfg"
	if FileAccess.file_exists(path):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	for lang in ["de", "en"]:
		var table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		var language = table.get_node("Layout/Language")
		var index := 0 if lang == "de" else 1
		language.select(index)
		language.item_selected.emit(index)
		table.queue_free()
		await process_frame
		table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		assert(table.texts.language == lang)
		assert(table.get_node("Layout/Language").selected == index)
		table.queue_free()
		await process_frame
	for enabled in [true, false]:
		var table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		table.get_node("Layout/KontraRe").button_pressed = enabled
		assert(table.kontra_re != enabled)
		table.get_node("ConfirmRestart").confirmed.emit()
		table.queue_free()
		await process_frame
		table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		assert(table.kontra_re == enabled)
		assert(table.get_node("Layout/KontraRe").button_pressed == enabled)
		for lang in ["de", "en"]:
			table.texts.load_language(lang)
			table._show()
			assert(table.get_node("Layout/KontraRe").text == table.texts.tr_key("option-kontra-re"))
		table.queue_free()
		await process_frame
	for enabled in [true, false]:
		var table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		table.get_node("Layout/OpenDisplay").button_pressed = enabled
		assert(table.open_display != enabled)
		table.get_node("ConfirmRestart").confirmed.emit()
		table.queue_free()
		await process_frame
		table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		assert(table.open_display == enabled)
		assert(table.get_node("Layout/OpenDisplay").button_pressed == enabled)
		var open_cards = table.get_node("Layout/OpenCards")
		var view: Dictionary = JSON.parse_string(table.app.view())
		assert((view["open_cards"] != null) == enabled and open_cards.visible == enabled)
		for lang in ["de", "en"]:
			table.texts.load_language(lang)
			table._show()
			assert(table.get_node("Layout/OpenDisplay").text == table.texts.tr_key("option-open-display"))
			if enabled:
				assert(open_cards.text.begins_with(table.texts.tr_key("label-open-cards")))
				assert(open_cards.text.count("\n") == 4)
		table.queue_free()
		await process_frame
	for index in range(4):
		var table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		var selector = table.get_node("Layout/Engine")
		selector.select(index)
		selector.item_selected.emit(index)
		assert(selector.selected == table.ENGINES.find(table.engine))
		assert(table.get_node("ConfirmRestart").visible)
		table.get_node("ConfirmRestart").confirmed.emit()
		assert(not table.failed, table.get_node("Layout/Status").text)
		table.queue_free()
		await process_frame
		table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		selector = table.get_node("Layout/Engine")
		assert(selector.selected == index and table.engine == table.ENGINES[index])
		assert(not table.failed, table.get_node("Layout/Status").text)
		for lang in ["de", "en"]:
			table.texts.load_language(lang)
			table._show()
			assert(selector.get_item_text(index) == table.texts.tr_key(table.ENGINE_KEYS[index]))
			assert(selector.tooltip_text == table.texts.tr_key("opponent-engine"))
		table.queue_free()
		await process_frame
	var reset = ConfigFile.new()
	reset.load(path)
	reset.set_value("game", "engine", "Classic")
	reset.save(path)
	for player in range(2):
		for index in range(4):
			var table = load("res://main.tscn").instantiate()
			table.settings_path = path
			root.add_child(table)
			var selector = table.get_node("Layout/Opponents").get_child(player)
			var before: String = table.app.view()
			selector.select(index)
			selector.item_selected.emit(index)
			assert(selector.selected == table.LEVELS.find(table.levels[player]))
			assert(table.app.view() == before)
			assert(table.get_node("ConfirmRestart").visible)
			table.get_node("ConfirmRestart").confirmed.emit()
			table.queue_free()
			await process_frame
			table = load("res://main.tscn").instantiate()
			table.settings_path = path
			root.add_child(table)
			selector = table.get_node("Layout/Opponents").get_child(player)
			assert(selector.selected == index and table.levels[player] == table.LEVELS[index])
			for lang in ["de", "en"]:
				table.texts.load_language(lang)
				table._show()
				assert(selector.tooltip_text == table.texts.tr_key(["opponent-left", "opponent-right"][player]))
				assert(selector.get_item_text(index) == "%s %s" % [selector.tooltip_text, table.texts.tr_key("level-" + table.LEVELS[index].to_lower())])
			table.queue_free()
			await process_frame
	DirAccess.remove_absolute(ProjectSettings.globalize_path(path))

func _bits(mask: int) -> Array[int]:
	var bits: Array[int] = []
	for i in range(32):
		if mask & (1 << i):
			bits.append(i)
	return bits

func play_series(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-settings.cfg"
	root.add_child(table)
	for label_lang in ["de", "en"]:
		assert(table.texts.load_language(label_lang))
		assert(table._label({"Bid": 18}, 18, false) == table.texts.tr_key("button-bid", {"value": 18}))
		assert(table._label("Hold", 18, false) == table.texts.tr_key("button-hold", {"value": 18}))
		assert(table._label("Pass", 18, false) == table.texts.tr_key("button-pass"))
		var announcement := {"game": {"Farbe": "Kreuz"}, "schneider": true, "schwarz": false, "ouvert": false}
		var label: String = table._label({"Announce": announcement}, 18, true)
		assert(table.texts.tr_key("suit-kreuz") in label and table.texts.tr_key("contract-hand") in label)
		assert(table.texts.tr_key("contract-schneider") in label)
		assert(table._trick([]) == "")
		assert(table._trick([{"player": 1, "card": 0}]) == "%s: %s" % [table.texts.tr_key("player-left"), table._card(0)])
	var language = table.get_node("Layout/Language")
	for selected in [0, 1, 0 if lang == "de" else 1]:
		language.select(selected)
		language.item_selected.emit(selected)
		assert(table.texts.language == ["de", "en"][selected])
		assert(language.tooltip_text == table.texts.tr_key("setting-language"))
		assert(table.texts.tr_key("trick-current") in table.get_node("Layout/Table").text)
	var saw_bid := false
	var saw_play := false
	var saw_next := false
	var saw_card_click := false
	var saw_discard := false
	var saw_announce := false
	var saw_skat_phase := false
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
		var hand_nodes = table.get_node("Layout/Hand").get_children().filter(
			func(node): return not node.is_queued_for_deletion())
		var trick_nodes = table.get_node("Layout/Trick").get_children().filter(
			func(node): return not node.is_queued_for_deletion())
		assert(hand_nodes.size() == view["hand"].size() and trick_nodes.size() == view["current_trick"].size())
		for node in hand_nodes + trick_nodes:
			assert(node.texture_normal != null)
		if view["opponent_to_move"]:
			continue
		var enabled = hand_nodes.filter(func(node): return not node.disabled)
		var plays = view["legal_actions"].filter(func(a): return a is Dictionary and a.has("Play"))
		var discards: Array = view["legal_actions"].filter(func(a): return a is Dictionary and a.has("Discard"))
		if discards.is_empty():
			assert(enabled.size() == plays.size())
		else:
			# Drücken: only cards of a legal discard are selectable and the
			# confirm button offers exactly the selected legal pair.
			var usable := 0
			for discard in discards:
				usable |= int(discard["Discard"])
			assert(enabled.size() == _bits(usable).size() and hand_nodes.size() == 12)
			assert(view["phase"] == "Druecken")
			var confirm = table.get_node("Layout/Scroll/Actions").get_node("ConfirmDiscard")
			assert(confirm.disabled and table.texts.tr_key("godot-discard-hint") == table.get_node("Layout/Status").text)
			var wanted := int(discards[0]["Discard"])
			var first := _bits(wanted)[0]
			var second := _bits(wanted)[1]
			table.get_node("Layout/Hand").get_node("Card%d" % first).pressed.emit()
			# _show rebuilt the nodes: select the second card on the new ones.
			table.get_node("Layout/Hand").get_node("Card%d" % second).pressed.emit()
			assert(table._selection_mask() == wanted)
			await process_frame
			confirm = table.get_node("Layout/Scroll/Actions").get_node("ConfirmDiscard")
			assert(not confirm.disabled)
			saw_discard = true
			confirm.pressed.emit()
			continue
		if not enabled.is_empty() and not saw_card_click and view["legal_actions"].size() == plays.size():
			saw_card_click = true
			enabled[0].pressed.emit()
			continue
		var actions: Array = view["legal_actions"]
		if not view["opponent_to_move"] and view["phase"] == "SkatAufnahme":
			saw_skat_phase = true
		var buttons = table.get_node("Layout/Scroll/Actions").get_children().filter(
			func(button): return not button.is_queued_for_deletion())
		assert(buttons.size() == actions.size() + int(view["can_next_game"]))
		if view["series_finished"]:
			assert(saw_bid and saw_play and saw_next and saw_card_click and actions.is_empty())
			assert(saw_skat_phase and saw_discard and saw_announce, "declarer flow not reached")
			table.queue_free()
			DirAccess.remove_absolute(ProjectSettings.globalize_path(table.settings_path))
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
			if str(action) == "Hold":
				selected = i
			if view["phase"] == "SkatAufnahme" and str(action) == "TakeSkat":
				selected = i
				break
			if action is Dictionary and action.has("Announce"):
				saw_announce = true
				selected = i
				break
			if action is Dictionary and (action.has("Bid") or action.has("Play")):
				selected = i
				if action.has("Bid"):
					saw_bid = true
				if action.has("Play"):
					saw_play = true
		buttons[selected].pressed.emit()
	push_error("Interaction did not reach bidding and card play")
	quit(1)
