extends Node

class_name NetworkClient

# Signals
signal lobby_created(lobby_code: String)
signal lobby_joined(lobby_code: String, lobby_data: Dictionary)
signal lobby_join_failed(reason: String)
signal connected_to_lobby(lobby_address: String, lobby_port: int)
signal connection_failed(reason: String)
signal player_position_updated(player_id: String, position: Vector3)
signal server_dummy_updated(position: Vector3)

# Configuration
const SERVER_HTTP_URL = "http://localhost:8080"
const UDP_BUFFER_SIZE = 4096

# State
var http_request: HTTPRequest
var udp_peer: PacketPeerUDP
var lobby_code: String = ""
var lobby_address: String = ""
var lobby_port: int = 0
var player_id: String = ""
var connected: bool = false
var position_update_timer: Timer

func _ready() -> void:
	_initialize_http_client()
	_initialize_udp_client()
	_initialize_position_timer()

func _initialize_http_client() -> void:
	http_request = HTTPRequest.new()
	add_child(http_request)
	http_request.request_completed.connect(_on_http_request_completed)

func _initialize_udp_client() -> void:
	udp_peer = PacketPeerUDP.new()

func _initialize_position_timer() -> void:
	position_update_timer = Timer.new()
	position_update_timer.wait_time = 0.1  # 10 updates per second
	position_update_timer.timeout.connect(_send_position_update)
	add_child(position_update_timer)

# HTTP API Methods
func create_lobby(lobby_code: String, player_name: String = "Player") -> void:
	var url = SERVER_HTTP_URL + "/lobbies"
	var headers = ["Content-Type: application/json"]
	var body = JSON.stringify({
		"code": lobby_code,
		"player_name": player_name
	})

	var error = http_request.request(url, headers, HTTPClient.METHOD_POST, body)
	if error != OK:
		lobby_join_failed.emit("Failed to send create lobby request")

func join_lobby(lobby_code: String, player_name: String = "Player") -> void:
	var url = SERVER_HTTP_URL + "/lobbies/" + lobby_code + "/join"
	var headers = ["Content-Type: application/json"]
	var body = JSON.stringify({
		"player_name": player_name
	})

	var error = http_request.request(url, headers, HTTPClient.METHOD_POST, body)
	if error != OK:
		lobby_join_failed.emit("Failed to send join lobby request")

func _on_http_request_completed(result: int, response_code: int, headers: PackedStringArray, body: PackedByteArray) -> void:
	if result != HTTPRequest.RESULT_SUCCESS:
		lobby_join_failed.emit("HTTP request failed")
		return

	var response_text = body.get_string_from_utf8()
	var json = JSON.new()
	var error = json.parse(response_text)

	if error != OK:
		lobby_join_failed.emit("Failed to parse server response")
		return

	var response = json.data

	match response_code:
		200, 201:
			# Success - lobby created or joined
			lobby_code = response.get("lobby_code", "")
			lobby_address = response.get("udp_address", "127.0.0.1")
			lobby_port = response.get("udp_port", 7778)
			player_id = response.get("player_id", "")

			lobby_joined.emit(lobby_code, response)
			_connect_to_lobby_udp()
		404:
			lobby_join_failed.emit("Lobby not found")
		409:
			lobby_join_failed.emit("Lobby full or already exists")
		_:
			lobby_join_failed.emit("Server error: " + str(response_code))

# UDP Methods
func _connect_to_lobby_udp() -> void:
	var error = udp_peer.connect_to_host(lobby_address, lobby_port)
	if error != OK:
		connection_failed.emit("Failed to connect UDP socket")
		return

	connected = true
	connected_to_lobby.emit(lobby_address, lobby_port)
	position_update_timer.start()

	print("Connected to lobby UDP: ", lobby_address, ":", lobby_port)

func _send_position_update() -> void:
	if not connected or player_id.is_empty():
		return

	# Get local player position (this will be injected from the game)
	var local_player = _get_local_player()
	if not local_player:
		return

	var position = local_player.global_position
	var data = {
		"type": "position_update",
		"player_id": player_id,
		"position": {
			"x": position.x,
			"y": position.y,
			"z": position.z
		}
	}

	var json_string = JSON.stringify(data)
	var packet = json_string.to_utf8_buffer()

	udp_peer.put_packet(packet)

func _process(delta: float) -> void:
	if not connected:
		return

	# Process incoming UDP packets
	while udp_peer.get_available_packet_count() > 0:
		var packet = udp_peer.get_packet()
		var packet_string = packet.get_string_from_utf8()

		var json = JSON.new()
		var error = json.parse(packet_string)

		if error != OK:
			print("Failed to parse UDP packet")
			continue

		var data = json.data
		_handle_udp_packet(data)

func _handle_udp_packet(data: Dictionary) -> void:
	var packet_type = data.get("type", "")

	match packet_type:
		"position_update":
			var player_id = data.get("player_id", "")
			var position_data = data.get("position", {})
			var position = Vector3(
				position_data.get("x", 0.0),
				position_data.get("y", 0.0),
				position_data.get("z", 0.0)
			)
			player_position_updated.emit(player_id, position)

		"server_dummy_update":
			var position_data = data.get("position", {})
			var position = Vector3(
				position_data.get("x", 0.0),
				position_data.get("y", 0.0),
				position_data.get("z", 0.0)
			)
			server_dummy_updated.emit(position)

# Helper methods
func _get_local_player() -> Node:
	# This should be set by the game world
	# For now, return null - will be implemented by the autoload
	return null

func set_local_player(player: Node) -> void:
	# Store reference to local player for position updates
	# This will be called by the game world
	pass

func disconnect_from_lobby() -> void:
	if connected:
		udp_peer.close()
		connected = false
		position_update_timer.stop()
		lobby_code = ""
		player_id = ""

func is_connected_to_lobby() -> bool:
	return connected
