extends Node

# Input state variables
var movement_input := Vector2.ZERO
var jump_pressed := false
var jump_just_pressed := false
var mouse_motion := Vector2.ZERO
var mouse_captured := true
var weapon_keys_pressed := [false, false, false]  # Track keys 1, 2, 3

# Signals for propagating input to other nodes
signal movement_input_changed(input: Vector2)
signal jump_pressed_changed(pressed: bool)
signal jump_just_pressed_changed(pressed: bool)
signal mouse_motion_changed(motion: Vector2)
signal mouse_capture_changed(captured: bool)
signal weapon_switch_requested(slot: int)
signal attack_pressed
signal reload_pressed

func _ready() -> void:
	# Capture mouse for camera control
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)

func _physics_process(_delta: float) -> void:
	# Reset frame-based inputs
	jump_just_pressed = false
	mouse_motion = Vector2.ZERO

	# Process escape key to toggle mouse capture or pause
	if Input.is_action_just_pressed("escape"):
		if GameStateManager.is_playing():
			toggle_mouse_capture()
		elif GameStateManager.is_paused():
			GameStateManager.resume()

	# Only process game input if in playing state
	if not GameStateManager.can_process_input():
		# Clear movement input when not playing
		if movement_input != Vector2.ZERO:
			movement_input = Vector2.ZERO
			movement_input_changed.emit(movement_input)
		if jump_pressed:
			jump_pressed = false
			jump_pressed_changed.emit(false)
		return

	# Process movement input
	var new_movement_input := Input.get_vector("move_left", "move_right", "move_up", "move_down")
	if new_movement_input != movement_input:
		movement_input = new_movement_input
		movement_input_changed.emit(movement_input)

	# Process jump input
	var new_jump_pressed := Input.is_action_pressed("accept")
	if new_jump_pressed != jump_pressed:
		jump_pressed = new_jump_pressed
		jump_pressed_changed.emit(jump_pressed)

	var new_jump_just_pressed := Input.is_action_just_pressed("accept")
	if new_jump_just_pressed:
		jump_just_pressed = true
		jump_just_pressed_changed.emit(true)
	
	# Process weapon switching (keys 1, 2, 3)
	for i in range(3):
		var key = KEY_1 + i
		var pressed = Input.is_key_pressed(key)
		if pressed and not weapon_keys_pressed[i]:
			weapon_switch_requested.emit(i + 1)
		weapon_keys_pressed[i] = pressed
	
	# Process attack input (left click or right trigger) - allows holding
	if Input.is_action_pressed("attack"):
		attack_pressed.emit()

	# Process reload input (R key or Y button) - just pressed
	if Input.is_action_just_pressed("reload"):
		reload_pressed.emit()

func _unhandled_input(event: InputEvent) -> void:
	# Only process mouse input if in playing state
	if not GameStateManager.can_process_input():
		return
	
	if event is InputEventMouseMotion and mouse_captured:
		mouse_motion = event.relative
		mouse_motion_changed.emit(mouse_motion)
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed and not mouse_captured:
		# Recapture mouse when clicking on screen
		capture_mouse()

# Public methods for other nodes to access input state
func get_movement_input() -> Vector2:
	return movement_input

func is_jump_pressed() -> bool:
	return jump_pressed

func is_jump_just_pressed() -> bool:
	return jump_just_pressed

func get_mouse_motion() -> Vector2:
	return mouse_motion

func is_mouse_captured() -> bool:
	return mouse_captured

func toggle_mouse_capture() -> void:
	if mouse_captured:
		release_mouse()
	else:
		capture_mouse()

func capture_mouse() -> void:
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	mouse_captured = true
	mouse_capture_changed.emit(true)

func release_mouse() -> void:
	Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	mouse_captured = false
	mouse_capture_changed.emit(false)
