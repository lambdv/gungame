extends Node3D

const PLAYER_SCENE = preload("res://entites/player/Player.tscn")
const PLAYER_SPAWN_POSITION = Vector3(0, 1.62212, -2.21878)

@onready var local_player: CharacterBody3D = null
@onready var remote_players: Dictionary = {}  # player_id -> player_instance
@onready var server_dummy: Node3D = null
@onready var connection_timer: Timer = null

func _ready() -> void:
	# Connect to networking signals
	NetworkingManager.lobby_joined.connect(_on_lobby_joined)
	NetworkingManager.lobby_join_failed.connect(_on_lobby_join_failed)
	NetworkingManager.player_joined.connect(_on_player_joined)
	NetworkingManager.player_left.connect(_on_player_left)
	NetworkingManager.position_update_received.connect(_on_position_update_received)
	NetworkingManager.server_dummy_updated.connect(_on_server_dummy_updated)

	# Connect to test lobby
	NetworkingManager.connect_to_test_lobby()

	# Set up connection timeout timer
	connection_timer = Timer.new()
	connection_timer.wait_time = 10.0  # 10 seconds timeout
	connection_timer.one_shot = true
	connection_timer.timeout.connect(_on_connection_timeout)
	add_child(connection_timer)
	connection_timer.start()

func _on_lobby_joined(lobby_data: Dictionary) -> void:
	# Stop connection timer
	if connection_timer:
		connection_timer.stop()

	# Spawn local player
	spawn_local_player()

	# Spawn server dummy
	spawn_server_dummy()

	# Spawn existing remote players (excluding ourselves)
	for player_data in lobby_data.get("players", []):
		var player_id = player_data.get("id", -1)
		if player_id != NetworkingManager.player_id:
			spawn_remote_player(player_id, player_data)

func _on_lobby_join_failed(error: String) -> void:
	print("Failed to join lobby, will try to create: ", error)

	# The networking manager will automatically try to create the lobby
	# when join fails, so we don't need to do anything here

func _on_player_joined(player_data: Dictionary) -> void:
	var player_id = player_data.get("id", -1)
	if player_id == NetworkingManager.player_id:
		# This is us, already spawned
		return

	print("Remote player joined: ", player_id)
	spawn_remote_player(player_id, player_data)

func _on_player_left(player_id: int) -> void:
	print("Remote player left: ", player_id)
	if player_id in remote_players:
		var player_instance = remote_players[player_id]
		remote_players.erase(player_id)
		if is_instance_valid(player_instance):
			player_instance.queue_free()

func _on_position_update_received(player_id: int, position: Vector3, rotation: Vector3) -> void:
	# Don't update our own position (we control it locally)
	if player_id == NetworkingManager.player_id:
		return

	# If player doesn't exist yet, spawn them
	if player_id not in remote_players:
		spawn_remote_player(player_id, {"id": player_id, "name": "Player" + str(player_id)})

	if player_id in remote_players:
		var player_instance = remote_players[player_id]
		if is_instance_valid(player_instance):
			player_instance.position = position

func cleanup_server_dummy() -> void:
	if server_dummy and is_instance_valid(server_dummy):
		server_dummy.queue_free()
		server_dummy = null

func spawn_local_player() -> void:
	var player_instance = PLAYER_SCENE.instantiate()
	if not player_instance:
		push_error("Failed to instantiate player scene!")
		return

	player_instance.position = PLAYER_SPAWN_POSITION
	player_instance.name = "LocalPlayer"
	add_child(player_instance)
	local_player = player_instance

	# Set camera as current
	var camera = player_instance.get_node_or_null("CameraRig/Head/Camera3D")
	if camera:
		camera.current = true
	else:
		push_error("Failed to find camera in player scene!")

	# Set as local player in multiplayer manager if available
	if MultiplayerManager:
		MultiplayerManager.set_local_player(player_instance)

func spawn_remote_player(player_id: int, player_data: Dictionary) -> void:
	if player_id in remote_players:
		return

	var player_instance = PLAYER_SCENE.instantiate()
	player_instance.position = PLAYER_SPAWN_POSITION + Vector3(randf_range(-2, 2), 0, randf_range(-2, 2))
	player_instance.name = "RemotePlayer_" + str(player_id)
	add_child(player_instance)

	# Make it non-local so it shows health bar and doesn't respond to input
	if player_instance.has_method("set_is_local"):
		player_instance.set_is_local(false)

	remote_players[player_id] = player_instance

func spawn_server_dummy() -> void:
	server_dummy = Node3D.new()
	server_dummy.name = "ServerDummy"

	# Add a visual representation (simple cube for now)
	var mesh_instance = MeshInstance3D.new()
	var box_mesh = BoxMesh.new()
	box_mesh.size = Vector3(1, 2, 1)
	mesh_instance.mesh = box_mesh

	var material = StandardMaterial3D.new()
	material.albedo_color = Color(1, 0, 1)  # Magenta color to distinguish from players
	mesh_instance.material_override = material

	server_dummy.add_child(mesh_instance)
	add_child(server_dummy)

func _on_server_dummy_updated(position: Vector3) -> void:
	if server_dummy:
		server_dummy.position = position

func _on_connection_timeout() -> void:
	print("Connection timeout - failed to connect to server after 10 seconds")
	push_error("Failed to connect to game server. Make sure the server is running on localhost:8080")

	# Create a simple error message on screen
	var error_label = Label.new()
	error_label.text = "Failed to connect to server!\nMake sure the server is running on localhost:8080\nCheck console for details."
	error_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	error_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	error_label.position = Vector2(400, 300)
	error_label.size = Vector2(400, 100)
	add_child(error_label)

# Position sync for local player - throttle updates to 10 per second
var position_update_timer: float = 0.0
const POSITION_UPDATE_INTERVAL: float = 0.1  # 10 updates per second

func _process(delta: float) -> void:
	if local_player and NetworkingManager.is_connected_to_lobby():
		position_update_timer += delta
		if position_update_timer >= POSITION_UPDATE_INTERVAL:
			position_update_timer = 0.0
			# Send position updates to server
			var pos = local_player.position
			var rot = local_player.rotation
			NetworkingManager.send_position_update(pos, rot)
