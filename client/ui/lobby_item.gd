extends PanelContainer

@onready var lobby_code_label: Label = $HBoxContainer/InfoContainer/LobbyCode
@onready var player_count_label: Label = $HBoxContainer/InfoContainer/PlayerCount
@onready var scene_label: Label = $HBoxContainer/InfoContainer/Scene
@onready var join_button: Button = $HBoxContainer/JoinButton

signal join_pressed

var lobby_data: Dictionary

func setup(data: Dictionary) -> void:
	lobby_data = data

	lobby_code_label.text = data.get("code", "UNKNOWN")
	var player_count = data.get("player_count", 0)
	var max_players = data.get("max_players", 4)
	player_count_label.text = "%d / %d players" % [player_count, max_players]

	var scene = data.get("scene", "world")
	match scene:
		"world":
			scene_label.text = "World Map"
		_:
			scene_label.text = scene.capitalize()

	# Disable join button if lobby is full
	join_button.disabled = player_count >= max_players
	if join_button.disabled:
		join_button.text = "FULL"
	else:
		join_button.text = "JOIN"

	join_button.connect("pressed", Callable(self, "_on_join_pressed"))

func _on_join_pressed() -> void:
	join_pressed.emit()
