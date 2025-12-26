extends Node
## NetworkingAdaptor - Thin wrapper around Godot's HTTP and UDP networking
##
## Provides low-level networking primitives:
## - HTTP request/response handling
## - UDP packet send/receive
## No business logic, just raw networking operations

var http_request: HTTPRequest
var udp_peer: PacketPeerUDP
var connected_to_udp: bool = false

# HTTP request tracking
var current_request_id: int = 0
var pending_requests: Dictionary = {}

signal http_response_received(request_id: int, result: int, response_code: int, headers: Array, body: PackedByteArray) # Signal emitted when HTTP response is received

signal udp_packet_received(data: Dictionary) # Signal emitted when UDP packet is received

func _ready() -> void:
	http_request = HTTPRequest.new()
	add_child(http_request)
	http_request.request_completed.connect(_on_http_request_completed)
	http_request.use_threads = false
	http_request.timeout = 10.0
	
	udp_peer = PacketPeerUDP.new()

func _exit_tree() -> void:
	if udp_peer:
		udp_peer.close()

## Sends an HTTP request
## @param url: Full URL to request
## @param headers: Array of header strings
## @param method: HTTPClient.METHOD_* constant
## @param body: Request body string
## @return: Request ID (or -1 on error)
func send_http_request(url: String, headers: Array, method: int, body: String) -> int:
	var error = http_request.request(url, headers, method, body)
	if error != OK:
		push_error("HTTP Request failed: " + str(error))
		return -1

	current_request_id += 1
	pending_requests[current_request_id] = {
		"url": url,
		"timestamp": Time.get_ticks_msec()
	}
	return current_request_id

func _on_http_request_completed(result: int, response_code: int, headers: Array, body: PackedByteArray) -> void:
	# Find the oldest pending request (HTTPRequest processes sequentially)
	var matched_request_id = -1
	if not pending_requests.is_empty():
		var keys = pending_requests.keys()
		if keys.size() > 0:
			keys.sort()
			matched_request_id = keys[0]
			pending_requests.erase(matched_request_id)

	http_response_received.emit(matched_request_id, result, response_code, headers, body)

## Connects to UDP server
## @param ip: Server IP address
## @param port: Server port
## @return: true if connection successful
func connect_udp(ip: String, port: int) -> bool:
	if udp_peer.connect_to_host(ip, port) != OK:
		push_error("Failed to connect UDP to " + ip + ":" + str(port))
		return false
	connected_to_udp = true
	return true

## Sends a UDP packet
## @param data: Dictionary to send (will be JSON stringified)
## @return: true if send successful
func send_udp_packet(data: Dictionary) -> bool:
	if not connected_to_udp:
		return false

	var json_string = JSON.stringify(data)
	if json_string.is_empty():
		push_error("UDP: Failed to stringify packet: " + str(data))
		return false

	var packet = json_string.to_utf8_buffer()
	var error = udp_peer.put_packet(packet)
	if error != OK:
		push_error("UDP: Failed to send packet '" + data.get("type", "unknown") + "': " + str(error))
		return false
	return true

## Processes incoming UDP packets (call from _process)
## Automatically emits udp_packet_received signal for valid packets
func process_udp_packets() -> void:
	if not connected_to_udp:
		return

	const MAX_PACKETS_PER_FRAME = 10
	var packets_processed = 0

	while udp_peer.get_available_packet_count() > 0 and packets_processed < MAX_PACKETS_PER_FRAME:
		var packet = udp_peer.get_packet()
		var packet_string = packet.get_string_from_utf8()

		if packet_string.is_empty():
			push_error("UDP: Received empty packet")
			packets_processed += 1
			continue

		var json = JSON.new()
		var parse_result = json.parse(packet_string)

		if parse_result == OK:
			udp_packet_received.emit(json.data)
		else:
			push_error("UDP: Failed to parse packet: '" + packet_string + "' (error: " + str(parse_result) + ")")

		packets_processed += 1

## Closes UDP connection
func close_udp() -> void:
	if udp_peer:
		udp_peer.close()
	connected_to_udp = false

## Checks if UDP is connected
func is_udp_connected() -> bool:
	return connected_to_udp
