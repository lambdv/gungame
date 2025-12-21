extends Window

@onready var lobby_code_input: LineEdit = $VBoxContainer/LobbyCodeInput
@onready var scene_select: OptionButton = $VBoxContainer/SceneSelect
@onready var max_players_select: OptionButton = $VBoxContainer/MaxPlayersSelect
@onready var create_button: Button = $VBoxContainer/HBoxContainer/CreateButton
@onready var cancel_button: Button = $VBoxContainer/HBoxContainer/CancelButton

var networking_manager: Node

func _ready() -> void:
	networking_manager = get_node("/root/NetworkingManager")

	create_button.connect("pressed", Callable(self, "_on_create_pressed"))
	cancel_button.connect("pressed", Callable(self, "_on_cancel_pressed"))

	# Setup scene options
	scene_select.add_item("World", 0)
	# Add more scenes here as they become available

	# Setup max players options
	max_players_select.add_item("2 Players", 2)
	max_players_select.add_item("4 Players", 4)
	max_players_select.add_item("6 Players", 6)
	max_players_select.add_item("8 Players", 8)
	max_players_select.select(1)  # Default to 4 players

func _on_create_pressed() -> void:
	var code = lobby_code_input.text.strip_edges()
	if code.is_empty():
		code = _generate_random_code()

	var scene = scene_select.get_item_text(scene_select.selected)
	var max_players = max_players_select.get_item_id(max_players_select.selected)

	networking_manager.create_lobby(code, scene.to_lower(), max_players)
	hide()

func _on_cancel_pressed() -> void:
	hide()

func _generate_random_code() -> String:
	var chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	var code = ""
	for i in range(6):
		code += chars[randi() % chars.length()]
	return code


