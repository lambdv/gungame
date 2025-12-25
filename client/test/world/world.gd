extends Node3D
## World Test Scene - Multiplayer Testing Environment

# Enable input processing for respawn handling
func _enter_tree() -> void:
	set_process_input(true)
##
## Test scene that demonstrates the complete multiplayer pipeline:
## - Connects to test lobby via NetworkingManager
## - Spawns local player with input controls
## - Spawns remote players as they join
## - Synchronizes positions in real-time
## - Shows server-controlled dummy bot
##
## This scene serves as both a test environment and reference implementation
## for how to integrate the networking system into actual game levels.
##
## Integration Steps for Game Levels:
## 1. Add PLAYER_SCENE and PLAYER_SPAWN_POSITION constants
## 2. Connect to NetworkingManager signals in _ready(): lobby_joined, player_joined, etc.
## 3. Implement spawn_local_player() and spawn_remote_player() functions
## 4. Add position synchronization in _process() with throttling
## 5. Handle player respawning and cleanup in lobby_left signal
## 6. Use GameStateManager for managing game state

const PLAYER_SCENE_PATH = "res://entites/player/Player.tscn"  # Player prefab path for spawning
const PLAYER_SPAWN_POSITION = Vector3(0, 1.62212, -2.21878)  # Where local player starts

# Player instance tracking
@onready var local_player: CharacterBody3D = null  # The player we control
@onready var remote_players: Dictionary = {}  # Network player_id -> CharacterBody3D instance
@onready var server_dummy: Node3D = null  # Server-controlled bot for testing
@onready var connection_timer: Timer = null  # Timeout for connection attempts
@onready var error_label: Label = $ErrorLabel  # UI for connection errors

# Cached player scene resource (loaded at runtime to avoid parse-time dependency issues)
var _player_scene: PackedScene = null

func get_player_scene() -> PackedScene:
	if _player_scene == null:
		_player_scene = load(PLAYER_SCENE_PATH) as PackedScene
		if _player_scene == null:
			push_error("Failed to load player scene from: " + PLAYER_SCENE_PATH)
	return _player_scene

## _ready
## Initialize the test world and start multiplayer connection
func _ready() -> void:
	# Connect to all networking signals to respond to multiplayer events
	NetworkingManager.lobby_joined.connect(_on_lobby_joined)
	NetworkingManager.lobby_left.connect(_on_lobby_left)
	NetworkingManager.lobby_created.connect(_on_lobby_created)
	NetworkingManager.lobby_join_failed.connect(_on_lobby_join_failed)
	NetworkingManager.player_joined.connect(_on_player_joined)
	NetworkingManager.player_left.connect(_on_player_left)
	NetworkingManager.position_update_received.connect(_on_position_update_received)
	NetworkingManager.server_dummy_updated.connect(_on_server_dummy_updated)
	NetworkingManager.connection_confirmed.connect(_on_connection_confirmed)
	NetworkingManager.state_sync_received.connect(_on_state_sync_received)

	# Connect to local player death signal to handle respawning
	# We'll connect this when the player is spawned

	# Check if we're already connected to a lobby (joined from lobby list)
	call_deferred("_check_existing_lobby_connection")

func _check_existing_lobby_connection() -> void:
	if NetworkingManager.is_connected_to_lobby():
		print("World: Already connected to lobby, initializing game...")
		_on_lobby_joined(NetworkingManager.current_lobby)
		# Don't start connection timer since we're already connected
		return
	elif NetworkingManager.connection_state == NetworkingManager.ConnectionState.CONNECTED_LOBBY:
		print("World: In lobby but UDP not confirmed yet, waiting for confirmation...")
		# Start timer but with shorter timeout since we should be close to connected
		connection_timer = Timer.new()
		connection_timer.wait_time = 5.0  # 5 seconds timeout
		connection_timer.one_shot = true
		connection_timer.timeout.connect(_on_connection_timeout)
		add_child(connection_timer)
		connection_timer.start()
	else:
		# Start the multiplayer connection process for testing
		print("World: Not connected to lobby, connecting to test lobby...")
		NetworkingManager.connect_to_test_lobby()

		# Set up connection timeout timer
		connection_timer = Timer.new()
		connection_timer.wait_time = 10.0  # 10 seconds timeout
		connection_timer.one_shot = true
		connection_timer.timeout.connect(_on_connection_timeout)
		add_child(connection_timer)
		connection_timer.start()

func _on_lobby_created(lobby_data: Dictionary) -> void:
	print("World: Lobby created, now joining it...")
	# After creating a lobby, join it
	NetworkingManager.join_lobby(lobby_data.get("code", "TEST"))

func _on_lobby_joined(lobby_data: Dictionary) -> void:
	print("=== WORLD RECEIVED LOBBY_JOINED SIGNAL ===")
	print("World: Received lobby_joined signal!")
	print("Lobby data:", lobby_data)

	# Stop connection timer if it exists
	if connection_timer and connection_timer.is_inside_tree():
		connection_timer.stop()

	# Ensure game state is PLAYING before spawning player
	if GameStateManager:
		GameStateManager.set_state(GameStateManager.GameState.PLAYING)
		print("World: Game state set to PLAYING, can_process_input: ", GameStateManager.can_process_input())

	# Capture mouse for gameplay
	InputManager.capture_mouse()

	# Spawn local player
	spawn_local_player()

	# Spawn server dummy
	spawn_server_dummy()

	# Spawn existing remote players (excluding ourselves)
	for player_data in lobby_data.get("players", []):
		var player_id = player_data.get("id", -1)
		if player_id != NetworkingManager.player_id:
			spawn_remote_player(player_id, player_data)

	print("World: Scene loading complete!")

func _on_lobby_left() -> void:
	print("World: Lobby left - cleaning up game objects")

	# Clean up local player
	if local_player and is_instance_valid(local_player):
		local_player.queue_free()
		local_player = null

	# Clean up remote players
	for player_id in remote_players.keys():
		var player_instance = remote_players[player_id]
		if is_instance_valid(player_instance):
			player_instance.queue_free()
	remote_players.clear()

	# Clean up server dummy
	cleanup_server_dummy()

	# Stop connection timer if running
	if connection_timer and connection_timer.is_inside_tree():
		connection_timer.stop()

	# Reset mouse capture
	InputManager.release_mouse()

	print("World: Cleanup complete")

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
			pass
		# Use smooth interpolation instead of direct position setting
		# rotation data: (body_y_rotation, head_x_rotation, roll)
		if player_instance.has_method("update_target_position"):
			player_instance.update_target_position(position, Vector3(rotation.x, rotation.y, 0.0))
		else:
			# Fallback to direct setting if interpolation method not available
			player_instance.position = position
			player_instance.rotation.y = rotation.x
			player_instance.get_node("CameraRig/Head").rotation.x = rotation.y
			if player_instance.character_model:
				player_instance.character_model.rotation.x = rotation.y

func _on_state_sync_received(player_states: Array) -> void:
	# Apply state sync data to all players in the sync
	for state_data in player_states:
		var player_id = state_data.get("id", -1)

		# Apply state sync to local player for server-authoritative data (weapon, ammo, health)
		if player_id == NetworkingManager.player_id:
			if local_player and is_instance_valid(local_player) and local_player.has_method("apply_state_sync"):
				local_player.apply_state_sync(state_data)
			continue

		# Ensure remote player exists
		if player_id not in remote_players:
			spawn_remote_player(player_id, {"id": player_id, "name": "Player" + str(player_id)})

		# Apply state sync to the player
		if player_id in remote_players:
			var player_instance = remote_players[player_id]
			if is_instance_valid(player_instance) and player_instance.has_method("apply_state_sync"):
				player_instance.apply_state_sync(state_data)

func cleanup_server_dummy() -> void:
	if server_dummy and is_instance_valid(server_dummy):
		server_dummy.queue_free()
		server_dummy = null

func spawn_local_player() -> void:
	# Don't spawn if we already have a local player
	if local_player and is_instance_valid(local_player):
		print("World: Local player already exists, skipping spawn")
		return

	var player_scene = get_player_scene()
	if player_scene == null:
		push_error("Cannot spawn player: player scene failed to load")
		return
	var player_instance = player_scene.instantiate()
	if not player_instance:
		push_error("Failed to instantiate player scene!")
		return

	player_instance.position = PLAYER_SPAWN_POSITION
	player_instance.name = "LocalPlayer"

	# Ensure player is set as local BEFORE adding to scene tree
	# This ensures input connections are set up properly
	if player_instance.has_method("set_is_local"):
		player_instance.set_is_local(true)

	# Connect to networking manager for multiplayer commands
	if player_instance.has_method("set_networking_manager"):
		player_instance.set_networking_manager(NetworkingManager)
	if player_instance.has_method("set_player_id"):
		player_instance.set_player_id(NetworkingManager.player_id)

	add_child(player_instance)
	local_player = player_instance

	# Connect to player death signal for respawning
	if player_instance.has_signal("died"):
		player_instance.died.connect(_on_local_player_died)

	# Set camera as current
	var camera = player_instance.get_node_or_null("CameraRig/Head/Camera3D")
	if camera:
		camera.current = true
	else:
		push_error("Failed to find camera in player scene!")

	print("World: Local player spawned successfully")

	# Local player setup complete - NetworkingManager handles multiplayer coordination

func spawn_remote_player(player_id: int, player_data: Dictionary) -> void:
	if player_id in remote_players:
		return

	var player_scene = get_player_scene()
	if player_scene == null:
		push_error("Cannot spawn player: player scene failed to load")
		return
	var player_instance = player_scene.instantiate()
	var spawn_pos = PLAYER_SPAWN_POSITION + Vector3(randf_range(-2, 2), 0, randf_range(-2, 2))
	player_instance.position = spawn_pos
	player_instance.name = "RemotePlayer_" + str(player_id)
	add_child(player_instance)

	# Make it non-local so it shows health bar and doesn't respond to input
	if player_instance.has_method("set_is_local"):
		player_instance.set_is_local(false)

	# Disable collision for remote players to prevent physics interference
	var collision_shape = player_instance.get_node_or_null("CollisionShape3D")
	if collision_shape:
		collision_shape.disabled = true

	# Initialize interpolation target to spawn position to prevent teleporting from origin
	if player_instance.has_method("update_target_position"):
		player_instance.update_target_position(spawn_pos, Vector3.ZERO)

	# Connect to networking manager for state synchronization
	if player_instance.has_method("set_networking_manager"):
		player_instance.set_networking_manager(NetworkingManager)
	if player_instance.has_method("set_player_id"):
		player_instance.set_player_id(player_id)

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

func _on_connection_confirmed() -> void:
	print("Connection confirmed by server - stopping timeout timer")
	if connection_timer:
		connection_timer.stop()

func _on_local_player_died(attacker: Node) -> void:
	print("World: Local player died!")
	if error_label:
		error_label.visible = true
		error_label.text = "You died! Press R to respawn or leave the game."

	# Disconnect from death signal to prevent multiple connections
	if local_player and local_player.has_signal("died"):
		local_player.died.disconnect(_on_local_player_died)

func _input(event: InputEvent) -> void:
	# Handle respawn input
	if event.is_action_pressed("reload") and local_player and local_player.has_node("Damageable"):
		var damageable = local_player.get_node("Damageable")
		if damageable and not damageable.is_alive():
			print("World: Respawning player...")
			respawn_local_player()

func respawn_local_player() -> void:
	if not local_player:
		spawn_local_player()
		return

	# Reset player health and position
	local_player.position = PLAYER_SPAWN_POSITION
	if local_player.has_node("Damageable"):
		var damageable = local_player.get_node("Damageable")
		if damageable.has_method("heal"):
			damageable.heal(999)  # Full heal

	# Reconnect death signal
	if local_player.has_signal("died"):
		local_player.died.connect(_on_local_player_died)

	# Hide error message
	if error_label:
		error_label.visible = false

	print("World: Player respawned")

func _on_connection_timeout() -> void:
	print("Connection timeout - failed to connect to server after 10 seconds")
	push_error("Failed to connect to game server. Make sure the server is running on localhost:8080")

	# Show the error message on screen
	if error_label:
		error_label.visible = true
	else:
		push_error("Error label not found in scene")

# Position synchronization - send local player position to server
# Throttled to 10 updates per second to balance responsiveness vs bandwidth
var position_update_timer: float = 0.0
const POSITION_UPDATE_INTERVAL: float = 0.2  # 5 updates per second

## _process
## Handle continuous position synchronization when connected to multiplayer
## @param delta: Time elapsed since last frame
func _process(delta: float) -> void:
	# Only send position updates if we have a local player and are connected
	if local_player and NetworkingManager.is_connected_to_lobby():
		position_update_timer += delta

		# Throttle updates to prevent network spam while maintaining responsiveness
		if position_update_timer >= POSITION_UPDATE_INTERVAL:
			position_update_timer = 0.0

			# Send current position and rotation to server for broadcasting
			var pos = local_player.position
			# Send rotation as (horizontal_body, vertical_head, roll) for clarity
			var rot = Vector3(
				local_player.rotation.y,  # Horizontal body rotation (Y-axis)
				local_player.get_node("CameraRig/Head").rotation.x,  # Vertical head rotation (X-axis)
				0.0  # No roll
			)
			NetworkingManager.send_position_update(pos, rot)
