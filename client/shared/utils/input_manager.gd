extends Node
## InputManager - Centralized Input Handling System
##
## Processes all player input and broadcasts it via signals to interested systems.
## This centralizes input logic and allows for easy input remapping, UI focus management,
## and state-based input filtering (e.g., disable input during pause menu).
##
## Key features:
## - State-aware input processing (only processes input when game is in playing state)
## - Mouse capture management for camera control vs UI interaction
## - Signal-based communication to avoid tight coupling
## - Frame-perfect input state tracking

# Input state tracking - current state of all input devices
var movement_input := Vector2.ZERO  # WASD/controller movement vector (-1 to 1)
var jump_pressed := false  # Jump button held down
var jump_just_pressed := false  # Jump button pressed this frame (edge trigger)
var mouse_motion := Vector2.ZERO  # Mouse movement delta this frame
var mouse_captured := true  # Whether mouse is captured for camera control
var weapon_keys_pressed := [false, false, false]  # Track keys 1, 2, 3 for weapon switching

# Signals - broadcast input changes to any system that needs them
signal movement_input_changed(input: Vector2)  # Movement vector changed
signal jump_pressed_changed(pressed: bool)  # Jump button state changed
signal jump_just_pressed_changed(pressed: bool)  # Jump button just pressed
signal mouse_motion_changed(motion: Vector2)  # Mouse moved
signal mouse_capture_changed(captured: bool)  # Mouse capture state changed
signal weapon_switch_requested(slot: int)  # Player wants to switch to weapon slot (1-3)
signal attack_pressed  # Attack button pressed (allows holding)
signal reload_pressed  # Reload button pressed

func _ready() -> void:
	# Capture mouse for camera control
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)

## _physics_process
## Main input processing loop - called every physics frame (60fps)
## Handles state-aware input processing and signal emission
func _physics_process(_delta: float) -> void:
	# Reset per-frame inputs that should only trigger once per press
	jump_just_pressed = false
	mouse_motion = Vector2.ZERO

	# Handle pause menu toggle - only when actively playing
	# (pause menu itself handles escape when already paused)
	if Input.is_action_just_pressed("escape") and GameStateManager.is_playing():
		GameStateManager.pause()

	# State-aware input processing - only process game input when actually playing
	# This prevents input from affecting gameplay during menus, pause, etc.
	if not GameStateManager.can_process_input():
		# Clear any lingering input state to prevent stuck inputs
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
	# Don't recapture mouse automatically - let individual scenes handle this
	# The mouse will be recaptured when entering gameplay

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
