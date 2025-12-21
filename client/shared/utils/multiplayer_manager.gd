extends Node

@onready var MULTIPLAYER_API = get_tree().get_multiplayer()

# Network connection state
var is_connected := false
var is_server := false
var peer_id := 1  # Local peer ID (1 = server, >1 = client)

# Player management
var local_player: CharacterBody3D = null
var remote_players := {}  # Dictionary: peer_id -> player_node

# Signals for network events
signal connected_to_server
signal connection_failed
signal server_disconnected
signal player_connected(peer_id: int)
signal player_disconnected(peer_id: int)
signal local_player_spawned(player: CharacterBody3D)
signal remote_player_spawned(peer_id: int, player: CharacterBody3D)

const DEFAULT_PORT := 7777
const MAX_PLAYERS := 32

func _ready() -> void:
	# Connect to multiplayer peer signals
	MULTIPLAYER_API.peer_connected.connect(_on_peer_connected)
	MULTIPLAYER_API.peer_disconnected.connect(_on_peer_disconnected)
	MULTIPLAYER_API.connected_to_server.connect(_on_connected_to_server)
	MULTIPLAYER_API.connection_failed.connect(_on_connection_failed)
	MULTIPLAYER_API.server_disconnected.connect(_on_server_disconnected)

func start_server(port: int = DEFAULT_PORT, max_players: int = MAX_PLAYERS) -> bool:
	if is_connected:
		push_warning("Already connected. Disconnect first.")
		return false
	
	var peer = ENetMultiplayerPeer.new()
	var error = peer.create_server(port, max_players)
	if error != OK:
		push_error("Failed to create server: " + str(error))
		return false
	
	MULTIPLAYER_API.multiplayer_peer = peer
	is_server = true
	is_connected = true
	peer_id = 1
	print("Server started on port ", port)
	return true

func connect_to_server(address: String = "127.0.0.1", port: int = DEFAULT_PORT) -> bool:
	if is_connected:
		push_warning("Already connected. Disconnect first.")
		return false
	
	var peer = ENetMultiplayerPeer.new()
	var error = peer.create_client(address, port)
	if error != OK:
		push_error("Failed to create client: " + str(error))
		connection_failed.emit()
		return false
	
	multiplayer.multiplayer_peer = peer
	is_server = false
	is_connected = true
	peer_id = peer.get_unique_id()
	print("Connecting to server at ", address, ":", port)
	return true

func disconnect_from_server() -> void:
	if not is_connected:
		return
	
	MULTIPLAYER_API.multiplayer_peer.close()
	MULTIPLAYER_API.multiplayer_peer = null
	is_connected = false
	is_server = false
	peer_id = 1
	local_player = null
	remote_players.clear()
	print("Disconnected from server")

func is_local_player_node(player: CharacterBody3D) -> bool:
	return player == local_player

func set_local_player(player: CharacterBody3D) -> void:
	local_player = player
	local_player_spawned.emit(player)

func add_remote_player(peer_id: int, player: CharacterBody3D) -> void:
	remote_players[peer_id] = player
	remote_player_spawned.emit(peer_id, player)

func remove_remote_player(peer_id: int) -> void:
	if peer_id in remote_players:
		var player = remote_players[peer_id]
		remote_players.erase(peer_id)
		if is_instance_valid(player):
			player.queue_free()

func get_remote_player(peer_id: int) -> CharacterBody3D:
	return remote_players.get(peer_id, null)

# Network event handlers
func _on_peer_connected(peer_id: int) -> void:
	print("Peer connected: ", peer_id)
	player_connected.emit(peer_id)

func _on_peer_disconnected(peer_id: int) -> void:
	print("Peer disconnected: ", peer_id)
	remove_remote_player(peer_id)
	player_disconnected.emit(peer_id)

func _on_connected_to_server() -> void:
	print("Successfully connected to server")
	connected_to_server.emit()

func _on_connection_failed() -> void:
	print("Connection to server failed")
	is_connected = false
	connection_failed.emit()

func _on_server_disconnected() -> void:
	print("Server disconnected")
	is_connected = false
	server_disconnected.emit()
	remote_players.clear()

