extends Control

@onready var lobby_container: VBoxContainer = $VBoxContainer/LobbyListContainer/ScrollContainer/LobbyContainer
@onready var no_lobbies_label: Label = $VBoxContainer/LobbyListContainer/NoLobbiesLabel
@onready var create_button: Button = $VBoxContainer/Header/ActionButtons/CreateButton
@onready var random_button: Button = $VBoxContainer/Header/ActionButtons/RandomButton
@onready var refresh_button: Button = $VBoxContainer/Header/ActionButtons/RefreshButton

var networking_manager: Node
var lobby_item_scene: PackedScene = preload("res://ui/lobby_item.tscn")
var _is_joining_random: bool = false

func _ready() -> void:
	# Release mouse capture for UI interaction
	if InputManager:
		InputManager.release_mouse()
	
	networking_manager = get_node("/root/NetworkingManager")
	networking_manager.lobby_list_received.connect(_on_lobby_list_received)
	networking_manager.lobby_created.connect(_on_lobby_created)
	networking_manager.lobby_joined.connect(_on_lobby_joined)
	networking_manager.lobby_join_failed.connect(_on_lobby_join_failed)

	create_button.connect("pressed", Callable(self, "_on_create_pressed"))
	random_button.connect("pressed", Callable(self, "_on_random_pressed"))
	refresh_button.connect("pressed", Callable(self, "_on_refresh_pressed"))

	# Load initial lobby list
	_on_refresh_pressed()

func _on_refresh_pressed() -> void:
	networking_manager.get_lobby_list()
	refresh_button.disabled = true
	refresh_button.text = "🔄 REFRESHING..."
	_set_buttons_enabled(false)

func _on_lobby_list_received(lobby_list: Array) -> void:
	refresh_button.disabled = false
	refresh_button.text = "🔄 REFRESH"
	_set_buttons_enabled(true)

	# Handle random join if requested
	if _is_joining_random:
		_is_joining_random = false
		_join_random_lobby(lobby_list)
		return

	# Clear existing lobby items
	for child in lobby_container.get_children():
		child.queue_free()

	# Show/hide no lobbies message
	no_lobbies_label.visible = lobby_list.is_empty()

	# Create lobby items
	for lobby_data in lobby_list:
		var lobby_item = lobby_item_scene.instantiate()
		lobby_item.setup(lobby_data)
		lobby_item.join_pressed.connect(_on_join_lobby_pressed.bind(lobby_data["code"]))
		lobby_container.add_child(lobby_item)

func _on_create_pressed() -> void:
	var random_code = _generate_random_code()
	networking_manager.create_lobby(random_code)
	create_button.disabled = true
	create_button.text = "⚡ CREATING..."

func _on_random_pressed() -> void:
	random_button.disabled = true
	random_button.text = "🎲 SEARCHING..."

	# Get current lobby list and join a random one
	_is_joining_random = true
	networking_manager.get_lobby_list()

func _on_join_lobby_pressed(lobby_code: String) -> void:
	print("Join lobby pressed for code: ", lobby_code)
	networking_manager.join_lobby(lobby_code)
	_set_buttons_enabled(false)

func _on_lobby_created(lobby_data: Dictionary) -> void:
	create_button.disabled = false
	create_button.text = "⚡ CREATE LOBBY"
	# Automatically join the lobby we just created
	networking_manager.join_lobby(lobby_data["code"])

func _on_lobby_joined(lobby_data: Dictionary) -> void:
	_set_buttons_enabled(true)
	_load_lobby_scene(lobby_data)

func _on_lobby_join_failed(error: String) -> void:
	_set_buttons_enabled(true)
	random_button.disabled = false
	random_button.text = "🎲 JOIN RANDOM"
	print("Failed to join lobby: ", error)

func _load_lobby_scene(lobby_data: Dictionary) -> void:
	var scene_name = lobby_data.get("scene", "world")
	var scene_path = "res://test/world/World.tscn"

	match scene_name:
		"world":
			scene_path = "res://test/world/World.tscn"

	get_tree().change_scene_to_file(scene_path)

func _set_buttons_enabled(enabled: bool) -> void:
	create_button.disabled = not enabled
	random_button.disabled = not enabled
	refresh_button.disabled = not enabled

func _generate_random_code() -> String:
	var chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	var code = ""
	for i in range(6):
		code += chars[randi() % chars.length()]
	return code

func _join_random_lobby(lobby_list: Array) -> void:
	if lobby_list.is_empty():
		random_button.disabled = false
		random_button.text = "🎲 NO LOBBIES"
		await get_tree().create_timer(2.0).timeout
		random_button.disabled = false
		random_button.text = "🎲 JOIN RANDOM"
		return

	# Filter out full lobbies
	var available_lobbies = []
	for lobby in lobby_list:
		if lobby.get("player_count", 0) < lobby.get("max_players", 4):
			available_lobbies.append(lobby)

	if available_lobbies.is_empty():
		random_button.disabled = false
		random_button.text = "🎲 ALL FULL"
		await get_tree().create_timer(2.0).timeout
		random_button.disabled = false
		random_button.text = "🎲 JOIN RANDOM"
		return

	# Join a random available lobby
	var random_lobby = available_lobbies[randi() % available_lobbies.size()]
	networking_manager.join_lobby(random_lobby["code"])
