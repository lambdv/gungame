extends Node
## ServerCallbacks - Signal definitions and emissions for networking events
##
## Contains all callback functions that emit signals when networking events occur.
## This separates signal handling from business logic.

# Connection state enum
enum ConnectionState {
	DISCONNECTED,
	CONNECTED_HTTP,
	CONNECTED_LOBBY,
	RECONNECTING
}

# Lobby events
signal lobby_created(lobby_data: Dictionary)
signal lobby_joined(lobby_data: Dictionary)
signal lobby_join_failed(error: String, lobby_code: String)
signal lobby_left()
signal lobby_list_received(lobby_list: Array)

# Player events
signal player_joined(player_data: Dictionary)
signal player_left(player_id: int)
signal position_update_received(player_id: int, position: Vector3, rotation: Vector3)
signal server_dummy_updated(position: Vector3)
signal weapon_switched(player_id: int, weapon_id: int)
signal player_damaged(player_id: int, damage: int, attacker_id: int)

# Connection events
signal connection_confirmed()
signal connection_state_changed(new_state: ConnectionState)
signal reconnection_attempt(attempt: int, max_attempts: int)
signal state_sync_received(player_states: Array)

## Callback: Lobby created successfully
func on_lobby_created(lobby_data: Dictionary) -> void:
	lobby_created.emit(lobby_data)

## Callback: Lobby joined successfully
func on_lobby_joined(lobby_data: Dictionary) -> void:
	lobby_joined.emit(lobby_data)

## Callback: Lobby join/create failed
func on_lobby_join_failed(error: String, lobby_code: String) -> void:
	lobby_join_failed.emit(error, lobby_code)

## Callback: Left current lobby
func on_lobby_left() -> void:
	lobby_left.emit()

## Callback: Received lobby list from server
func on_lobby_list_received(lobby_list: Array) -> void:
	lobby_list_received.emit(lobby_list)

## Callback: Another player joined the lobby
func on_player_joined(player_data: Dictionary) -> void:
	player_joined.emit(player_data)

## Callback: A player left the lobby
func on_player_left(player_id: int) -> void:
	player_left.emit(player_id)

## Callback: Received position update from another player
func on_position_update_received(player_id: int, position: Vector3, rotation: Vector3) -> void:
	position_update_received.emit(player_id, position, rotation)

## Callback: Server dummy (bot) position updated
func on_server_dummy_updated(position: Vector3) -> void:
	server_dummy_updated.emit(position)

## Callback: A player switched weapons
func on_weapon_switched(player_id: int, weapon_id: int) -> void:
	weapon_switched.emit(player_id, weapon_id)

## Callback: A player was damaged
func on_player_damaged(player_id: int, damage: int, attacker_id: int) -> void:
	player_damaged.emit(player_id, damage, attacker_id)

## Callback: UDP connection confirmed
func on_connection_confirmed() -> void:
	connection_confirmed.emit()

## Callback: Connection state changed
func on_connection_state_changed(new_state: ConnectionState) -> void:
	connection_state_changed.emit(new_state)

## Callback: Reconnection attempt made
func on_reconnection_attempt(attempt: int, max_attempts: int) -> void:
	reconnection_attempt.emit(attempt, max_attempts)

## Callback: Received full state sync from server
func on_state_sync_received(player_states: Array) -> void:
	state_sync_received.emit(player_states)
