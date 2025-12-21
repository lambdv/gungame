extends Node

# Server configuration
const SERVER_URL = "http://127.0.0.1:8080"
const UDP_PORT = 8081

# HTTP client
var http_request: HTTPRequest
var current_request_id: int = 0
var pending_requests: Dictionary = {}

# UDP client
var udp_peer: PacketPeerUDP
var connected_to_udp: bool = false

# Lobby state
var current_lobby: Dictionary = {}
var player_id: int = -1
var connected_players: Dictionary = {}

# Signals
signal lobby_created(lobby_data: Dictionary)
signal lobby_joined(lobby_data: Dictionary)
signal lobby_join_failed(error: String)
signal lobby_list_received(lobby_list: Array)
signal player_joined(player_data: Dictionary)
signal player_left(player_id: int)
signal position_update_received(player_id: int, position: Vector3, rotation: Vector3)
signal server_dummy_updated(position: Vector3)

func _ready() -> void:
	_setup_http_client()
	_setup_udp_client()

func _setup_http_client() -> void:
	http_request = HTTPRequest.new()
	add_child(http_request)
	http_request.request_completed.connect(_on_http_request_completed)
	# Configure HTTP request
	http_request.use_threads = false
	http_request.timeout = 10.0

func _setup_udp_client() -> void:
	udp_peer = PacketPeerUDP.new()

func _exit_tree() -> void:
	if udp_peer:
		udp_peer.close()

# HTTP API Methods
func create_lobby(code: String, scene: String = "world", max_players: int = 4) -> void:
	var url = SERVER_URL + "/lobbies"
	var headers = ["Content-Type: application/json"]
	var body = JSON.stringify({
		"code": code,
		"scene": scene,
		"max_players": max_players
	})

	var request_id = _make_request(url, headers, HTTPClient.METHOD_POST, body)
	pending_requests[request_id] = "create_lobby"

func join_lobby(code: String) -> void:
	var url = SERVER_URL + "/lobbies/" + code + "/join"
	var headers = ["Content-Type: application/json"]
	var body = JSON.stringify({
		"player_name": "Player"
	})

	var request_id = _make_request(url, headers, HTTPClient.METHOD_POST, body)
	pending_requests[request_id] = "join_lobby"

func get_lobby_info(code: String) -> void:
	var url = SERVER_URL + "/lobbies/" + code
	var request_id = _make_request(url, [], HTTPClient.METHOD_GET, "")
	pending_requests[request_id] = "get_lobby_info"

func get_lobby_list() -> void:
	var url = SERVER_URL + "/lobbies"
	var request_id = _make_request(url, [], HTTPClient.METHOD_GET, "")
	pending_requests[request_id] = "get_lobby_list"

func _make_request(url: String, headers: Array, method: int, body: String) -> int:
	var error = http_request.request(url, headers, method, body)
	if error != OK:
		push_error("HTTP Request failed: " + str(error))
		return -1

	current_request_id += 1
	return current_request_id

func _on_http_request_completed(result: int, response_code: int, headers: Array, body: PackedByteArray) -> void:
	var response_text = body.get_string_from_utf8()
	var json = JSON.new()
	var parse_result = json.parse(response_text)

	var response_data = {}
	if parse_result == OK:
		response_data = json.data
	else:
		push_error("Failed to parse JSON response: " + response_text)
		return

	# Find the request type
	var request_type = pending_requests.get(current_request_id, "")
	pending_requests.erase(current_request_id)

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
	if response_code == 200:
		current_lobby = data.get("lobby", {})
		player_id = data.get("player_id", -1)

		# Connect to UDP server
		var server_ip = current_lobby.get("server_ip", "127.0.0.1")
		var udp_port = current_lobby.get("udp_port", UDP_PORT)
		connect_udp_server(server_ip, udp_port)

		lobby_joined.emit(current_lobby)
		print("Joined lobby successfully: ", current_lobby.get("code", "unknown"))
	else:
		lobby_join_failed.emit("Failed to join lobby: " + str(response_code))
		push_error("Failed to join lobby: " + str(response_code) + " - " + str(data))

func _handle_get_lobby_info_response(response_code: int, data: Dictionary) -> void:
	if response_code == 200:
		current_lobby = data
		print("Lobby info retrieved: ", data.get("code", "unknown"))
	else:
		push_error("Failed to get lobby info: " + str(response_code) + " - " + str(data))

func _handle_get_lobby_list_response(response_code: int, data: Array) -> void:
	if response_code == 200:
		lobby_list_received.emit(data)
		print("Lobby list retrieved: ", data.size(), " lobbies")
	else:
		push_error("Failed to get lobby list: " + str(response_code) + " - " + str(data))

func _handle_try_connect_test_lobby_response(response_code: int, data: Dictionary) -> void:
	if response_code == 200:
		# Lobby exists, try to join it
		print("Test lobby exists, attempting to join...")
		join_lobby("TEST")
	else:
		# Lobby doesn't exist, create it
		print("Test lobby doesn't exist, creating it...")
		create_lobby("TEST")

# UDP Methods
func connect_udp_server(ip: String, port: int) -> void:
	print("Attempting to connect UDP to " + ip + ":" + str(port))
	if udp_peer.connect_to_host(ip, port) != OK:
		push_error("Failed to connect UDP to " + ip + ":" + str(port))
		return

	connected_to_udp = true
	print("Connected to UDP server at " + ip + ":" + str(port))

	# Send join packet
	send_join_packet()

func send_join_packet() -> void:
	if not connected_to_udp or current_lobby.is_empty():
		return

	var packet = {
		"type": "join",
		"lobby_code": current_lobby.get("code", ""),
		"player_id": player_id
	}

	_send_udp_packet(packet)

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

func _send_udp_packet(data: Dictionary) -> void:
	if not connected_to_udp:
		return

	var json_string = JSON.stringify(data)
	var packet = json_string.to_utf8_buffer()
	udp_peer.put_packet(packet)

func _process(delta: float) -> void:
	if not connected_to_udp:
		return

	# Process incoming UDP packets
	while udp_peer.get_available_packet_count() > 0:
		var packet = udp_peer.get_packet()
		var packet_string = packet.get_string_from_utf8()
		var json = JSON.new()

		if json.parse(packet_string) == OK:
			_process_udp_packet(json.data)

func _process_udp_packet(data: Dictionary) -> void:
	var packet_type = data.get("type", "")

	match packet_type:
		"welcome":
			print("Received welcome from server")
			# Server acknowledged our connection

		"position_update":
			var player_id = data.get("player_id", -1)
			var pos_data = data.get("position", {})

			var position = Vector3(
				pos_data.get("x", 0.0),
				pos_data.get("y", 0.0),
				pos_data.get("z", 0.0)
			)

			# For now, we don't receive rotation updates from server
			var rotation = Vector3.ZERO

			position_update_received.emit(player_id, position, rotation)

		"player_joined":
			var player_data = data.get("player", {})
			connected_players[player_data.get("id", -1)] = player_data
			player_joined.emit(player_data)

		"player_left":
			var player_id = data.get("player_id", -1)
			connected_players.erase(player_id)
			player_left.emit(player_id)

		"server_dummy_update":
			var pos_data = data.get("position", {})
			var position = Vector3(
				pos_data.get("x", 0.0),
				pos_data.get("y", 0.0),
				pos_data.get("z", 0.0)
			)
			server_dummy_updated.emit(position)

# Test lobby methods
func connect_to_test_lobby() -> void:
	print("Attempting to connect to test lobby...")
	# Try to get lobby info first
	var get_lobby_request_id = _make_request(
		SERVER_URL + "/lobbies/TEST",
		[],
		HTTPClient.METHOD_GET,
		""
	)
	pending_requests[get_lobby_request_id] = "try_connect_test_lobby"

# Utility methods
func is_connected_to_lobby() -> bool:
	return not current_lobby.is_empty() and connected_to_udp

func get_current_lobby_code() -> String:
	return current_lobby.get("code", "")

func get_connected_player_ids() -> Array:
	return connected_players.keys()
