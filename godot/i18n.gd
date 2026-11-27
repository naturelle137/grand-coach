# SPDX-License-Identifier: EUPL-1.2
extends RefCounted
## Reads the single-line messages of `assets/i18n/<lang>.ftl` (D-32), so the
## Godot frontend shares the texts of the egui app. Selector messages
## (`{ $x -> ...}`) and continuation lines are not needed here and skipped.

var messages := {}
var language := "de"

func load_language(lang: String) -> bool:
	var dir := ProjectSettings.globalize_path("res://").path_join("../assets/i18n")
	var file := FileAccess.open(dir.path_join(lang + ".ftl"), FileAccess.READ)
	if file == null:
		return false
	language = lang
	messages = _parse(file.get_as_text())
	return true

static func _parse(text: String) -> Dictionary:
	var parsed := {}
	for line in text.split("\n"):
		if line.begins_with("#") or line.begins_with(" ") or not " = " in line:
			continue
		var at := line.find(" = ")
		var value := line.substr(at + 3)
		if not value.ends_with("{") and not value.ends_with("->"):
			parsed[line.substr(0, at)] = value
	return parsed

## Message `key` with `{ $name }` placeholders replaced; the key itself when
## it is unknown, so a gap is visible instead of silent.
func tr_key(key: String, args := {}) -> String:
	var text: String = messages.get(key, key)
	for name in args:
		text = text.replace("{ $%s }" % name, str(args[name]))
	return text
