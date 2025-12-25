extends Node

enum GameState {
	PLAYING,
	PAUSED,
	SPECTATING
}

var current_state: GameState = GameState.PLAYING

# Signals for state changes
signal state_changed(new_state: GameState)
signal paused
signal resumed
signal spectating_started
signal spectating_ended

func _ready() -> void:
	set_state(GameState.PLAYING)

func set_state(new_state: GameState) -> void:
	if current_state == new_state:
		return
	
	var previous_state = current_state
	current_state = new_state
	state_changed.emit(new_state)
	
	# Emit specific state signals
	# Note: We don't pause the scene tree for multiplayer - only disable input processing
	match new_state:
		GameState.PAUSED:
			if previous_state == GameState.PLAYING:
				paused.emit()
		GameState.PLAYING:
			if previous_state == GameState.PAUSED:
				resumed.emit()
		GameState.SPECTATING:
			if previous_state != GameState.SPECTATING:
				spectating_started.emit()
		_:
			if previous_state == GameState.SPECTATING:
				spectating_ended.emit()

func pause() -> void:
	if current_state == GameState.PLAYING:
		set_state(GameState.PAUSED)

func resume() -> void:
	if current_state == GameState.PAUSED:
		set_state(GameState.PLAYING)

func toggle_pause() -> void:
	if current_state == GameState.PAUSED:
		resume()
	elif current_state == GameState.PLAYING:
		pause()

func start_spectating() -> void:
	set_state(GameState.SPECTATING)

func stop_spectating() -> void:
	if current_state == GameState.SPECTATING:
		set_state(GameState.PLAYING)

func is_playing() -> bool:
	return current_state == GameState.PLAYING

func is_paused() -> bool:
	return current_state == GameState.PAUSED

func is_spectating() -> bool:
	return current_state == GameState.SPECTATING

func can_process_input() -> bool:
	return current_state == GameState.PLAYING

