# SPDX-License-Identifier: EUPL-1.2
extends SceneTree

func _initialize() -> void:
	run.call_deferred()

func run() -> void:
	await settings_persistence()
	await bidding_reachable()
	for lang in ["de", "en"]:
		await practice_history_flow(lang)
		await assist_flow(lang)
	for lang in ["de", "en"]:
		await play_series(lang)
	print("Godot interaction: German and English bidding, cards, next game and finished series")
	quit(0)

func bidding_reachable() -> void:
	root.size = Vector2i(1152, 648)
	for mode in ["Levels", "GodMode"]:
		for lang in ["de", "en"]:
			for choice in range(2):
				var table = load("res://main.tscn").instantiate()
				table.settings_path = "user://reachability-settings.cfg"
				root.add_child(table)
				table.texts.load_language(lang)
				table.mode = mode
				table._start_series()
				for i in range(600):
					if not JSON.parse_string(table.app.view())["opponent_to_move"]:
						break
					await create_timer(0.1).timeout
				await process_frame
				await process_frame
				var actions = table.get_node("Layout/Scroll/Actions")
				assert(actions.get_child_count() == 2)
				table.ensure_control_visible(actions.get_parent())
				await process_frame
				assert(table.get_node("Layout/Scroll").size.y >= 100)
				for button in actions.get_children():
					var bounds: Rect2 = button.get_global_rect()
					assert(bounds.position.y >= 0 and bounds.end.y <= table.size.y)
				var before: String = table.app.view()
				actions.get_child(choice).pressed.emit()
				assert(table.app.view() != before)
				table.queue_free()
				await process_frame

func practice_history_flow(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-practice.cfg"
	table.learning_path = "user://interaction-practice-history.json"
	root.add_child(table)
	assert(table.texts.load_language(lang))
	table._show()
	var original: Dictionary = JSON.parse_string(table.app.view())
	table.get_node("Layout/PracticeControls/Practice").pressed.emit()
	var first: Dictionary = JSON.parse_string(table.app.view())
	assert(first["practice"] and first["hand"] == original["hand"] and first["open_cards"] == null)
	assert(table.get_node("Layout/PracticeControls/Return").visible)
	assert(table.get_node("Layout/PracticeControls/Practice").text == table.texts.tr_key("practice-restart"))
	table.get_node("Layout/PracticeControls/Practice").pressed.emit()
	assert(JSON.parse_string(table.app.view())["hand"] == first["hand"])
	table.get_node("Layout/PracticeControls/HistoryButton").pressed.emit()
	assert(table.get_node("History/Scroll/Summary").text.contains(table.texts.tr_key("learning-col-answers")))
	if OS.has_environment("GODOT_PRACTICE_SCREENSHOT"):
		await process_frame
		root.get_texture().get_image().save_png(OS.get_environment("GODOT_PRACTICE_SCREENSHOT") + "-" + lang + ".png")
	table.get_node("History").hide()
	table.get_node("Layout/PracticeControls/Return").pressed.emit()
	assert(JSON.parse_string(table.app.view()) == original)
	table.queue_free()
	await process_frame

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
	await opponent_modes(path)
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

# Goldfish, Mephisto and God Mode: the selector persists, restarts the series,
# discloses the mode in both languages and shows every card only in God Mode.
func opponent_modes(path: String) -> void:
	for index in range(4):
		var table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		var selector = table.get_node("Layout/Mode")
		selector.select(index)
		selector.item_selected.emit(index)
		assert(selector.selected == table.MODES.find(table.mode))
		assert(table.get_node("ConfirmRestart").visible)
		table.get_node("ConfirmRestart").confirmed.emit()
		assert(not table.failed, table.get_node("Layout/Status").text)
		table.queue_free()
		await process_frame
		table = load("res://main.tscn").instantiate()
		table.settings_path = path
		root.add_child(table)
		selector = table.get_node("Layout/Mode")
		var mode: String = table.MODES[index]
		assert(selector.selected == index and table.mode == mode)
		assert(not table.failed, table.get_node("Layout/Status").text)
		var view: Dictionary = JSON.parse_string(table.app.view())
		var god := mode == "GodMode"
		assert((view["open_cards"] != null) == god and table.get_node("Layout/OpenCards").visible == god)
		var note = table.get_node("Layout/ModeNote")
		for lang in ["de", "en"]:
			table.texts.load_language(lang)
			table._show()
			assert(selector.get_item_text(index) == table.texts.tr_key(table.MODE_KEYS[index]))
			assert(note.visible == (index > 0))
			if god:
				assert(note.text.contains(table.texts.tr_key("option-open-display-god")))
				assert(note.text.contains(table.texts.tr_key("mode-god-description")))
				assert(table.get_node("Layout/OpenCards").text.begins_with(table.texts.tr_key("mode-god")))
			for child in table.get_node("Layout/Opponents").get_children():
				assert(child.disabled == (index > 0))
		table.queue_free()
		await process_frame
	# A learned engine needs individual levels and takes the mode back to them.
	var table = load("res://main.tscn").instantiate()
	table.settings_path = path
	root.add_child(table)
	table.get_node("Layout/Mode").select(2)
	table.get_node("Layout/Mode").item_selected.emit(2)
	table.get_node("ConfirmRestart").confirmed.emit()
	table.get_node("Layout/Engine").item_selected.emit(1)
	table.get_node("ConfirmRestart").confirmed.emit()
	assert(table.mode == "Levels" and table.engine == "Neural" and not table.failed)
	table.queue_free()
	await process_frame
	var reset = ConfigFile.new()
	reset.load(path)
	reset.set_value("game", "mode", "Levels")
	reset.set_value("game", "engine", "Classic")
	reset.save(path)

# Counting aid and quiz: settings persist, a question holds the move and hides
# the aid, and answer, help and skip reach the learning history.
func assist_flow(lang: String) -> void:
	var path := "user://interaction-assist.cfg"
	var history := "user://interaction-assist-history.json"
	for file in [path, history]:
		if FileAccess.file_exists(file):
			DirAccess.remove_absolute(ProjectSettings.globalize_path(file))
	var table = load("res://main.tscn").instantiate()
	table.settings_path = path
	table.learning_path = history
	root.add_child(table)
	assert(not table.counting_aid and not table.quiz)
	table.get_node("Layout/CountingAid").button_pressed = true
	table.get_node("Layout/Quiz").button_pressed = true
	table.queue_free()
	await process_frame
	table = load("res://main.tscn").instantiate()
	table.settings_path = path
	table.learning_path = history
	root.add_child(table)
	assert(table.counting_aid and table.quiz)
	assert(table.get_node("Layout/CountingAid").button_pressed and table.get_node("Layout/Quiz").button_pressed)
	assert(table.texts.load_language(lang))
	table._show()
	assert(table.get_node("Layout/CountingAid").text == table.texts.tr_key("setting-aid"))
	assert(table.get_node("Layout/Quiz").text == table.texts.tr_key("setting-quiz"))
	assert(table.get_node("Layout/Aid").visible and not table.get_node("Layout/QuizBox").visible)
	var box = table.get_node("Layout/QuizBox")
	var deadline := Time.get_ticks_msec() + 60000
	var view: Dictionary = JSON.parse_string(table.app.view())
	while view["assist"]["quiz"]["question"] == null and Time.get_ticks_msec() < deadline:
		await process_frame
		view = JSON.parse_string(table.app.view())
		if view["assist"]["quiz"]["question"] != null:
			break  # the question arrived during this frame and holds the move
		if view["opponent_to_move"]:
			continue
		if not view["legal_actions"].is_empty():
			table._act({"Act": view["legal_actions"][0]})
		elif view["can_next_game"]:
			table._act("NextGame")
		else:
			break
	assert(view["assist"]["quiz"]["question"] != null, "no counting question was asked")
	assert(box.visible and table.get_node("Layout/Aid").text.contains(table.texts.tr_key("aid-withheld")))
	assert(view["assist"]["counting"] == "Withheld" and view["assist"]["quiz"]["help"] == null)
	assert(box.get_node("Question").text.contains(table._question_text(view["assist"]["quiz"]["question"])))
	for node in table.get_node("Layout/Hand").get_children():
		assert(node.disabled)
	assert(table.get_node("Layout/Scroll/Actions").get_children().filter(
		func(button): return not button.is_queued_for_deletion()).is_empty())
	for other in ["de", "en"]:
		table.texts.load_language(other)
		table._show()
		assert(box.get_node("Question").text.contains(table.texts.tr_key("quiz-hint")))
		assert(box.get_node("Controls/Answer").text == table.texts.tr_key("quiz-submit"))
		assert(box.get_node("Controls/Skip").text == table.texts.tr_key("quiz-skip"))
		assert(box.get_node("Controls/Off").text == table.texts.tr_key("quiz-off"))
	table.texts.load_language(lang)
	table._show()
	var asked: int = table.app.learning_events()
	box.get_node("Controls/ShowHelp").pressed.emit()
	view = JSON.parse_string(table.app.view())
	var help = view["assist"]["quiz"]["help"]
	assert(help != null and view["assist"]["quiz"]["question"] != null)
	assert(box.get_node("Question").text.contains(table._derivation_lines(help)[0]))
	assert(box.get_node("Controls/ShowHelp").disabled)
	assert(table.get_node("Layout/Aid").text.contains(table.texts.tr_key("aid-exact")))
	if lang == "de":
		box.get_node("Controls/Answer").pressed.emit()
	else:
		box.get_node("Controls/Skip").pressed.emit()
	assert(not table.failed, table.get_node("Layout/Status").text)
	view = JSON.parse_string(table.app.view())
	assert(view["assist"]["quiz"]["question"] == null and not box.visible)
	var feedback = view["assist"]["quiz"]["feedback"]
	assert(feedback != null and table.get_node("Layout/QuizFeedback").visible)
	var lines = table._feedback_lines(feedback)
	assert(table.get_node("Layout/QuizFeedback").text.contains(lines[0]))
	assert(table.get_node("Layout/Aid").text.contains(table.texts.tr_key("aid-title")))
	assert(not table.get_node("Layout/Aid").text.contains(table.texts.tr_key("aid-withheld")))
	assert(table.app.learning_events() > asked)
	table.get_node("Layout/PracticeControls/HistoryButton").pressed.emit()
	var summary: Dictionary = JSON.parse_string(table.app.learning_summary())
	assert(not summary["rows"].is_empty())
	assert(table.get_node("History/Scroll/Summary").text.contains(str(int(summary["totals"]["opportunities"]))))
	table.get_node("History").hide()
	var recorded: int = table.app.learning_events()
	table.get_node("Layout/Quiz").button_pressed = false
	assert(not table.quiz and table.app.learning_events() == recorded)
	assert(not box.visible)
	table.queue_free()
	await process_frame
	table = load("res://main.tscn").instantiate()
	table.settings_path = path
	table.learning_path = history
	root.add_child(table)
	assert(table.counting_aid and not table.quiz)
	assert(table.app.learning_events() == recorded, "history survives a new session")
	table.queue_free()
	await process_frame
	for file in [path, history]:
		DirAccess.remove_absolute(ProjectSettings.globalize_path(file))

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
	var saw_skat := false
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
		var announces = actions.filter(func(a): return a is Dictionary and a.has("Announce"))
		var picker_rows := 3 if not announces.is_empty() else 0
		assert(buttons.size() == actions.size() - announces.size() + picker_rows + int(view["can_next_game"]))
		if not announces.is_empty():
			announce(table, view, announces)
			saw_announce = true
			continue
		if view["own_skat"] is Array:
			saw_skat = true
			assert(table.texts.tr_key("label-own-skat") in table.get_node("Layout/Table").text)
		if view["series_finished"]:
			await check_recording(table, lang)
			check_sheet(table, view)
			assert(saw_bid and saw_play and saw_next and saw_card_click and saw_discard and saw_announce and saw_skat and actions.is_empty())
			if OS.has_environment("GODOT_SHEET_SCREENSHOTS"):
				table.texts.load_language(lang)
				table._show()
				await process_frame
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(OS.get_environment("GODOT_SHEET_SCREENSHOTS").path_join(lang + ".png")) == OK)
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
		assert(table.get_node("Layout/Status").text == table.texts.tr_key("series-finished", {"games": int(series["games"])}))
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

# Ansage: the picker lists exactly the legal announcements, in both languages.
func announce(table: Node, view: Dictionary, announces: Array) -> void:
	var picker: OptionButton = table.get_node("Layout/Scroll/Actions/AnnouncePicker")
	assert(picker.item_count == announces.size())
	for lang in ["de", "en"]:
		table.texts.load_language(lang)
		table._show()
		picker = table.get_node("Layout/Scroll/Actions/AnnouncePicker")
		assert(table.get_node("Layout/Scroll/Actions/AnnounceLabel").text == table.texts.tr_key("label-announce"))
		for i in range(announces.size()):
			assert(picker.get_item_text(i) == table._label(announces[i], 0, view["hand_game"]))
	picker.select(picker.item_count - 1)
	table.get_node("Layout/Scroll/Actions/ConfirmAnnounce").pressed.emit()
	assert(not table.failed, table.get_node("Layout/Status").text)

# Save and reopen the finished game, navigating every recorded action.
func check_recording(table: Node, lang: String) -> void:
	table.texts.load_language(lang)
	table._show()
	var path := "user://interaction-recording-" + lang + ".json"
	var absolute := ProjectSettings.globalize_path(path)
	DirAccess.remove_absolute(absolute)
	assert(not table.get_node("Layout/RecordingFiles/SaveRecording").disabled)
	table.get_node("Layout/RecordingFiles/SaveRecording").pressed.emit()
	assert(table.get_node("RecordingConsent").dialog_text == table.texts.tr_key("record-export-warning"))
	table.get_node("RecordingConsent").confirmed.emit()
	table.get_node("RecordingConsent").hide()
	await process_frame
	table.get_node("RecordingFile").file_selected.emit(absolute)
	table.get_node("RecordingFile").hide()
	assert(FileAccess.file_exists(path))
	table._recording_selected(absolute)
	await process_frame
	assert(table.get_node("RecordingError").dialog_text == table.texts.tr_key("record-error-exists"))
	table.get_node("RecordingError").hide()
	table.get_node("Layout/RecordingFiles/OpenRecording").pressed.emit()
	table.get_node("RecordingFile").file_selected.emit(absolute)
	table.get_node("RecordingFile").hide()
	await process_frame
	assert(table.get_node("Replay").visible and table.replay_index == 0)
	var total: int = table.replay_total
	assert(total > 0)
	if OS.has_environment("GODOT_REPLAY_SCREENSHOTS"):
		table.replay_index = int(total / 2)
		table._replay_show()
		await process_frame
		await RenderingServer.frame_post_draw
		assert(table.get_node("Replay").get_texture().get_image().save_png(OS.get_environment("GODOT_REPLAY_SCREENSHOTS").path_join(lang + ".png")) == OK)
		table.replay_index = 0
		table._replay_show()
	for index in range(1, total + 1):
		table.get_node("Replay/Content/Navigation/Next").pressed.emit()
		assert(table.replay_index == index)
	for index in range(total - 1, -1, -1):
		table.get_node("Replay/Content/Navigation/Previous").pressed.emit()
		assert(table.replay_index == index)
	table.get_node("Replay").close_requested.emit()
	assert(not table.replay_active)
	DirAccess.remove_absolute(absolute)
	await process_frame
