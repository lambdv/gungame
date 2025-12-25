extends Control

@onready var lobby_container: VBoxContainer = $VBoxContainer/LobbyListContainer/ScrollContainer/LobbyContainer
@onready var no_lobbies_label: Label = $VBoxContainer/LobbyListContainer/NoLobbiesLabel
@onready var create_button: Button = $VBoxContainer/Header/ActionButtons/CreateButton
@onready var random_button: Button = $VBoxContainer/Header/ActionButtons/RandomButton
@onready var refresh_button: Button = $VBoxContainer/Header/ActionButtons/RefreshButton

var lobby_item_scene: PackedScene = preload("res://ui/menus/lobbies/lobby_item.tscn")
var _is_joining_random: bool = false
var _current_status: String = ""

func _ready() -> void:
	# Release mouse capture for UI interaction
	if InputManager:
		InputManager.release_mouse()
	
	# Connect to signals from NetworkingManager
	NetworkingManager.lobby_list_received.connect(_on_lobby_list_received)
	NetworkingManager.lobby_created.connect(_on_lobby_created)
	NetworkingManager.lobby_joined.connect(_on_lobby_joined)
	NetworkingManager.lobby_join_failed.connect(_on_lobby_join_failed)

	create_button.connect("pressed", Callable(self, "_on_create_pressed"))
	random_button.connect("pressed", Callable(self, "_on_random_pressed"))
	refresh_button.connect("pressed", Callable(self, "_on_refresh_pressed"))

	# Load initial lobby list
	_on_refresh_pressed()

func _on_refresh_pressed() -> void:
	NetworkingManager.get_lobby_list()
	refresh_button.disabled = true
	refresh_button.text = "🔄 REFRESHING..."
	_set_buttons_enabled(false)

func _on_lobby_list_received(lobby_list: Array) -> void:
	print("Received lobby list with ", lobby_list.size(), " lobbies")
	refresh_button.disabled = false
	refresh_button.text = "🔄 REFRESH"
	_set_buttons_enabled(true)
	_set_status("")  # Clear status when receiving lobby list

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
	if not lobby_list.is_empty():
		no_lobbies_label.text = "No lobbies available. Create one to get started!"
	else:
		no_lobbies_label.text = _current_status if _current_status != "" else "No lobbies available. Create one to get started!"

	print(lobby_list)
	# Create lobby items
	for lobby_data in lobby_list:
		print("Creating lobby item for lobby data: ", lobby_data)

		# Extract data safely
		var lobby_code = lobby_data.get("code", "unknown") if lobby_data is Dictionary else "unknown"
		var player_count = lobby_data.get("player_count", 0) if lobby_data is Dictionary else 0
		var max_players = lobby_data.get("max_players", 4) if lobby_data is Dictionary else 4
		var scene = lobby_data.get("scene", "world") if lobby_data is Dictionary else "world"

		print("Extracted lobby data - code: ", lobby_code, ", players: ", player_count, "/", max_players)

		var lobby_item = lobby_item_scene.instantiate()
		var setup_data = {
			"code": lobby_code,
			"player_count": player_count,
			"max_players": max_players,
			"scene": scene
		}

		lobby_item.setup(setup_data)
		lobby_item.join_pressed.connect(_on_join_lobby_pressed)
		lobby_container.add_child(lobby_item)

func _on_create_pressed() -> void:
	var random_code = _generate_random_code()
	NetworkingManager.create_lobby(random_code)
	create_button.disabled = true
	create_button.text = "⚡ CREATING..."

func _on_random_pressed() -> void:
	random_button.disabled = true
	random_button.text = "🎲 SEARCHING..."

	# Get current lobby list and join a random one
	_is_joining_random = true
	NetworkingManager.get_lobby_list()

func _on_join_lobby_pressed(lobby_code: String) -> void:
	print("=== LOBBY LIST RECEIVED JOIN SIGNAL ===")
	print("Join lobby pressed for code: ", lobby_code)
	print("Networking manager available: ", NetworkingManager != null)
	if NetworkingManager:
		print("Setting status and calling join_lobby...")
		_set_status("Joining lobby " + lobby_code + "...")
		NetworkingManager.join_lobby(lobby_code)
		_set_buttons_enabled(false)
		print("Join lobby called successfully")
	else:
		print("ERROR: Networking manager is null!")
		_set_status("Error: Networking not available")

func _on_lobby_created(lobby_data: Dictionary) -> void:
	create_button.disabled = false
	create_button.text = "⚡ CREATE LOBBY"
	# Automatically join the lobby we just created
	NetworkingManager.join_lobby(lobby_data["code"])

func _on_lobby_joined(lobby_data: Dictionary) -> void:
	_set_buttons_enabled(true)
	_set_status("")  # Clear status on successful join
	_load_lobby_scene(lobby_data)

func _on_lobby_join_failed(error: String, lobby_code: String) -> void:
	_set_buttons_enabled(true)
	random_button.disabled = false
	random_button.text = "🎲 JOIN RANDOM"
	print("Failed to join lobby: ", error, " (code: ", lobby_code, ")")
	_set_status("Failed to join lobby " + lobby_code + ": " + error)
	push_error("Failed to join lobby: " + str(error) + " - " + str(lobby_code))

func _load_lobby_scene(lobby_data: Dictionary) -> void:
	var scene_name = lobby_data.get("scene", "world")
	var scene_path = "res://test/world/World.tscn"

	print("=== LOADING LOBBY SCENE ===")
	print("Loading lobby scene: ", scene_name, " -> ", scene_path)
	print("Lobby data: ", lobby_data)
	match scene_name:
		"world":
			scene_path = "res://test/world/World.tscn"

	print("Changing scene to: ", scene_path)
	var result = get_tree().change_scene_to_file(scene_path)
	print("Scene change result: ", result)
	if result != OK:
		print("ERROR: Failed to change scene! Error code: ", result)

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
	_set_status("Joining random lobby " + random_lobby["code"] + "...")
	NetworkingManager.join_lobby(random_lobby["code"])

func _set_status(message: String) -> void:
	_current_status = message
	if lobby_container.get_child_count() == 0:
		# If no lobby items, show status in the no lobbies label
		no_lobbies_label.text = message if message != "" else "No lobbies available. Create one to get started!"
		no_lobbies_label.visible = true
	else:
		# If there are lobby items, hide the label (or show status elsewhere)
		no_lobbies_label.visible = _current_status != "" and lobby_container.get_child_count() == 0
		if _current_status != "":
			no_lobbies_label.text = _current_status
