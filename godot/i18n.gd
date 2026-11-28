# SPDX-License-Identifier: EUPL-1.2
extends RefCounted
## Rust Fluent formats the shared embedded resources, including multiline
## coach messages and numeric/string selectors.

var language := "de"
var formatter := SkatApp.new()

func load_language(lang: String) -> bool:
	if lang not in ["de", "en"]:
		return false
	language = lang
	return true

func tr_key(key: String, args := {}) -> String:
	var result = JSON.parse_string(formatter.translate(language, key, JSON.stringify(args)))
	if result is Dictionary and result.has("Ok"):
		return result["Ok"]
	push_error("Fluent: %s" % str(result))
	return key
