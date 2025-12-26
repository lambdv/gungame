extends Node
## ServerRepository - Networking business logic
##
## Contains all networking business logic:
## - Lobby management (create, join, leave)
## - Connection state management
## - Reconnection handling
## - Uses NetworkingAdaptor for low-level networking
## - Uses ServerCallbacks for signal emissions

const SERVER_URL = "http://127.0.0.1:8080"
const UDP_PORT = 8081

# Dependencies
var adaptor: Node  # NetworkingAdaptor instance
var callbacks: Node  # ServerCallbacks instance

# Lobby state
var current_lobby: Dictionary = {}
var player_id: int = -1
var connected_players: Dictionary = {}
var _last_joined_lobby_code: String = ""

# Connection state
var connection_state: int = 0  # ConnectionState enum from callbacks
var connection_timeout: float = 30.0
var last_udp_activity: float = 0.0

# Reconnection settings
var max_reconnect_attempts: int = 3
var current_reconnect_attempt: int = 0
var reconnect_timer: Timer = null

# Connection monitoring
var connection_monitor_timer: Timer = null

# Request type tracking
var request_type_map: Dictionary = {}  # Maps request_id to request_type

func _ready() -> void:
	_setup_dependencies()
	_setup_connection_monitoring()

func _setup_dependencies() -> void:
	# Get adaptor and callbacks from parent or autoload
	adaptor = get_node_or_null("/root/NetworkingAdaptor")
	if not adaptor:
		push_error("NetworkingAdaptor not found!")
	
	callbacks = get_node_or_null("/root/ServerCallbacks")
	if not callbacks:
		push_error("ServerCallbacks not found!")
	
	# Connect to adaptor signals
	if adaptor:
		adaptor.http_response_received.connect(_on_http_response_received)
		adaptor.udp_packet_received.connect(_on_udp_packet_received)

func _setup_connection_monitoring() -> void:
	connection_monitor_timer = Timer.new()
	connection_monitor_timer.wait_time = 5.0
	connection_monitor_timer.timeout.connect(_check_connection_health)
	add_child(connection_monitor_timer)
	connection_monitor_timer.start()
	
	# Setup keepalive timer to send periodic heartbeat messages
	var keepalive_timer = Timer.new()
	keepalive_timer.wait_time = 10.0  # Send keepalive every 10 seconds
	keepalive_timer.timeout.connect(_send_keepalive)
	add_child(keepalive_timer)
	keepalive_timer.start()

func _process(_delta: float) -> void:
	if adaptor:
		adaptor.process_udp_packets()

# HTTP API Methods

func create_lobby(code: String, scene: String = "world", max_players: int = 4) -> void:
	var url = SERVER_URL + "/lobbies"
	var headers = ["Content-Type: application/json"]
	var body = JSON.stringify({
		"code": code,
		"scene": scene,
		"max_players": max_players
	})
	_make_request(url, headers, HTTPClient.METHOD_POST, body, "create_lobby")

func join_lobby(code: String) -> void:
	print("=== SERVER REPOSITORY JOIN LOBBY ===")
	print("Joining lobby with code: ", code)

	if connection_state == callbacks.ConnectionState.CONNECTED_LOBBY:
		print("Already in a lobby, leaving first...")
		leave_current_lobby()

	_last_joined_lobby_code = code
	var url = SERVER_URL + "/lobbies/" + code + "/join"
	var headers = ["Content-Type: application/json"]
	var player_name = "Player_" + str(Time.get_ticks_msec() % 10000)
	var body = JSON.stringify({
		"player_name": player_name
	})
	print("Request URL: ", url)
	print("Request body: ", body)

	_make_request(url, headers, HTTPClient.METHOD_POST, body, "join_lobby")

func get_lobby_info(code: String) -> void:
	var url = SERVER_URL + "/lobbies/" + code
	_make_request(url, [], HTTPClient.METHOD_GET, "", "get_lobby_info")

func get_lobby_list() -> void:
	var url = SERVER_URL + "/lobbies"
	_make_request(url, [], HTTPClient.METHOD_GET, "", "get_lobby_list")

func _make_request(url: String, headers: Array, method: int, body: String, request_type: String) -> void:
	if not adaptor:
		push_error("Cannot make request - adaptor not available")
		return
	
	var request_id = adaptor.send_http_request(url, headers, method, body)
	if request_id >= 0:
		# Store request type for response handling
		request_type_map[request_id] = request_type

func _on_http_response_received(request_id: int, _result: int, response_code: int, _headers: Array, body: PackedByteArray) -> void:
	var response_text = body.get_string_from_utf8()
	var json = JSON.new()
	var parse_result = json.parse(response_text)

	var response_data = {}
	if parse_result == OK:
		response_data = json.data
	else:
		push_error("Failed to parse JSON response: " + response_text)
		return

	# Get request type from our tracking map
	var request_type = ""
	if request_type_map.has(request_id):
		request_type = request_type_map[request_id]
		request_type_map.erase(request_id)

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
	if response_code == 200:
		current_lobby = data
		callbacks.on_lobby_created(data)
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

		_set_connection_state(callbacks.ConnectionState.CONNECTED_HTTP)

		var server_ip = current_lobby.get("server_ip", "127.0.0.1")
		var udp_port = current_lobby.get("udp_port", UDP_PORT)
		print("Connecting to UDP:", server_ip, ":", udp_port)
		connect_udp_server(server_ip, udp_port)

		var udp_fallback_timer = Timer.new()
		udp_fallback_timer.wait_time = 5.0
		udp_fallback_timer.one_shot = true
		udp_fallback_timer.timeout.connect(_on_udp_connection_timeout.bind(current_lobby.duplicate(true)))
		add_child(udp_fallback_timer)
		udp_fallback_timer.start()

		print("UDP connection initiated, waiting for confirmation...")
	else:
		print("Join failed with code:", response_code)
		callbacks.on_lobby_join_failed("Failed to join lobby: " + str(response_code), _last_joined_lobby_code)
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
		callbacks.on_lobby_list_received(data)
		print("Lobby list retrieved: ", data.size(), " lobbies")
	else:
		push_error("Failed to get lobby list: HTTP " + str(response_code) + " - " + str(data))

func _handle_try_connect_test_lobby_response(response_code: int, _data: Dictionary) -> void:
	if response_code == 200:
		print("Test lobby exists, attempting to join...")
		join_lobby("test")
	else:
		print("Test lobby doesn't exist, creating it...")
		create_lobby("test")

# UDP Methods

func connect_udp_server(ip: String, port: int) -> void:
	print("=== CONNECTING UDP TO SERVER ===")
	print("Attempting to connect UDP to " + ip + ":" + str(port))
	
	if not adaptor:
		push_error("Cannot connect UDP - adaptor not available")
		return
	
	if not adaptor.connect_udp(ip, port):
		return

	last_udp_activity = Time.get_ticks_msec() / 1000.0
	print("Connected to UDP server at " + ip + ":" + str(port))
	send_join_packet()

func send_join_packet() -> void:
	print("=== SENDING UDP JOIN PACKET ===")
	if not adaptor or not adaptor.is_udp_connected():
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
	adaptor.send_udp_packet(packet)
	print("Join packet sent!")

func send_position_update(position: Vector3, rotation: Vector3) -> void:
	if not adaptor or not adaptor.is_udp_connected():
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

	adaptor.send_udp_packet(packet)

func send_weapon_switch(weapon_id: int) -> void:
	if not adaptor or not adaptor.is_udp_connected():
		return

	var packet = {
		"type": "weapon_switch",
		"lobby_code": current_lobby.get("code", ""),
		"player_id": player_id,
		"weapon_id": weapon_id
	}

	adaptor.send_udp_packet(packet)

func _on_udp_packet_received(data: Dictionary) -> void:
	# Update activity timestamp for any UDP packet
	last_udp_activity = Time.get_ticks_msec() / 1000.0
	
	var packet_type = data.get("type", "")

	match packet_type:
		"welcome":
			print("=== UDP CONNECTION CONFIRMED ===")
			print("Received welcome from server - connection confirmed!")
			_set_connection_state(callbacks.ConnectionState.CONNECTED_LOBBY)
			reset_reconnection_state()

			for child in get_children():
				if child is Timer and child.wait_time == 5.0 and child.one_shot:
					child.stop()
					child.queue_free()
					print("Cancelled UDP fallback timer")
					break

			print("Emitting lobby_joined signal after UDP confirmation...")
			callbacks.on_lobby_joined(current_lobby)
			callbacks.on_connection_confirmed()
			print("UDP connection fully established!")

		"position_update":
			var remote_player_id = data.get("player_id", -1)
			var pos_data = data.get("position", {})
			var position = Vector3(
				pos_data.get("x", 0.0),
				pos_data.get("y", 0.0),
				pos_data.get("z", 0.0)
			)
			var rot_data = data.get("rotation", {})
			var rotation = Vector3(
				rot_data.get("x", 0.0),
				rot_data.get("y", 0.0),
				rot_data.get("z", 0.0)
			)
			callbacks.on_position_update_received(remote_player_id, position, rotation)

		"player_joined":
			var player_data = data.get("player", {})
			connected_players[player_data.get("id", -1)] = player_data
			callbacks.on_player_joined(player_data)

		"player_left":
			var leaving_player_id = data.get("player_id", -1)
			connected_players.erase(leaving_player_id)
			callbacks.on_player_left(leaving_player_id)

		"server_dummy_update":
			var pos_data = data.get("position", {})
			var position = Vector3(
				pos_data.get("x", 0.0),
				pos_data.get("y", 0.0),
				pos_data.get("z", 0.0)
			)
			callbacks.on_server_dummy_updated(position)

		"state_sync":
			var player_states = data.get("players", [])
			callbacks.on_state_sync_received(player_states)

		"weapon_switched":
			var remote_player_id = data.get("player_id", -1)
			var weapon_id = data.get("weapon_id", -1)
			callbacks.on_weapon_switched(remote_player_id, weapon_id)

		"player_damaged":
			var damaged_player_id = data.get("player_id", -1)
			var damage_amount = data.get("damage", 0)
			var attacker_id = data.get("attacker_id", -1)
			callbacks.on_player_damaged(damaged_player_id, damage_amount, attacker_id)

# Utility methods

func is_connected_to_lobby() -> bool:
	return not current_lobby.is_empty() and adaptor and adaptor.is_udp_connected()

func get_current_lobby_code() -> String:
	return current_lobby.get("code", "")

func get_connected_player_ids() -> Array:
	return connected_players.keys()

func leave_current_lobby() -> void:
	print("=== LEAVING CURRENT LOBBY ===")

	if adaptor and adaptor.is_udp_connected() and not current_lobby.is_empty():
		var leave_packet = {
			"type": "leave",
			"lobby_code": current_lobby.get("code", ""),
			"player_id": player_id
		}
		adaptor.send_udp_packet(leave_packet)
		print("Sent leave packet to server")

	if adaptor:
		adaptor.close_udp()

	current_lobby.clear()
	player_id = -1
	connected_players.clear()

	_set_connection_state(callbacks.ConnectionState.CONNECTED_HTTP)
	callbacks.on_lobby_left()
	print("Left current lobby (UDP disconnected, HTTP still available)")

func return_to_lobby_browser() -> void:
	if connection_state == callbacks.ConnectionState.CONNECTED_LOBBY:
		leave_current_lobby()
	else:
		print("Not in a lobby, already in browser state")

func disconnect_from_network() -> void:
	leave_current_lobby()
	_set_connection_state(callbacks.ConnectionState.DISCONNECTED)
	print("Fully disconnected from networking")

func connect_to_test_lobby() -> void:
	print("Attempting to connect to test lobby...")
	_make_request(
		SERVER_URL + "/lobbies/test",
		[],
		HTTPClient.METHOD_GET,
		"",
		"try_connect_test_lobby"
	)

# Connection state management

func _set_connection_state(new_state: int) -> void:
	if connection_state != new_state:
		var old_state = connection_state
		connection_state = new_state
		callbacks.on_connection_state_changed(new_state)
		print("Connection state changed: ", _connection_state_to_string(old_state), " -> ", _connection_state_to_string(new_state))

func _connection_state_to_string(state: int) -> String:
	match state:
		callbacks.ConnectionState.DISCONNECTED:
			return "DISCONNECTED"
		callbacks.ConnectionState.CONNECTED_HTTP:
			return "CONNECTED_HTTP"
		callbacks.ConnectionState.CONNECTED_LOBBY:
			return "CONNECTED_LOBBY"
		callbacks.ConnectionState.RECONNECTING:
			return "RECONNECTING"
	return "UNKNOWN"

# Reconnection logic

func attempt_reconnection() -> void:
	if connection_state != callbacks.ConnectionState.DISCONNECTED and connection_state != callbacks.ConnectionState.CONNECTED_HTTP:
		print("Cannot reconnect - not in appropriate state")
		return

	if _last_joined_lobby_code.is_empty():
		print("Cannot reconnect - no previous lobby code")
		return

	if current_reconnect_attempt >= max_reconnect_attempts:
		print("Max reconnection attempts reached")
		return

	current_reconnect_attempt += 1
	_set_connection_state(callbacks.ConnectionState.RECONNECTING)
	callbacks.on_reconnection_attempt(current_reconnect_attempt, max_reconnect_attempts)

	print("Attempting reconnection to lobby: ", _last_joined_lobby_code, " (attempt ", current_reconnect_attempt, "/", max_reconnect_attempts, ")")
	join_lobby(_last_joined_lobby_code)

	if reconnect_timer:
		reconnect_timer.stop()
		reconnect_timer.queue_free()

	reconnect_timer = Timer.new()
	add_child(reconnect_timer)
	reconnect_timer.wait_time = 5.0
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
		_set_connection_state(callbacks.ConnectionState.CONNECTED_HTTP)
		current_reconnect_attempt = 0

	if reconnect_timer:
		reconnect_timer.queue_free()
		reconnect_timer = null

func reset_reconnection_state() -> void:
	current_reconnect_attempt = 0
	if reconnect_timer:
		reconnect_timer.stop()
		reconnect_timer.queue_free()
		reconnect_timer = null

func _check_connection_health() -> void:
	if connection_state != callbacks.ConnectionState.CONNECTED_LOBBY:
		return

	var current_time = Time.get_ticks_msec() / 1000.0
	var time_since_last_activity = current_time - last_udp_activity

	if time_since_last_activity > connection_timeout:
		print("Connection timeout detected (%.1f seconds since last UDP activity)" % time_since_last_activity)
		_set_connection_state(callbacks.ConnectionState.DISCONNECTED)
		attempt_reconnection()

func _send_keepalive() -> void:
	# Send keepalive heartbeat to prevent being marked inactive during scene loading
	if connection_state == callbacks.ConnectionState.CONNECTED_LOBBY and adaptor and adaptor.is_udp_connected():
		if not current_lobby.is_empty() and player_id >= 0:
			var packet = {
				"type": "keepalive",
				"lobby_code": current_lobby.get("code", ""),
				"player_id": player_id
			}
			adaptor.send_udp_packet(packet)

func _on_udp_connection_timeout(lobby_data: Dictionary) -> void:
	if connection_state != callbacks.ConnectionState.CONNECTED_LOBBY:
		print("UDP connection timeout - falling back to HTTP-only mode")
		push_error("UDP connection failed, but you can still see the lobby. Multiplayer features may not work.")

		print("Fallback: Emitting lobby_joined signal without UDP confirmation")
		callbacks.on_lobby_joined(lobby_data)
		_set_connection_state(callbacks.ConnectionState.CONNECTED_HTTP)
