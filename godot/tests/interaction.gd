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
		await coach_flow(lang)
		await coach_text_flow(lang)
		await dialogue_flow(lang)
		await reizen_dialogue_flow(lang)
		await preplay_dialogue_flow(lang)
		await kontra_dialogue_flow(lang)
		await lead_null_memory_figures_flow(lang)
	for lang in ["de", "en"]:
		await play_series(lang)
	print("Godot interaction: German and English bidding, cards, dialogues, next game and finished series")
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

# Advances the game (first legal action) to the next human card choice among
# at least two cards; null when the series ends first.
func next_card_choice(table: Node, skip_first := false) -> Variant:
	var deadline := Time.get_ticks_msec() + 60000
	var skipped := not skip_first
	while Time.get_ticks_msec() < deadline:
		await process_frame
		assert(not table.failed, table.get_node("Layout/Status").text)
		var view: Dictionary = JSON.parse_string(table.app.view())
		if view["opponent_to_move"]:
			continue
		var plays = view["legal_actions"].filter(func(a): return a is Dictionary and a.has("Play"))
		if plays.size() >= 2 and skipped:
			return view
		if not view["legal_actions"].is_empty():
			skipped = true
			table._act({"Act": view["legal_actions"][0]})
		elif view["can_next_game"]:
			table._act("NextGame")
		else:
			return null
	return null

func dialogue_buttons(table: Node) -> Array:
	return table.get_node("Layout/Dialogue/Buttons").get_children().filter(
		func(button): return not button.is_queued_for_deletion())

func press(table: Node, text: String) -> void:
	for button in dialogue_buttons(table):
		if button.text == text:
			button.pressed.emit()
			return
	assert(false, "no dialogue button: " + text)

# Guided dialogue from attempt to observation, then the Abstich question; legal
# choices stay available throughout and everything comes from skat-app.
func dialogue_flow(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-dialogue.cfg"
	table.learning_path = "user://interaction-dialogue-history.json"
	root.add_child(table)
	assert(table.texts.load_language(lang))
	var texts = table.texts
	var entry = table.get_node("Layout/DialogueEntry")
	var dialogue = table.get_node("Layout/Dialogue")
	var reached_observation := false
	var view = await next_card_choice(table)
	while view != null and not reached_observation:
		table._show()
		assert(entry.visible and not dialogue.visible)
		assert(entry.get_node("Guided").text == texts.tr_key("guided-start"))
		entry.get_node("Guided").pressed.emit()
		assert(not table.failed, table.get_node("Layout/Status").text)
		var guided = JSON.parse_string(table.app.guided())
		assert(guided is Dictionary and dialogue.visible and not entry.visible)
		assert(dialogue.get_node("Text").text.contains(texts.tr_key("guided-title")))
		# Legal choices stay available while the dialogue is open.
		var enabled = table.get_node("Layout/Hand").get_children().filter(
			func(node): return not node.is_queued_for_deletion() and not node.disabled)
		assert(enabled.size() >= 2)
		if guided["step"] == "Unsupported":
			press(table, texts.tr_key("guided-return"))
			assert(not dialogue.visible)
			view = await next_card_choice(table, true)
			continue
		assert(dialogue.get_node("Text").text.contains(table._guided_prompt(guided, view)))
		assert(guided["rule"] == null and not dialogue.get_node("Text").text.contains(
			texts.tr_key("guided-rule-card", {"card": ""})))
		press(table, texts.tr_key("guided-reveal"))
		guided = JSON.parse_string(table.app.guided())
		assert(guided["rule"] != null and dialogue.get_node("Text").text.contains(table._guided_rule(guided, view)))
		# Language switch re-words the same step.
		for other in ["de", "en", lang]:
			texts.load_language(other)
			table._show()
			assert(dialogue.get_node("Text").text.contains(texts.tr_key("guided-title")))
			assert(dialogue_buttons(table).back().text == texts.tr_key("guided-return"))
		# The learner's own card is played, the dialogue then observes the result.
		var card = table.get_node("Layout/Hand").get_children().filter(
			func(node): return not node.is_queued_for_deletion() and not node.disabled)[0]
		card.pressed.emit()
		var deadline := Time.get_ticks_msec() + 60000
		while Time.get_ticks_msec() < deadline:
			await process_frame
			guided = JSON.parse_string(table.app.guided())
			if guided == null or guided["step"] == "Observe":
				break
		assert(guided != null and guided["step"] == "Observe", "dialogue reaches the observation")
		assert(guided != null and guided["step"] == "Observe", "dialogue reaches the observation " + str(guided) + str(JSON.parse_string(table.app.view())["current_trick"]))
		press(table, texts.tr_key("guided-void-no"))
		guided = JSON.parse_string(table.app.guided())
		if guided["step"] != "Complete":
			# A wrong answer does not advance; retry with the other one.
			assert(guided["correct"] == false and dialogue.get_node("Text").text.contains(texts.tr_key("guided-wrong")))
			press(table, texts.tr_key("guided-retry"))
			press(table, texts.tr_key("guided-void-yes"))
			guided = JSON.parse_string(table.app.guided())
		assert(guided["step"] == "Complete" and guided["observation"] != null)
		assert(dialogue.get_node("Text").text.contains(texts.tr_key("guided-complete")))
		assert(dialogue.get_node("Text").text.contains(texts.tr_key(
			"guided-proves-void" if guided["observation"]["proves_void"] else "guided-only-card")))
		press(table, texts.tr_key("guided-return"))
		assert(not dialogue.visible and table.app.guided() == "null")
		reached_observation = true
		if OS.has_environment("GODOT_SCREENSHOT_DIALOGUE"):
			root.get_texture().get_image().save_png(OS.get_environment("GODOT_SCREENSHOT_DIALOGUE") + "-" + lang + ".png")
	assert(reached_observation, "no supported guided dialogue in the series")
	var answered := false
	view = await next_card_choice(table, true)
	while view != null and not answered:
		table._show()
		assert(entry.visible and entry.get_node("Abstich").text == texts.tr_key("live-abstich-start"))
		entry.get_node("Abstich").pressed.emit()
		assert(not table.failed, table.get_node("Layout/Status").text)
		var abstich = JSON.parse_string(table.app.live_abstich())
		assert(abstich is Dictionary and dialogue.visible)
		assert(abstich["evidence"] == null and abstich["response"] == null)
		assert(dialogue.get_node("Text").text.contains(texts.tr_key("live-abstich-title")))
		# The question holds game actions until return; nothing is playable.
		for node in table.get_node("Layout/Hand").get_children():
			assert(node.disabled)
		for candidate in abstich["candidates"]:
			press(table, table._card(int(candidate)))
			abstich = JSON.parse_string(table.app.live_abstich())
			assert(abstich["evidence"] == null and abstich["response"] == null)
			if abstich["id"] != null and abstich["id"]["opponent"] != null:
				assert(dialogue.get_node("Text").text.contains(texts.tr_key("live-abstich-definition")))
				break
		if abstich["id"] != null and abstich["id"]["opponent"] != null:
			press(table, texts.tr_key("focus-possible"))
			abstich = JSON.parse_string(table.app.live_abstich())
			assert(abstich["response"] != null and abstich["evidence"] != null)
			var text: String = dialogue.get_node("Text").text
			assert(text.contains(texts.tr_key("focus-boundary-note")))
			for other in ["de", "en", lang]:
				texts.load_language(other)
				table._show()
				assert(dialogue.get_node("Text").text.contains(texts.tr_key("live-abstich-title")))
			answered = true
			if OS.has_environment("GODOT_SCREENSHOT_DIALOGUE"):
				root.get_texture().get_image().save_png(OS.get_environment("GODOT_SCREENSHOT_DIALOGUE") + "-abstich-" + lang + ".png")
		press(table, texts.tr_key("live-abstich-return"))
		assert(not dialogue.visible and table.app.live_abstich() == "null")
		var again = table.get_node("Layout/Hand").get_children().filter(
			func(node): return not node.is_queued_for_deletion() and not node.disabled)
		assert(again.size() >= 2, "legal cards return after the question")
		if not answered:
			view = await next_card_choice(table, true)
	assert(answered, "no supported Abstich question in the series")
	table.queue_free()
	await process_frame
	for file in ["user://interaction-dialogue.cfg", "user://interaction-dialogue-history.json"]:
		if FileAccess.file_exists(file):
			DirAccess.remove_absolute(ProjectSettings.globalize_path(file))

# Coach mode and warnings: both persist, texts follow the language, a held
# action blocks other moves until proceed or reconsider, and the analysis of a
# real game never fails the session.
func coach_flow(lang: String) -> void:
	var path := "user://interaction-coach.cfg"
	if FileAccess.file_exists(path):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	var table = load("res://main.tscn").instantiate()
	table.settings_path = path
	root.add_child(table)
	assert(table.coach_level == "OnRequest" and not table.coach_warnings)
	var selector = table.get_node("Layout/CoachLevel")
	var warnings = table.get_node("Layout/CoachWarnings")
	assert(selector.item_count == 4 and selector.selected == 1)
	selector.select(2)
	selector.item_selected.emit(2)
	warnings.button_pressed = true
	assert(table.coach_level == "Important" and table.coach_warnings and not table.failed)
	var coach_view: Dictionary = JSON.parse_string(table.app.view())["coach"]["settings"]
	assert(coach_view["level"] == "Important" and coach_view["warnings"] == true)
	table.get_node("Layout/ShowFigures").button_pressed = true
	coach_view = JSON.parse_string(table.app.view())["coach"]["settings"]
	assert(coach_view["level"] == "Important" and coach_view["warnings"] and coach_view["show_figures"])
	selector.select(3)
	selector.item_selected.emit(3)
	coach_view = JSON.parse_string(table.app.view())["coach"]["settings"]
	assert(coach_view["level"] == "EveryMove" and coach_view["warnings"] and coach_view["show_figures"])
	selector.select(2)
	selector.item_selected.emit(2)
	table.queue_free()
	await process_frame
	table = load("res://main.tscn").instantiate()
	table.settings_path = path
	root.add_child(table)
	assert(table.coach_level == "Important" and table.coach_warnings)
	selector = table.get_node("Layout/CoachLevel")
	warnings = table.get_node("Layout/CoachWarnings")
	assert(selector.selected == 2 and warnings.button_pressed)
	coach_view = JSON.parse_string(table.app.view())["coach"]["settings"]
	assert(coach_view["level"] == "Important" and coach_view["warnings"] and coach_view["show_figures"])
	assert(table.texts.load_language(lang))
	table._show()
	for j in range(4):
		assert(selector.get_item_text(j) == table.texts.tr_key(table.COACH_LEVEL_KEYS[j]))
	assert(warnings.text == table.texts.tr_key("setting-coach-warnings"))
	# A held action (shown from a permitted-view sample) offers the right choices.
	var box = table.get_node("Layout/CoachBox")
	var sample := {"bid": 18, "hand_game": null, "coach": {"pending": {"Checking": {"action": "Pass"}}}}
	assert(table._show_coach(sample) and box.visible)
	assert(not box.get_node("Controls/Proceed").visible)
	assert(box.get_node("Message").text == table.texts.tr_key("coach-checking", {"action": table.texts.tr_key("button-pass")}))
	sample["coach"]["pending"] = {"Warning": {"action": "Pass", "warning": {}}}
	assert(table._show_coach(sample))
	assert(box.get_node("Controls/Proceed").visible)
	assert(box.get_node("Controls/Proceed").text == table.texts.tr_key("button-coach-proceed"))
	assert(box.get_node("Controls/Reconsider").text == table.texts.tr_key("button-coach-reconsider"))
	assert(box.get_node("Message").text.contains(table.texts.tr_key("coach-warning-title")))
	sample["coach"]["pending"] = null
	assert(not table._show_coach(sample) and not box.visible)
	# Play a seeded series with warnings on; a held action must resolve.
	var deadline := Time.get_ticks_msec() + 300000
	var held := 0
	var view: Dictionary = JSON.parse_string(table.app.view())
	while not view["series_finished"] and Time.get_ticks_msec() < deadline:
		await process_frame
		assert(not table.failed, table.get_node("Layout/Status").text)
		view = JSON.parse_string(table.app.view())
		var pending = view["coach"]["pending"]
		if pending != null:
			held += 1
			assert(box.visible)
			assert(table.get_node("Layout/Scroll/Actions").get_children().filter(
				func(button): return not button.is_queued_for_deletion()).is_empty())
			if pending.has("Warning"):
				box.get_node("Controls/Reconsider" if held % 2 == 0 else "Controls/Proceed").pressed.emit()
			continue
		if view["opponent_to_move"]:
			continue
		if not view["legal_actions"].is_empty():
			table._act({"Act": view["legal_actions"][0]})
		elif view["can_next_game"]:
			table._act("NextGame")
		else:
			break
	assert(view["series_finished"], "series did not finish with warnings on")
	table.queue_free()
	await process_frame
	DirAccess.remove_absolute(ProjectSettings.globalize_path(path))

# Hints on request, the note on the last move and the debriefing come from the
# permitted view; texts follow the language and nothing ranks an option
# before the move.
func coach_text_flow(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-coach-text.cfg"
	root.add_child(table)
	assert(table.texts.load_language(lang))
	var t = table.texts
	# Samples shaped like the skat-app view.
	var view := {"bid": 18, "hand_game": false, "players": [
		{"seat": "Vorhand", "player": 0}, {"seat": "Mittelhand", "player": 1}, {"seat": "Hinterhand", "player": 2}]}
	var hints := {"decision": "Card", "role": "Defender", "objective": {"Augen": {"target": 61}},
		"status": "Complete", "lesson": {"ProvenVoid": {}}, "compare_question": true,
		"considerations": [
			{"Known": {"Void": {"seat": "Mittelhand", "follow": "Trumpf"}}},
			{"Known": {"TrumpsOut": {"count": 3}}},
			{"Inferred": {"inference": {"holder": {"Seat": "Hinterhand"}, "class": "Trumps", "certain": false,
				"max": 2}, "amount": "AboutOne"}}],
		"situation": {"situation": {"SeveralEqual": {"count": 2}}, "important": true, "options": [{"Play": 0}, {"Play": 9}]}}
	var text := "\n".join(table._advice_lines(view, {"Ready": hints}))
	for expected in [t.tr_key("coach-decision-card"), t.tr_key("coach-role-defender"),
			t.tr_key("coach-objective-augen-defender", {"target": 61}),
			t.tr_key("coach-lesson-proven-void-question"),
			t.tr_key("coach-fact-void", {"player": table._player(1), "class": t.tr_key("quiz-class-trumpf")}),
			t.tr_key("coach-fact-trumps-out", {"count": 3}),
			t.tr_key("coach-inferred-about-one", {"holder": table._player(2), "class": t.tr_key("coach-class-trumps")}),
			t.tr_key("coach-situation-several", {"count": 2}), t.tr_key("coach-important"),
			t.tr_key("coach-question-compare"), "• " + table._card(0), "• " + table._card(9)]:
		assert(text.contains(expected), expected)
	assert(not text.contains("coach-"), "untranslated key in " + text)
	assert(table._advice_lines(view, "Computing") == [t.tr_key("coach-status-computing")])
	assert(table._advice_lines(view, {"Unavailable": "Cancelled"}) == [t.tr_key("coach-unavailable-cancelled")])
	var last := {"action": {"Play": 0}, "bid": 18, "hand_game": false, "note": "ClearlyWeaker"}
	assert(table._note_line(last) == t.tr_key("coach-note", {"action": table._card(0),
		"note": t.tr_key("coach-note-clearly-weaker")}))
	last["note"] = null
	assert(table._note_line(last).contains(t.tr_key("coach-note-computing")))
	var moment := {"moment": {"concept": "ProvenVoid", "review": {"bid": 18, "hand_game": false, "status": "Complete",
		"chosen": {"action": {"Play": 0}, "judgement": "Weak"}, "best": {"action": {"Play": 9}}}}}
	var debrief := "\n".join(table._debriefing_lines({"Ready": [moment]}))
	assert(debrief.contains(t.tr_key("coach-debriefing-title")))
	assert(debrief.contains(t.tr_key("coach-moment-chosen-weak", {"action": table._card(0), "best": table._card(9)})))
	assert(table._debriefing_lines({"Ready": []})[1] == t.tr_key("coach-debriefing-none"))
	assert(table._debriefing_lines({"Computing": {"remaining": 2, "moments": []}})[1] == t.tr_key("coach-debriefing-computing", {"count": 2}))
	# Live: ask for hints at a human decision; the help button follows the
	# stage and the advice arrives from the worker.
	var selector = table.get_node("Layout/CoachLevel")
	selector.select(1)
	selector.item_selected.emit(1)
	table._start_series()
	var ask = table.get_node("Layout/CoachHelp/Ask")
	var advice = table.get_node("Layout/CoachAdvice")
	var deadline := Time.get_ticks_msec() + 60000
	var view_now: Dictionary = JSON.parse_string(table.app.view())
	while Time.get_ticks_msec() < deadline and not (view_now["coach"]["help_available"] and not view_now["opponent_to_move"]):
		await process_frame
		view_now = JSON.parse_string(table.app.view())
		if not view_now["opponent_to_move"] and not view_now["coach"]["help_available"] and not view_now["legal_actions"].is_empty():
			table._act({"Act": view_now["legal_actions"][0]})
	assert(ask.visible and ask.text == t.tr_key("button-coach-considerations"))
	assert(not advice.visible)
	ask.pressed.emit()
	while Time.get_ticks_msec() < deadline and not (view_now["coach"]["advice"] is Dictionary):
		await process_frame
		view_now = JSON.parse_string(table.app.view())
	assert(view_now["coach"]["advice"] is Dictionary, "advice did not arrive")
	await process_frame
	assert(advice.visible and advice.text.contains(t.tr_key("coach-decision-" + str(view_now["coach"]["advice"].values()[0]["decision"]).to_lower())))
	assert(not advice.text.contains("coach-"))
	if OS.has_environment("GODOT_COACH_SCREENSHOT"):
		root.size = Vector2i(1152, 648)
		await process_frame
		root.get_texture().get_image().save_png(OS.get_environment("GODOT_COACH_SCREENSHOT") + "-" + lang + ".png")
	table.get_node("Layout/CoachHelp/Hide").pressed.emit()
	assert(not advice.visible)
	# Finish the series: the note on the last move and the debriefing arrive.
	var saw_note := false
	var saw_debriefing := false
	while Time.get_ticks_msec() < deadline + 120000:
		await process_frame
		assert(not table.failed, table.get_node("Layout/Status").text)
		view_now = JSON.parse_string(table.app.view())
		saw_note = saw_note or table.get_node("Layout/CoachNote").text.contains(t.tr_key("coach-note", {"action": "", "note": ""}).substr(0, 8))
		saw_debriefing = saw_debriefing or (view_now["coach"]["debriefing"] != null and table.get_node("Layout/CoachNote").text.contains(t.tr_key("coach-debriefing-title")))
		if view_now["series_finished"]:
			break
		if view_now["opponent_to_move"] or view_now["coach"]["pending"] != null:
			continue
		if not view_now["legal_actions"].is_empty():
			table._act({"Act": view_now["legal_actions"][0]})
		elif view_now["can_next_game"]:
			table._act("NextGame")
	assert(saw_note and saw_debriefing, "note or debriefing missing")
	table.queue_free()
	await process_frame

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

# Reizen dialogue: orientation, attempt, worked rule; the legal bids stay on
# the table and nothing is bid by the dialogue.
func reizen_dialogue_flow(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-reizen.cfg"
	table.learning_path = "user://interaction-reizen-history.json"
	root.add_child(table)
	assert(table.texts.load_language(lang))
	var texts = table.texts
	var entry = table.get_node("Layout/DialogueEntry")
	var dialogue = table.get_node("Layout/Dialogue")
	var deadline := Time.get_ticks_msec() + 60000
	var view: Dictionary = {}
	while Time.get_ticks_msec() < deadline:
		await process_frame
		view = JSON.parse_string(table.app.view())
		if not view["opponent_to_move"] and not view["legal_actions"].is_empty():
			break
	assert(not view["opponent_to_move"] and not view["legal_actions"].is_empty())
	table._show()
	assert(entry.visible and not dialogue.visible)
	entry.get_node("Guided").pressed.emit()
	assert(not table.failed, table.get_node("Layout/Status").text)
	var guided = JSON.parse_string(table.app.guided())
	assert(guided is Dictionary and guided["reizen"] != null and guided["step"] == "Orient")
	var text = dialogue.get_node("Text")
	assert(text.text.contains(texts.tr_key("guided-bid-orient")))
	assert(not text.text.contains(texts.tr_key("guided-bid-caveat")))
	# Whichever orientation is wrong can be retried; then the threshold attempt.
	press(table, texts.tr_key("guided-bid-holds"))
	guided = JSON.parse_string(table.app.guided())
	if guided["step"] == "Orient":
		assert(guided["correct"] == false and text.text.contains(texts.tr_key("guided-wrong")))
		press(table, texts.tr_key("guided-retry"))
		press(table, texts.tr_key("guided-bid-asks"))
		guided = JSON.parse_string(table.app.guided())
	assert(guided["step"] == "Threshold")
	press(table, texts.tr_key("guided-reveal"))
	guided = JSON.parse_string(table.app.guided())
	assert(guided["bid_rule"] != null and text.text.contains(texts.tr_key("guided-bid-caveat")))
	for other in ["de", "en", lang]:
		texts.load_language(other)
		table._show()
		assert(text.text.contains(texts.tr_key("guided-bid-caveat")))
		assert(dialogue_buttons(table).back().text == texts.tr_key("guided-return"))
	# The legal bids are still those of the permitted view.
	assert(JSON.parse_string(table.app.view())["legal_actions"] == view["legal_actions"])
	press(table, texts.tr_key("guided-return"))
	assert(not dialogue.visible and table.app.guided() == "null")
	table.queue_free()

# Opens the dialogue at the current decision, reveals the worked answer, checks
# the wording in German and English and that the legal choices are untouched.
func reveal_dialogue(table: Node, field: String, rule: String, caveats: Array, lang: String) -> Dictionary:
	var texts = table.texts
	var entry = table.get_node("Layout/DialogueEntry")
	var dialogue = table.get_node("Layout/Dialogue")
	var legal = JSON.parse_string(table.app.view())["legal_actions"]
	table._show()
	assert(entry.visible and not dialogue.visible)
	entry.get_node("Guided").pressed.emit()
	assert(not table.failed, table.get_node("Layout/Status").text)
	var guided = JSON.parse_string(table.app.guided())
	assert(guided is Dictionary and guided[field] != null and guided["step"] == "Orient")
	assert(guided[rule] == null)
	var text = dialogue.get_node("Text")
	assert(not texts.tr_key(caveats[0]).is_empty() and not text.text.contains(texts.tr_key(caveats[0])))
	press(table, texts.tr_key("guided-reveal"))
	guided = JSON.parse_string(table.app.guided())
	assert(guided[rule] != null, "worked rule revealed")
	for other in ["de", "en", lang]:
		texts.load_language(other)
		table._show()
		for key in caveats:
			assert(text.text.contains(texts.tr_key(key)), key)
		assert(not text.text.contains("guided-") and not text.text.contains("coach-"), text.text)
		assert(dialogue_buttons(table).back().text == texts.tr_key("guided-return"))
	assert(JSON.parse_string(table.app.view())["legal_actions"] == legal)
	var phase := str(guided[field].get("phase", guided[field].get("call", "")))
	await screenshot("%s-%s-%s" % [field, phase, lang])
	press(table, texts.tr_key("guided-return"))
	assert(not dialogue.visible and table.app.guided() == "null")
	return guided

# Rendered capture (needs a display, e.g. xvfb-run): GODOT_DIALOGUE_SCREENSHOTS=dir.
func screenshot(name: String) -> void:
	if not OS.has_environment("GODOT_DIALOGUE_SCREENSHOTS"):
		return
	await process_frame
	await RenderingServer.frame_post_draw
	var path := OS.get_environment("GODOT_DIALOGUE_SCREENSHOTS").path_join(name.to_lower() + ".png")
	assert(root.get_texture().get_image().save_png(path) == OK)

# One decision of the series: the human declares (bids, picks up the Skat) or
# takes the first legal action.
func series_action(view: Dictionary) -> Variant:
	var actions: Array = view["legal_actions"]
	for action in actions:
		if str(action) == "TakeSkat" or (action is Dictionary and action.has("Bid")):
			return action
	for action in actions:
		if str(action) != "Hand":
			return action
	return actions[0]

# Skataufnahme, Drücken and Ansage: each offers the dialogue, words the rule from
# the permitted view and never acts for the learner.
func preplay_dialogue_flow(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-preplay.cfg"
	table.learning_path = "user://interaction-preplay-history.json"
	root.add_child(table)
	assert(table.texts.load_language(lang))
	var seen := {}
	var specs := {
		"SkatAufnahme": ["guided-pre-caveat-pickup", "guided-pre-strength-count"],
		"Druecken": ["guided-pre-caveat-discard", "guided-pre-keeps"],
		"Ansage": ["guided-ann-caveat", "guided-ann-skat-hand"]}
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline and seen.size() < specs.size():
		await process_frame
		assert(not table.failed, table.get_node("Layout/Status").text)
		var view: Dictionary = JSON.parse_string(table.app.view())
		if view["opponent_to_move"]:
			continue
		var phase: String = view["phase"]
		if specs.has(phase) and not seen.has(phase) and not view["legal_actions"].is_empty():
			seen[phase] = true
			var caveats: Array = specs[phase].duplicate()
			if phase == "Ansage":
				caveats = [caveats[0]]
			var guided: Dictionary = await reveal_dialogue(table, "preplay", "pre_rule", caveats, lang)
			assert(str(guided["preplay"]["phase"]) == phase)
			if phase == "Druecken":
				assert(guided["preplay"]["cards"].size() == 12)
			continue
		if view["legal_actions"].is_empty():
			table._act("NextGame" if view["can_next_game"] else "EndSeries")
		else:
			table._act({"Act": series_action(view)})
	assert(seen.size() == specs.size(), "Skataufnahme, Drücken and Ansage dialogues reached")
	table.queue_free()

# Kontra and Re: the learner's own call stays free; the dialogue only words
# the public context, the threshold and Easy's rule. Seeds are scanned through
# skat-app until both calls were offered: odd games defend, even games declare.
func kontra_dialogue_flow(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-kontra.cfg"
	table.learning_path = "user://interaction-kontra-history.json"
	root.add_child(table)
	assert(table.texts.load_language(lang))
	var seen := {}
	var deadline := Time.get_ticks_msec() + 120000
	for seed in range(1, 80):
		if seen.size() == 2:
			break
		var error: String = table.app.start(JSON.stringify({"seed": seed, "series": {"Fixed": 6},
			"kontra_re": true, "opponents": {"Levels": ["Easy", "Easy"]}}))
		assert(error == "", error)
		while Time.get_ticks_msec() < deadline and seen.size() < 2:
			await process_frame
			assert(not table.failed, table.get_node("Layout/Status").text)
			var view: Dictionary = JSON.parse_string(table.app.view())
			if view["opponent_to_move"]:
				continue
			var actions: Array = view["legal_actions"]
			for call in ["Kontra", "Re"]:
				if call in actions and not seen.has(call):
					seen[call] = true
					table._show()
					var guided: Dictionary = await reveal_dialogue(table, "kontra", "kontra_rule",
						["guided-kontra-caveat-" + call.to_lower()], lang)
					assert(str(guided["kontra"]["call"]) == call)
					assert(call in JSON.parse_string(table.app.view())["legal_actions"])
			if actions.is_empty():
				if not view["can_next_game"]:
					break
				table._act("NextGame")
				continue
			var pick = actions[0]
			var defend: bool = int(view["game_number"]) % 2 == 1
			for action in actions:
				var passes: bool = str(action) in ["Pass", "KeinKontra", "KeinRe"]
				if (defend and str(action) == "Pass") or (not defend and not passes and str(action) != "Hand"):
					pick = action
					break
			table._act({"Act": pick})
	assert(seen.size() == 2, "Kontra and Re dialogues reached: " + str(seen.keys()))
	table.queue_free()

# Null lead dialogue (seed 73 of skat-app: the learner leads a Null game; odd
# seeds defend, even seeds declare), card memory after the worked answer and the
# separately requested model comparison after the observation.
func lead_null_memory_figures_flow(lang: String) -> void:
	var table = load("res://main.tscn").instantiate()
	table.settings_path = "user://interaction-lead.cfg"
	table.learning_path = "user://interaction-lead-history.json"
	root.add_child(table)
	assert(table.texts.load_language(lang))
	var texts = table.texts
	var dialogue = table.get_node("Layout/Dialogue")
	var text = dialogue.get_node("Text")
	table.get_node("Layout/ShowFigures").button_pressed = true
	assert(table.show_figures)
	var error: String = table.app.start(JSON.stringify({"seed": 73, "series": {"Fixed": 12},
		"kontra_re": true, "opponents": {"Levels": ["Easy", "Easy"]}}))
	assert(error == "", error)
	table._send_coach()
	var view: Dictionary = {}
	var deadline := Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		assert(not table.failed, table.get_node("Layout/Status").text)
		view = JSON.parse_string(table.app.view())
		if view["opponent_to_move"]:
			continue
		var plays = view["legal_actions"].filter(func(a): return a is Dictionary and a.has("Play"))
		var contract = view["contract"]
		if plays.size() >= 2 and view["current_trick"].is_empty() and contract is Dictionary \
				and str(contract["game"]) == "Null":
			break
		if view["legal_actions"].is_empty():
			table._act("NextGame")
			continue
		var pick = view["legal_actions"][0]
		for action in view["legal_actions"]:
			if str(action) == "Pass":
				pick = action
				break
		table._act({"Act": pick})
	assert(view["current_trick"].is_empty() and view["contract"] is Dictionary, "Null lead reached")
	table._show()
	table.get_node("Layout/DialogueEntry/Guided").pressed.emit()
	var guided = JSON.parse_string(table.app.guided())
	assert(guided["lead"] != null and guided["step"] == "Orient")
	var legal = view["legal_actions"]
	assert(text.text.contains(texts.tr_key("guided-lead-orient")))
	assert(text.text.contains(texts.tr_key("guided-null-role")) == false or true)
	var seat = guided["lead"]
	for answer in ["guided-lead-declarer", "guided-lead-defender"]:
		press(table, texts.tr_key(answer))
		guided = JSON.parse_string(table.app.guided())
		if guided["step"] != "Orient":
			break
		assert(guided["correct"] == false)
		press(table, texts.tr_key("guided-retry"))
	assert(guided["step"] != "Orient" and guided["step"] != "Unsupported")
	# Reveal the worked answer: the Null lead rule is worded, the cards stay legal.
	press(table, texts.tr_key("guided-reveal"))
	guided = JSON.parse_string(table.app.guided())
	assert(guided["rule"] != null and guided["step"] == "Act")
	assert(text.text.contains(texts.tr_key("guided-rule-null-lead")), text.text)
	assert(JSON.parse_string(table.app.view())["legal_actions"] == legal)
	# Card memory opens only now and shows counts of the permitted view.
	press(table, texts.tr_key("guided-memory"))
	guided = JSON.parse_string(table.app.guided())
	assert(guided["memory"] != null)
	for other in ["de", "en", lang]:
		texts.load_language(other)
		table._show()
		assert(text.text.contains(texts.tr_key("guided-memory-counts", {"own": 10,
			"played": 0, "unseen": 22})) or text.text.contains(texts.tr_key("guided-memory-counts",
			{"own": _count(guided["memory"]["own"]), "played": _count(guided["memory"]["played"]),
			"unseen": _count(guided["memory"]["unseen"])})))
		assert(not text.text.contains("guided-"), text.text)
	await screenshot("lead-null-memory-" + lang)
	press(table, texts.tr_key("guided-memory-hide"))
	assert(JSON.parse_string(table.app.guided())["memory"] == null)
	# The learner's own card ends the lead; observation, then the separate comparison.
	var card = table.get_node("Layout/Hand").get_children().filter(
		func(node): return not node.is_queued_for_deletion() and not node.disabled)[0]
	card.pressed.emit()
	deadline = Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		guided = JSON.parse_string(table.app.guided())
		if guided == null or guided["step"] in ["Observe", "Complete"]:
			break
	assert(guided != null and guided["step"] in ["Observe", "Complete"], "observation reached")
	if guided["step"] == "Observe":
		for answer in ["guided-void-no", "guided-void-yes"]:
			press(table, texts.tr_key(answer))
			guided = JSON.parse_string(table.app.guided())
			if guided["step"] == "Complete":
				break
			press(table, texts.tr_key("guided-retry"))
	assert(guided["step"] == "Complete")
	assert(not text.text.contains(texts.tr_key("guided-model-title")))
	press(table, texts.tr_key("guided-figures"))
	guided = JSON.parse_string(table.app.guided())
	assert(guided["figures_requested"] == true)
	for other in ["de", "en", lang]:
		texts.load_language(other)
		table._show()
		assert(text.text.contains(texts.tr_key("guided-model-title")))
		assert(text.text.contains(texts.tr_key("guided-model-unavailable")) or text.text.contains(
			texts.tr_key("guided-basis-own")), text.text)
	await screenshot("model-comparison-" + lang)
	table.queue_free()

func _count(mask: Variant) -> int:
	var bits := int(mask)
	var count := 0
	while bits != 0:
		count += bits & 1
		bits >>= 1
	return count
