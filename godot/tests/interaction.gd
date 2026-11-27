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
		var discards = view["legal_actions"].filter(func(a): return a is Dictionary and a.has("Discard"))
		if not discards.is_empty():
			assert(enabled.size() == hand_nodes.size() and hand_nodes.size() == 12)
			await discard(table, view, discards)
			saw_discard = true
			continue
		assert(enabled.size() == plays.size())
		if not enabled.is_empty() and not saw_card_click and view["legal_actions"].size() == plays.size():
			saw_card_click = true
			enabled[0].pressed.emit()
			continue
		var actions: Array = view["legal_actions"]
		var buttons = table.get_node("Layout/Scroll/Actions").get_children().filter(
			func(button): return not button.is_queued_for_deletion())
		assert(buttons.size() == actions.size() + int(view["can_next_game"]))
		if view["series_finished"]:
			check_sheet(table, view)
			assert(saw_bid and saw_play and saw_next and saw_card_click and saw_discard and actions.is_empty())
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
			if action is Dictionary and (action.has("Bid") or action.has("Play")):
				selected = i
				if action.has("Bid"):
					saw_bid = true
				if action.has("Play"):
					saw_play = true
		buttons[selected].pressed.emit()
	push_error("Interaction did not reach bidding and card play")
	quit(1)

# The sheet shows exactly the skat-app values, in both languages.
func check_sheet(table: Node, view: Dictionary) -> void:
	assert(view["sheet"]["records"].size() == 3 and view["series_result"] != null)
	for lang in ["de", "en"]:
		table.texts.load_language(lang)
		table._show()
		var rows: Array = table._sheet_rows(view)
		assert(rows.size() == 5)
		assert(rows[0][0] == table.texts.tr_key("sheet-number") and rows[0][1] == table.texts.tr_key("sheet-game"))
		assert(rows[4][0] == table.texts.tr_key("sheet-total"))
		for i in range(3):
			assert(rows[0][2 + i] == table._player(i))
			assert(rows[4][2 + i] == str(int(view["players"][i]["total"])))
			assert(rows[3][2 + i] == str(int(view["sheet"]["records"][2]["totals"][i])))
		for r in range(3):
			var record: Dictionary = view["sheet"]["records"][r]
			assert(rows[1 + r][0] == str(int(record["number"])))
			if record["result"] == null:
				assert(rows[1 + r][1] == table.texts.tr_key("sheet-eingepasst"))
			else:
				assert(rows[1 + r][1].begins_with(table._player(int(record["declarer"]))))
				assert(str(absi(int(record["result"]["score"]))) in rows[1 + r][1])
		var grid = table.get_node("Layout/Sheet")
		assert(grid.get_children().filter(func(n): return not n.is_queued_for_deletion()).size() == 25)
		assert(table.get_node("Layout/SheetTitle").text == table.texts.tr_key("sheet-title"))
		var series: Dictionary = view["series_result"]
		var text: String = table.get_node("Layout/Series").text
		assert(table.get_node("Layout/Series").visible)
		assert(text.begins_with(table.texts.tr_key("series-finished", {"games": int(series["games"])})))
		assert(text.split("\n").size() == 4)
		var best := int(series["ranking"][0])
		assert(table.texts.tr_key("series-rank", {"rank": 1, "player": table._player(best),
			"total": int(series["totals"][best]), "won": int(series["won"][best]),
			"lost": int(series["lost"][best])}) in text)
		# Series figures: the view's cast names and portraits, per language pool.
		var keys: Array = JSON.parse_string(table.app.figure_keys())
		var pool: Array = view["cast"]["german" if lang == "de" else "english"]
		var cast = table.get_node("Layout/Cast")
		var shown: Array = cast.get_children().filter(func(n): return not n.is_queued_for_deletion())
		assert(shown.size() == 4)
		for seat in range(2):
			var key: String = keys[int(pool[seat])]
			assert(key.begins_with("de-" if lang == "de" else "en-"))
			assert(shown[2 * seat + 1].text == table.texts.tr_key("figure-" + key))
			assert(table._player(1 + seat) == shown[2 * seat + 1].text)
			assert(shown[2 * seat].texture != null)

# Drücken: two marked cards confirm exactly one legal Discard action, in both languages.
func discard(table: Node, view: Dictionary, discards: Array) -> void:
	var hand: Array = view["hand"]
	var buttons = func(): return table.get_node("Layout/Scroll/Actions").get_children().filter(
		func(button): return not button.is_queued_for_deletion() and button.name == "ConfirmDiscard")
	assert(buttons.call().size() == 1 and buttons.call()[0].disabled)
	var nodes = func(): return table.get_node("Layout/Hand").get_children().filter(
		func(node): return not node.is_queued_for_deletion())
	nodes.call()[0].button_pressed = true
	assert(table.discard_selection == [int(hand[0])] and buttons.call()[0].disabled)
	nodes.call()[1].button_pressed = true
	var expected: int = (1 << int(hand[0])) | (1 << int(hand[1]))
	assert(table._discard_mask() == expected and not buttons.call()[0].disabled)
	for lang in ["de", "en"]:
		table.texts.load_language(lang)
		table._show()
		assert(table.discard_selection.size() == 2 and not buttons.call()[0].disabled)
		assert(buttons.call()[0].text == table.texts.tr_key("coach-action-discard", {"cards": ", ".join([table._card(int(hand[0])), table._card(int(hand[1]))])}))
		assert(nodes.call()[0].button_pressed and nodes.call()[1].button_pressed)
	nodes.call()[1].button_pressed = false
	assert(buttons.call()[0].disabled)
	nodes.call()[1].button_pressed = true
	buttons.call()[0].pressed.emit()
	assert(not table.failed, table.get_node("Layout/Status").text)
	assert(JSON.parse_string(table.app.view())["phase"] != "Druecken" and table.discard_selection.is_empty())
	await process_frame
