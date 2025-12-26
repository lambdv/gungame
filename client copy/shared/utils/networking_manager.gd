extends Node
## NetworkingManager - Unified Client Networking System
##
## Handles all client-side networking for GunGame:
## - HTTP REST API for lobby management (create/join lobbies)
## - UDP real-time game synchronization (position updates, player events)
## - Connection state management and signal broadcasting
##
## This consolidates what was previously split across multiple managers
## for a cleaner, more maintainable networking architecture.

# Server endpoints - configure these for production deployment
const SERVER_URL = "http://127.0.0.1:8080"  # HTTP API server for lobbies
const UDP_PORT = 8081  # UDP port for real-time game communication

# HTTP client setup - handles REST API calls to server
var http_request: HTTPRequest  # Godot's HTTP request node
var current_request_id: int = 0  # Unique ID for tracking concurrent requests
var pending_requests: Dictionary = {}  # Maps request IDs to their metadata

# UDP client setup - handles real-time game communication
var udp_peer: PacketPeerUDP  # Godot's UDP networking peer
var connected_to_udp: bool = false  # Whether UDP connection is established

# Lobby state - tracks current multiplayer session
var current_lobby: Dictionary = {}  # Lobby info (code, players, server_ip, etc.)
var player_id: int = -1  # This client's assigned player ID from server
var connected_players: Dictionary = {}  # Other players in current lobby
var _last_joined_lobby_code: String = ""

# Connection state tracking
var connection_state: ConnectionState = ConnectionState.DISCONNECTED
enum ConnectionState {
	DISCONNECTED,    # Not connected to any server
	CONNECTED_HTTP,  # Connected to HTTP server, no active lobby
	CONNECTED_LOBBY, # In active lobby with UDP connection
	RECONNECTING     # Attempting to reconnect
}

# Reconnection settings
var max_reconnect_attempts: int = 3
var current_reconnect_attempt: int = 0
var reconnect_timer: Timer = null

# Connection monitoring
var connection_monitor_timer: Timer = null
var last_udp_activity: float = 0.0
var connection_timeout: float = 30.0  # 30 seconds without UDP activity

# Signals - Event broadcasting for UI and game systems to react to network events
signal lobby_created(lobby_data: Dictionary)  # Fired when new lobby is successfully created
signal lobby_joined(lobby_data: Dictionary)  # Fired when successfully joined existing lobby
signal lobby_join_failed(error: String, lobby_code: String)  # Fired when lobby join/create fails
signal lobby_left()  # Fired when successfully left current lobby
signal lobby_list_received(lobby_list: Array)  # Fired when lobby list is retrieved from server
signal player_joined(player_data: Dictionary)  # Fired when another player joins the lobby
signal player_left(player_id: int)  # Fired when a player disconnects from lobby
signal position_update_received(player_id: int, position: Vector3, rotation: Vector3)  # Real-time position sync
signal server_dummy_updated(position: Vector3)  # Server-controlled bot position updates
signal connection_confirmed()  # Fired when UDP connection to game server is confirmed
signal connection_state_changed(new_state: ConnectionState)  # Fired when connection state changes
signal reconnection_attempt(attempt: int, max_attempts: int)  # Fired during reconnection attempts
signal state_sync_received(player_states: Array)  # Fired when full state sync is received from server
signal weapon_switched(player_id: int, weapon_id: int)  # Fired when a player switches weapons
signal player_damaged(player_id: int, damage: int, attacker_id: int)  # Fired when a player takes damage

func _ready() -> void:
	_setup_http_client()
	_setup_udp_client()
	_setup_connection_monitoring()

func _setup_http_client() -> void:
	http_request = HTTPRequest.new()
	add_child(http_request)
	http_request.request_completed.connect(_on_http_request_completed)
	# Configure HTTP request
	http_request.use_threads = false
	http_request.timeout = 10.0

func _setup_udp_client() -> void:
	udp_peer = PacketPeerUDP.new()

func _setup_connection_monitoring() -> void:
	connection_monitor_timer = Timer.new()
	connection_monitor_timer.wait_time = 5.0  # Check connection every 5 seconds
	connection_monitor_timer.timeout.connect(_check_connection_health)
	add_child(connection_monitor_timer)
	connection_monitor_timer.start()

func _exit_tree() -> void:
	if udp_peer:
		udp_peer.close()

# HTTP API Methods - Lobby management via REST API

## create_lobby
## Creates a new multiplayer lobby on the server
## @param code: Unique lobby identifier (must be unique across server)
## @param scene: Scene/world name for this lobby (default: "world")
## @param max_players: Maximum number of players allowed (default: 4)
func create_lobby(code: String, scene: String = "world", max_players: int = 4) -> void:
	var url = SERVER_URL + "/lobbies"
	var headers = ["Content-Type: application/json"]
	var body = JSON.stringify({
		"code": code,
		"scene": scene,
		"max_players": max_players
	})

	_make_request(url, headers, HTTPClient.METHOD_POST, body, "create_lobby")

## join_lobby
## Joins an existing lobby and establishes UDP game connection
## @param code: Lobby code to join
func join_lobby(code: String) -> void:
	print("=== NETWORKING MANAGER JOIN LOBBY ===")
	print("Joining lobby with code: ", code)

	# If already in a lobby, leave it first
	if connection_state == ConnectionState.CONNECTED_LOBBY:
		print("Already in a lobby, leaving first...")
		leave_current_lobby()

	_last_joined_lobby_code = code
	var url = SERVER_URL + "/lobbies/" + code + "/join"
	var headers = ["Content-Type: application/json"]
	# Generate unique player name based on system time to avoid conflicts
	var player_name = "Player_" + str(Time.get_ticks_msec() % 10000)
	var body = JSON.stringify({
		"player_name": player_name
	})
	print("Request URL: ", url)
	print("Request body: ", body)

	var request_id = _make_request(url, headers, HTTPClient.METHOD_POST, body, "join_lobby")
	print("Request made with ID: ", request_id)

func get_lobby_info(code: String) -> void:
	var url = SERVER_URL + "/lobbies/" + code
	_make_request(url, [], HTTPClient.METHOD_GET, "", "get_lobby_info")

func get_lobby_list() -> void:
	var url = SERVER_URL + "/lobbies"
	_make_request(url, [], HTTPClient.METHOD_GET, "", "get_lobby_list")

func _make_request(url: String, headers: Array, method: int, body: String, request_type: String) -> int:
	var error = http_request.request(url, headers, method, body)
	if error != OK:
		push_error("HTTP Request failed: " + str(error))
		return -1

	current_request_id += 1
	pending_requests[current_request_id] = {
		"type": request_type,
		"url": url,
		"timestamp": Time.get_ticks_msec()
	}
	return current_request_id

## _on_http_request_completed
## Handles HTTP responses from the server
## Uses a simple request tracking system since HTTPRequest processes requests sequentially
func _on_http_request_completed(_result: int, response_code: int, _headers: Array, body: PackedByteArray) -> void:
	# Parse JSON response from server
	var response_text = body.get_string_from_utf8()
	var json = JSON.new()
	var parse_result = json.parse(response_text)

	var response_data = {}
	if parse_result == OK:
		response_data = json.data
	else:
		push_error("Failed to parse JSON response: " + response_text)
		return

	# Find the request that matches this response
	# HTTPRequest doesn't provide request IDs, so we use a simple heuristic
	var request_type = ""
	var matched_request_id = -1

	if not pending_requests.is_empty():
		# For now, use the oldest pending request (simplified approach)
		# In production, you'd want to match based on URL/timing
		var keys = pending_requests.keys()
		if keys.size() > 0:
			keys.sort()  # Sort by request ID (which is timestamp-based)
			matched_request_id = keys[0]
			var request_info = pending_requests[matched_request_id]
			request_type = request_info["type"]
			pending_requests.erase(matched_request_id)
			print("HTTP: Matched response to request type: ", request_type)

	# Route response to appropriate handler based on request type
	match request_type:
		"create_lobby":
			_handle_create_lobby_response(response_code, response_data)
		"join_lobby":
			_handle_join_lobby_response(response_code, response_data)
		"get_lobby_info":
			_handle_get_lobby_info_response(response_code, response_data)
		"get_lobby_list":
			_handle_get_lobby_list_response(response_code, response_data)
		"try_connect_test_lobby":
			_handle_try_connect_test_lobby_response(response_code, response_data)

func _handle_create_lobby_response(response_code: int, data: Dictionary) -> void:
	if response_code == 200:  # Axum returns 200 for successful creation
		current_lobby = data
		lobby_created.emit(data)
		print("Lobby created successfully: ", data.get("code", "unknown"))
	else:
		push_error("Failed to create lobby: " + str(response_code) + " - " + str(data))

func _handle_join_lobby_response(response_code: int, data: Dictionary) -> void:
	print("=== JOIN LOBBY RESPONSE ===")
	print("Join lobby response - Code:", response_code, " Data:", data)
	if response_code == 200:
		print("Join successful!")
		current_lobby = data.get("lobby", {})
		player_id = data.get("player_id", -1)

		print("Parsed lobby data - code:", current_lobby.get("code", "none"), " player_id:", player_id)

		# Update connection state to HTTP connected (before UDP)
		_set_connection_state(ConnectionState.CONNECTED_HTTP)

		# Connect to UDP server
		var server_ip = current_lobby.get("server_ip", "127.0.0.1")
		var udp_port = current_lobby.get("udp_port", UDP_PORT)
		print("Connecting to UDP:", server_ip, ":", udp_port)
		connect_udp_server(server_ip, udp_port)

		# Set up a fallback timer in case UDP connection fails
		var udp_fallback_timer = Timer.new()
		udp_fallback_timer.wait_time = 5.0  # 5 seconds fallback
		udp_fallback_timer.one_shot = true
		udp_fallback_timer.timeout.connect(_on_udp_connection_timeout.bind(current_lobby.duplicate(true)))
		add_child(udp_fallback_timer)
		udp_fallback_timer.start()

		# Don't emit lobby_joined yet - wait for UDP connection confirmation
		print("UDP connection initiated, waiting for confirmation...")
	else:
		print("Join failed with code:", response_code)
		lobby_join_failed.emit("Failed to join lobby: " + str(response_code), _last_joined_lobby_code)
		push_error("Failed to join lobby: " + str(response_code) + " - " + str(data))

func _handle_get_lobby_info_response(response_code: int, data: Dictionary) -> void:
	if response_code == 200:
		current_lobby = data
		print("Lobby info retrieved: ", data.get("code", "unknown"))
	else:
		push_error("Failed to get lobby info: " + str(response_code) + " - " + str(data))

func _handle_get_lobby_list_response(response_code: int, data: Array) -> void:
	print("=== LOBBY LIST RESPONSE ===")
	print("Response code: ", response_code)
	print("Data type: ", typeof(data))
	print("Data size: ", data.size())

	if response_code == 200:
		lobby_list_received.emit(data)
		print("Lobby list retrieved: ", data.size(), " lobbies")
	else:
		push_error("Failed to get lobby list: HTTP " + str(response_code) + " - " + str(data))

func _handle_try_connect_test_lobby_response(response_code: int, _data: Dictionary) -> void:
	if response_code == 200:
		# Lobby exists, try to join it
		print("Test lobby exists, attempting to join...")
		join_lobby("test")
	else:
		# Lobby doesn't exist, create it
		print("Test lobby doesn't exist, creating it...")
		create_lobby("test")

# UDP Methods - Real-time game communication

## connect_udp_server
## Establishes UDP connection to game server for real-time communication
## Called automatically after successfully joining a lobby via HTTP
## @param ip: Server IP address
## @param port: UDP port number
func connect_udp_server(ip: String, port: int) -> void:
	print("=== CONNECTING UDP TO SERVER ===")
	print("Attempting to connect UDP to " + ip + ":" + str(port))
	if udp_peer.connect_to_host(ip, port) != OK:
		push_error("Failed to connect UDP to " + ip + ":" + str(port))
		return

	connected_to_udp = true
	last_udp_activity = Time.get_ticks_msec() / 1000.0  # Initialize activity timer
	print("Connected to UDP server at " + ip + ":" + str(port))

	# Immediately announce our presence to the server
	print("Sending join packet to server...")
	send_join_packet()

func send_join_packet() -> void:
	print("=== SENDING UDP JOIN PACKET ===")
	if not connected_to_udp:
		print("Cannot send join packet - UDP not connected")
		return
	if current_lobby.is_empty():
		print("Cannot send join packet - no lobby data")
		return

	var packet = {
		"type": "join",
		"lobby_code": current_lobby.get("code", ""),
		"player_id": player_id
	}

	print("Sending join packet to server - lobby: ", current_lobby.get("code", ""), " player_id: ", player_id)
	print("Lobby data:", current_lobby)
	_send_udp_packet(packet)
	print("Join packet sent!")

## send_position_update
## Sends this player's position and rotation to server for broadcasting to other players
## Called continuously during gameplay (typically 10 times per second)
## @param position: Player's 3D position
## @param rotation: Player's 3D rotation (for camera/look direction)
func send_position_update(position: Vector3, rotation: Vector3) -> void:
	if not connected_to_udp:
		return

	var packet = {
		"type": "position_update",
		"player_id": player_id,
		"position": {
			"x": position.x,
			"y": position.y,
			"z": position.z
		},
		"rotation": {
			"x": rotation.x,
			"y": rotation.y,
			"z": rotation.z
		}
	}

	_send_udp_packet(packet)

func send_weapon_switch(weapon_id: int) -> void:
	if not connected_to_udp:
		return

	var packet = {
		"type": "weapon_switch",
		"lobby_code": current_lobby.get("code", ""),
		"player_id": player_id,
		"weapon_id": weapon_id
	}

	_send_udp_packet(packet)

func _send_udp_packet(data: Dictionary) -> void:
	if not connected_to_udp:
		return

	var json_string = JSON.stringify(data)
	if json_string.is_empty():
		push_error("UDP: Failed to stringify packet: " + str(data))
		return

	var packet = json_string.to_utf8_buffer()
	var error = udp_peer.put_packet(packet)
	if error != OK:
		push_error("UDP: Failed to send packet '" + data.get("type", "unknown") + "': " + str(error))
	# Removed success logging to reduce spam

func _process(_delta: float) -> void:
	if connected_to_udp:
		# Process incoming UDP packets (limit to 10 per frame to prevent flooding)
		var packets_processed = 0
		const MAX_PACKETS_PER_FRAME = 10
		while udp_peer.get_available_packet_count() > 0 and packets_processed < MAX_PACKETS_PER_FRAME:
			var packet = udp_peer.get_packet()
			var packet_string = packet.get_string_from_utf8()

			if packet_string.is_empty():
				push_error("UDP: Received empty packet")
				continue

			var json = JSON.new()
			var parse_result = json.parse(packet_string)

			if parse_result == OK:
				last_udp_activity = Time.get_ticks_msec() / 1000.0
				var packet_type = json.data.get("type", "unknown")
				# Reduce spam for frequent packet types - only log important events
				if packet_type not in ["server_dummy_update", "state_sync", "position_update"]:
					print("UDP: Received packet '" + packet_type + "'")
				_process_udp_packet(json.data)
				packets_processed += 1
			else:
				push_error("UDP: Failed to parse packet: '" + packet_string + "' (error: " + str(parse_result) + ")")
				packets_processed += 1

		# Warn if we hit the packet limit
		if packets_processed >= MAX_PACKETS_PER_FRAME:
			print("UDP: Processed maximum packets per frame (" + str(MAX_PACKETS_PER_FRAME) + "), deferring remaining packets")

## _process_udp_packet
## Routes incoming UDP packets to appropriate handlers
## Server sends various message types for game state synchronization
func _process_udp_packet(data: Dictionary) -> void:
	var packet_type = data.get("type", "")

	match packet_type:
		"welcome":
			# Server acknowledges our join request - UDP connection is fully established
			print("=== UDP CONNECTION CONFIRMED ===")
			print("Received welcome from server - connection confirmed!")
			last_udp_activity = Time.get_ticks_msec() / 1000.0  # Mark initial activity
			_set_connection_state(ConnectionState.CONNECTED_LOBBY)
			reset_reconnection_state()  # Reset reconnection attempts on successful connection

			# Cancel any fallback timer that might be running
			for child in get_children():
				if child is Timer and child.wait_time == 5.0 and child.one_shot:
					child.stop()
					child.queue_free()
					print("Cancelled UDP fallback timer")
					break

			# Now emit the lobby_joined signal since we're fully connected
			print("Emitting lobby_joined signal after UDP confirmation...")
			print("Current lobby data:", current_lobby)
			lobby_joined.emit(current_lobby)
			print("Emitted lobby_joined signal for lobby: ", current_lobby.get("code", "unknown"))

			connection_confirmed.emit()
			print("UDP connection fully established!")

		"position_update":
			# Another player moved - update their position in our local game world
			var remote_player_id = data.get("player_id", -1)
			var pos_data = data.get("position", {})

			var position = Vector3(
				pos_data.get("x", 0.0),
				pos_data.get("y", 0.0),
				pos_data.get("z", 0.0)
			)

			# Extract rotation data from the packet
			var rot_data = data.get("rotation", {})
			var rotation = Vector3(
				rot_data.get("x", 0.0),
				rot_data.get("y", 0.0),
				rot_data.get("z", 0.0)
			)

			position_update_received.emit(remote_player_id, position, rotation)

		"player_joined":
			# New player joined our lobby - add them to our connected players list
			var player_data = data.get("player", {})
			connected_players[player_data.get("id", -1)] = player_data
			player_joined.emit(player_data)

		"player_left":
			# Player disconnected - remove them from our game world
			var leaving_player_id = data.get("player_id", -1)
			connected_players.erase(leaving_player_id)
			player_left.emit(leaving_player_id)

		"server_dummy_update":
			# Server-controlled bot (dummy player) moved - update its position
			var pos_data = data.get("position", {})
			var position = Vector3(
				pos_data.get("x", 0.0),
				pos_data.get("y", 0.0),
				pos_data.get("z", 0.0)
			)
			server_dummy_updated.emit(position)

		"state_sync":
			# Full state synchronization from server - contains all player states
			var player_states = data.get("players", [])
			state_sync_received.emit(player_states)

		"weapon_switched":
			# A player switched weapons
			var remote_player_id = data.get("player_id", -1)
			var weapon_id = data.get("weapon_id", -1)
			weapon_switched.emit(remote_player_id, weapon_id)

		"player_damaged":
			# A player was damaged
			var damaged_player_id = data.get("player_id", -1)
			var damage_amount = data.get("damage", 0)
			var attacker_id = data.get("attacker_id", -1)
			player_damaged.emit(damaged_player_id, damage_amount, attacker_id)

# Test lobby methods
func connect_to_test_lobby() -> void:
	print("Attempting to connect to test lobby...")
	# Try to get lobby info first (use lowercase "test" to match server)
	_make_request(
		SERVER_URL + "/lobbies/test",
		[],
		HTTPClient.METHOD_GET,
		"",
		"try_connect_test_lobby"
	)

# Utility methods
func is_connected_to_lobby() -> bool:
	return not current_lobby.is_empty() and connected_to_udp

func get_current_lobby_code() -> String:
	return current_lobby.get("code", "")

func get_connected_player_ids() -> Array:
	return connected_players.keys()

func leave_current_lobby() -> void:
	print("=== LEAVING CURRENT LOBBY ===")

	# Send leave packet to server if we're connected
	if connected_to_udp and not current_lobby.is_empty():
		var leave_packet = {
			"type": "leave",
			"lobby_code": current_lobby.get("code", ""),
			"player_id": player_id
		}
		_send_udp_packet(leave_packet)
		print("Sent leave packet to server")

	# Close UDP connection
	if udp_peer:
		udp_peer.close()
		connected_to_udp = false

	# Clear current lobby state but keep HTTP client ready
	current_lobby.clear()
	player_id = -1
	connected_players.clear()

	# Update connection state
	_set_connection_state(ConnectionState.CONNECTED_HTTP)

	lobby_left.emit()
	print("Left current lobby (UDP disconnected, HTTP still available)")

## return_to_lobby_browser
## Leaves the current lobby and returns to lobby browser state
## This is appropriate when going back to the lobby list from a game
func return_to_lobby_browser() -> void:
	if connection_state == ConnectionState.CONNECTED_LOBBY:
		leave_current_lobby()
	else:
		print("Not in a lobby, already in browser state")

func disconnect_from_network() -> void:
	# Fully disconnect from network (closes both UDP and clears all state)
	leave_current_lobby()
	_set_connection_state(ConnectionState.DISCONNECTED)
	print("Fully disconnected from networking")

## _set_connection_state
## Updates connection state and emits signal for state changes
## @param new_state: The new connection state
func _set_connection_state(new_state: ConnectionState) -> void:
	if connection_state != new_state:
		var old_state = connection_state
		connection_state = new_state
		connection_state_changed.emit(new_state)
		print("Connection state changed: ", _connection_state_to_string(old_state), " -> ", _connection_state_to_string(new_state))

## _connection_state_to_string
## Helper method to convert connection state enum to readable string
func _connection_state_to_string(state: ConnectionState) -> String:
	match state:
		ConnectionState.DISCONNECTED:
			return "DISCONNECTED"
		ConnectionState.CONNECTED_HTTP:
			return "CONNECTED_HTTP"
		ConnectionState.CONNECTED_LOBBY:
			return "CONNECTED_LOBBY"
		ConnectionState.RECONNECTING:
			return "RECONNECTING"
	return "UNKNOWN"

## attempt_reconnection
## Attempts to reconnect to the last lobby if we're disconnected
func attempt_reconnection() -> void:
	if connection_state != ConnectionState.DISCONNECTED and connection_state != ConnectionState.CONNECTED_HTTP:
		print("Cannot reconnect - not in appropriate state")
		return

	if _last_joined_lobby_code.is_empty():
		print("Cannot reconnect - no previous lobby code")
		return

	if current_reconnect_attempt >= max_reconnect_attempts:
		print("Max reconnection attempts reached")
		return

	current_reconnect_attempt += 1
	_set_connection_state(ConnectionState.RECONNECTING)
	reconnection_attempt.emit(current_reconnect_attempt, max_reconnect_attempts)

	print("Attempting reconnection to lobby: ", _last_joined_lobby_code, " (attempt ", current_reconnect_attempt, "/", max_reconnect_attempts, ")")

	# Try to rejoin the lobby
	join_lobby(_last_joined_lobby_code)

	# Set up timeout for reconnection attempt
	if reconnect_timer:
		reconnect_timer.stop()
		reconnect_timer.queue_free()

	reconnect_timer = Timer.new()
	add_child(reconnect_timer)
	reconnect_timer.wait_time = 5.0  # 5 second timeout
	reconnect_timer.one_shot = true
	reconnect_timer.timeout.connect(_on_reconnection_timeout)
	reconnect_timer.start()

func _on_reconnection_timeout() -> void:
	print("Reconnection attempt timed out")
	if current_reconnect_attempt < max_reconnect_attempts:
		print("Will retry reconnection...")
		attempt_reconnection()
	else:
		print("Max reconnection attempts reached, giving up")
		_set_connection_state(ConnectionState.CONNECTED_HTTP)
		current_reconnect_attempt = 0

	# Clean up timer
	if reconnect_timer:
		reconnect_timer.queue_free()
		reconnect_timer = null

## reset_reconnection_state
## Resets reconnection attempt counter (called on successful connection)
func reset_reconnection_state() -> void:
	current_reconnect_attempt = 0
	if reconnect_timer:
		reconnect_timer.stop()
		reconnect_timer.queue_free()
		reconnect_timer = null

## _check_connection_health
## Monitors UDP connection health and triggers reconnection if needed
func _check_connection_health() -> void:
	if connection_state != ConnectionState.CONNECTED_LOBBY:
		return

	var current_time = Time.get_ticks_msec() / 1000.0
	var time_since_last_activity = current_time - last_udp_activity

	if time_since_last_activity > connection_timeout:
		print("Connection timeout detected (%.1f seconds since last UDP activity)" % time_since_last_activity)
		_set_connection_state(ConnectionState.DISCONNECTED)
		attempt_reconnection()

## _on_udp_connection_timeout
## Fallback handler when UDP connection fails to establish
func _on_udp_connection_timeout(lobby_data: Dictionary) -> void:
	if connection_state != ConnectionState.CONNECTED_LOBBY:
		print("UDP connection timeout - falling back to HTTP-only mode")
		push_error("UDP connection failed, but you can still see the lobby. Multiplayer features may not work.")

		# Emit lobby_joined anyway so the scene loads
		print("Fallback: Emitting lobby_joined signal without UDP confirmation")
		lobby_joined.emit(lobby_data)

		# Set a degraded connection state
		_set_connection_state(ConnectionState.CONNECTED_HTTP)
