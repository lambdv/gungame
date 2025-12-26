extends Node
## ClientState - Master Game State Manager
##
## Central manager for all client-side game state:
## - Client game state machine (PLAYING, PAUSED, SPECTATING)
## - Lobby state (current lobby, player ID, connected players)
## - Player state machine (IDLE, WALKING, RUNNING, JUMPING, CROUCHING, DEAD)
## - Player management (main player, other players)
## - Controller/Input reference
##
## This is the single source of truth for all game state on the client.

# ============================================================================
# CLIENT GAME STATE MACHINE
# ============================================================================

enum GameState {
	PLAYING,
	PAUSED,
	SPECTATING
}

var current_game_state: GameState = GameState.PLAYING

# Game state signals
signal game_state_changed(new_state: GameState)
signal game_paused
signal game_resumed
signal spectating_started
signal spectating_ended

# ============================================================================
# PLAYER STATE MACHINE
# ============================================================================

enum PlayerState {
	IDLE,
	WALKING,
	RUNNING,
	JUMPING,
	FALLING,
	CROUCHING,
	DEAD,
	RELOADING
}

# Player state tracking - main player and other players
var main_player_state: PlayerState = PlayerState.IDLE
var other_player_states: Dictionary = {}  # player_id -> PlayerState

# Player state signals
signal player_state_changed(player_id: int, new_state: PlayerState)
signal main_player_state_changed(new_state: PlayerState)
signal player_died(player_id: int)
signal player_respawned(player_id: int)

# ============================================================================
# LOBBY STATE
# ============================================================================

var current_lobby: Dictionary = {}
var player_id: int = -1
var connected_players: Dictionary = {}  # player_id -> player_data Dictionary

# Lobby state signals
signal lobby_state_changed()
signal player_joined_lobby(player_id: int, player_data: Dictionary)
signal player_left_lobby(player_id: int)

# ============================================================================
# PLAYER MANAGEMENT
# ============================================================================

var main_player: CharacterBody3D = null  # The local player we control
var other_players: Dictionary = {}  # player_id -> CharacterBody3D instance

# Player management signals
signal main_player_set(player: CharacterBody3D)
signal main_player_cleared()
signal other_player_added(player_id: int, player: CharacterBody3D)
signal other_player_removed(player_id: int)

# ============================================================================
# DEPENDENCIES (References to other systems)
# ============================================================================

var server_repository: Node = null  # ServerRepository instance
var server_callbacks: Node = null   # ServerCallbacks instance
var networking_adaptor: Node = null # NetworkingAdaptor instance
var input_manager: Node = null      # InputManager instance

# ============================================================================
# INITIALIZATION
# ============================================================================

func _ready() -> void:
	_setup_dependencies()
	_setup_signal_connections()
	set_game_state(GameState.PLAYING)

func _setup_dependencies() -> void:
	# Get dependencies from autoload
	server_repository = get_node_or_null("/root/ServerRepository")
	server_callbacks = get_node_or_null("/root/ServerCallbacks")
	networking_adaptor = get_node_or_null("/root/NetworkingAdaptor")
	input_manager = get_node_or_null("/root/InputManager")
	
	if not server_repository:
		push_warning("ClientState: ServerRepository not found")
	if not server_callbacks:
		push_warning("ClientState: ServerCallbacks not found")
	if not networking_adaptor:
		push_warning("ClientState: NetworkingAdaptor not found")
	if not input_manager:
		push_warning("ClientState: InputManager not found")
	
	# Sync initial state from ServerRepository if already connected
	if server_repository:
		call_deferred("_sync_initial_state")

func _setup_signal_connections() -> void:
	# Connect to ServerCallbacks for networking events (only if not already connected)
	if server_callbacks:
		if not server_callbacks.lobby_joined.is_connected(_on_lobby_joined):
			server_callbacks.lobby_joined.connect(_on_lobby_joined)
		if not server_callbacks.lobby_left.is_connected(_on_lobby_left):
			server_callbacks.lobby_left.connect(_on_lobby_left)
		if not server_callbacks.player_joined.is_connected(_on_player_joined):
			server_callbacks.player_joined.connect(_on_player_joined)
		if not server_callbacks.player_left.is_connected(_on_player_left):
			server_callbacks.player_left.connect(_on_player_left)
		if not server_callbacks.connection_state_changed.is_connected(_on_connection_state_changed):
			server_callbacks.connection_state_changed.connect(_on_connection_state_changed)

func _sync_initial_state() -> void:
	# Sync lobby state from ServerRepository if already connected
	if not server_repository:
		return
	
	if server_repository.has_method("is_connected_to_lobby") and server_repository.is_connected_to_lobby():
		# Access current_lobby property directly
		var repo_lobby = {}
		if "current_lobby" in server_repository:
			repo_lobby = server_repository.current_lobby
		
		if not repo_lobby.is_empty():
			set_current_lobby(repo_lobby)
		
		# Get player_id from server_repository property
		var repo_player_id = -1
		if "player_id" in server_repository:
			repo_player_id = server_repository.player_id
		
		if repo_player_id >= 0:
			set_player_id(repo_player_id)
		
		# Sync connected players
		if server_repository.has_method("get_connected_player_ids"):
			var connected_pids = server_repository.get_connected_player_ids()
			if connected_pids:
				for pid in connected_pids:
					if pid != player_id and pid >= 0:
						# Access connected_players property directly
						var connected_players_dict = {}
						if "connected_players" in server_repository:
							connected_players_dict = server_repository.connected_players
						
						if connected_players_dict is Dictionary:
							var player_data = connected_players_dict.get(pid, {})
							if not player_data.is_empty():
								add_connected_player(pid, player_data)

# ============================================================================
# GAME STATE MANAGEMENT
# ============================================================================

func set_game_state(new_state: GameState) -> void:
	if current_game_state == new_state:
		return
	
	var previous_state = current_game_state
	current_game_state = new_state
	game_state_changed.emit(new_state)
	
	# Emit specific state signals
	match new_state:
		GameState.PAUSED:
			if previous_state == GameState.PLAYING:
				game_paused.emit()
		GameState.PLAYING:
			if previous_state == GameState.PAUSED:
				game_resumed.emit()
		GameState.SPECTATING:
			if previous_state != GameState.SPECTATING:
				spectating_started.emit()
		_:
			if previous_state == GameState.SPECTATING:
				spectating_ended.emit()

func pause() -> void:
	if current_game_state == GameState.PLAYING:
		set_game_state(GameState.PAUSED)

func resume() -> void:
	if current_game_state == GameState.PAUSED:
		set_game_state(GameState.PLAYING)

func toggle_pause() -> void:
	if current_game_state == GameState.PAUSED:
		resume()
	elif current_game_state == GameState.PLAYING:
		pause()

func start_spectating() -> void:
	set_game_state(GameState.SPECTATING)

func stop_spectating() -> void:
	if current_game_state == GameState.SPECTATING:
		set_game_state(GameState.PLAYING)

func is_playing() -> bool:
	return current_game_state == GameState.PLAYING

func is_paused() -> bool:
	return current_game_state == GameState.PAUSED

func is_spectating() -> bool:
	return current_game_state == GameState.SPECTATING

func can_process_input() -> bool:
	return current_game_state == GameState.PLAYING

# ============================================================================
# PLAYER STATE MANAGEMENT
# ============================================================================

func set_main_player_state(new_state: PlayerState) -> void:
	if main_player_state == new_state:
		return
	
	var previous_state = main_player_state
	main_player_state = new_state
	main_player_state_changed.emit(new_state)
	
	# Only emit player_state_changed if player_id is valid
	if player_id >= 0:
		player_state_changed.emit(player_id, new_state)
	
	# Handle state-specific logic
	if player_id >= 0:
		if new_state == PlayerState.DEAD and previous_state != PlayerState.DEAD:
			player_died.emit(player_id)
		elif previous_state == PlayerState.DEAD and new_state != PlayerState.DEAD:
			player_respawned.emit(player_id)

func set_other_player_state(player_id: int, new_state: PlayerState) -> void:
	if other_player_states.get(player_id, PlayerState.IDLE) == new_state:
		return
	
	var previous_state = other_player_states.get(player_id, PlayerState.IDLE)
	other_player_states[player_id] = new_state
	player_state_changed.emit(player_id, new_state)
	
	# Handle state-specific logic
	if new_state == PlayerState.DEAD and previous_state != PlayerState.DEAD:
		player_died.emit(player_id)
	elif previous_state == PlayerState.DEAD and new_state != PlayerState.DEAD:
		player_respawned.emit(player_id)

func get_main_player_state() -> PlayerState:
	return main_player_state

func get_other_player_state(player_id: int) -> PlayerState:
	return other_player_states.get(player_id, PlayerState.IDLE)

func update_player_state_from_movement(player_id: int, is_local: bool, velocity: Vector3, is_on_floor: bool, is_crouching: bool) -> void:
	# Safety check: don't update state if player_id is invalid
	if player_id < 0:
		return
	
	var new_state: PlayerState
	
	if is_crouching:
		new_state = PlayerState.CROUCHING
	elif not is_on_floor:
		if velocity.y > 0:
			new_state = PlayerState.JUMPING
		else:
			new_state = PlayerState.FALLING
	elif velocity.length() < 0.1:
		new_state = PlayerState.IDLE
	elif velocity.length() < 5.0:
		new_state = PlayerState.WALKING
	else:
		new_state = PlayerState.RUNNING
	
	if is_local:
		set_main_player_state(new_state)
	else:
		set_other_player_state(player_id, new_state)

# ============================================================================
# LOBBY STATE MANAGEMENT
# ============================================================================

func set_current_lobby(lobby_data: Dictionary) -> void:
	current_lobby = lobby_data
	lobby_state_changed.emit()

func set_player_id(id: int) -> void:
	player_id = id

func get_current_lobby() -> Dictionary:
	return current_lobby

func get_player_id() -> int:
	return player_id

func is_in_lobby() -> bool:
	return not current_lobby.is_empty() and player_id >= 0

func add_connected_player(player_id: int, player_data: Dictionary) -> void:
	connected_players[player_id] = player_data
	player_joined_lobby.emit(player_id, player_data)

func remove_connected_player(player_id: int) -> void:
	connected_players.erase(player_id)
	other_player_states.erase(player_id)
	player_left_lobby.emit(player_id)

func get_connected_players() -> Dictionary:
	return connected_players

func get_connected_player_ids() -> Array:
	return connected_players.keys()

func clear_lobby_state() -> void:
	current_lobby.clear()
	player_id = -1
	connected_players.clear()
	other_player_states.clear()
	lobby_state_changed.emit()

# ============================================================================
# PLAYER INSTANCE MANAGEMENT
# ============================================================================

func set_main_player(player: CharacterBody3D) -> void:
	if main_player == player:
		return
	
	var old_player = main_player
	main_player = player
	
	# Only emit cleared if old player was valid
	if old_player and is_instance_valid(old_player):
		main_player_cleared.emit()
	
	if main_player and is_instance_valid(main_player):
		main_player_set.emit(main_player)
	else:
		# Invalid player reference, clear it
		main_player = null

func get_main_player() -> CharacterBody3D:
	# Check if the instance is still valid (not freed)
	if main_player and not is_instance_valid(main_player):
		# Clean up invalid reference
		main_player = null
		main_player_cleared.emit()
	
	return main_player

func add_other_player(player_id: int, player: CharacterBody3D) -> void:
	# Validate player instance before adding
	if not is_instance_valid(player):
		push_warning("ClientState: Attempted to add invalid player instance for player_id: " + str(player_id))
		return
	
	if player_id in other_players:
		remove_other_player(player_id)
	
	other_players[player_id] = player
	other_player_added.emit(player_id, player)

func remove_other_player(player_id: int) -> void:
	if player_id in other_players:
		other_players.erase(player_id)
		other_player_states.erase(player_id)
		other_player_removed.emit(player_id)

func get_other_player(player_id: int) -> CharacterBody3D:
	if not player_id in other_players:
		return null
	
	var player = other_players[player_id]
	# Check if the instance is still valid (not freed)
	if not is_instance_valid(player):
		# Clean up invalid reference
		other_players.erase(player_id)
		other_player_states.erase(player_id)
		return null
	
	return player

func get_all_other_players() -> Dictionary:
	# Clean up any invalid references before returning
	var valid_players = {}
	var invalid_ids = []
	
	for pid in other_players.keys():
		var player = other_players[pid]
		if is_instance_valid(player):
			valid_players[pid] = player
		else:
			invalid_ids.append(pid)
	
	# Remove invalid references
	for pid in invalid_ids:
		other_players.erase(pid)
		other_player_states.erase(pid)
	
	return valid_players

func clear_all_players() -> void:
	main_player = null
	other_players.clear()
	other_player_states.clear()
	main_player_state = PlayerState.IDLE
	main_player_cleared.emit()

# ============================================================================
# NETWORKING EVENT HANDLERS
# ============================================================================

func _on_lobby_joined(lobby_data: Dictionary) -> void:
	if lobby_data.is_empty():
		push_warning("ClientState: Received empty lobby data")
		return
	
	set_current_lobby(lobby_data)
	# Ensure game state is PLAYING when joining lobby
	set_game_state(GameState.PLAYING)
	
	if server_repository:
		# Get player_id from server_repository (it's a property, not a method)
		var repo_player_id = -1
		if "player_id" in server_repository:
			repo_player_id = server_repository.player_id
		
		if repo_player_id >= 0:
			set_player_id(repo_player_id)
		else:
			# Try to get player_id from lobby_data
			var lobby_player_id = lobby_data.get("player_id", -1)
			if lobby_player_id >= 0:
				set_player_id(lobby_player_id)
		
		# Sync connected players from server repository
		if server_repository.has_method("get_connected_player_ids"):
			var connected_pids = server_repository.get_connected_player_ids()
			if connected_pids:
				for pid in connected_pids:
					if pid != player_id and pid >= 0:
						# Access connected_players property directly
						var connected_players_dict = {}
						if "connected_players" in server_repository:
							connected_players_dict = server_repository.connected_players
						
						if connected_players_dict is Dictionary:
							var player_data = connected_players_dict.get(pid, {})
							if not player_data.is_empty():
								add_connected_player(pid, player_data)

func _on_lobby_left() -> void:
	clear_lobby_state()
	clear_all_players()

func _on_player_joined(player_data: Dictionary) -> void:
	var pid = player_data.get("id", -1)
	if pid >= 0 and pid != player_id:
		add_connected_player(pid, player_data)

func _on_player_left(pid: int) -> void:
	remove_connected_player(pid)
	remove_other_player(pid)

func _on_connection_state_changed(new_state: int) -> void:
	# Sync connection state changes
	if new_state == ServerCallbacks.ConnectionState.DISCONNECTED:
		clear_lobby_state()
		clear_all_players()

# ============================================================================
# UTILITY METHODS
# ============================================================================

func get_controller() -> Node:
	return input_manager

func get_server_repository() -> Node:
	return server_repository

func get_server_callbacks() -> Node:
	return server_callbacks

func get_networking_adaptor() -> Node:
	return networking_adaptor
