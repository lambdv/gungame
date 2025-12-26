extends CharacterBody3D
## Player - First-Person Character Controller
##
## Main player character with FPS controls, inventory management, and combat.
## Supports both local player (with input/camera) and remote players (networked).
##
## Features:
## - WASD movement with mouse look camera
## - Jump mechanics with physics
## - Weapon sway animations for immersion
## - Inventory system with weapon switching
## - Health/damage system
## - Network synchronization for multiplayer
##
## Architecture:
## - CharacterBody3D for physics-based movement
## - Camera rig with head bob and weapon sway
## - Component-based: Damageable, Inventory, HealthBar3D
## - Signal-based communication with other systems

# Movement and camera constants
const SPEED = 8.0  # Base movement speed in units per second
const JUMP_VELOCITY = 4.5  # Initial upward velocity when jumping
const SENSITIVITY = 0.003  # Mouse sensitivity multiplier for camera rotation
const MAX_VERTICAL_ROTATION = deg_to_rad(89)  # Prevent camera flipping upside down

# Weapon sway animation constants - for realistic weapon movement
const SWAY_AMOUNT = 0.02  # How far weapons sway side-to-side while moving
const SWAY_SPEED = 8.0  # How quickly weapon sway animation responds
const JUMP_SWAY_AMOUNT = 0.05  # Extra weapon movement when jumping/landing
const JUMP_SWAY_SPEED = 10.0  # Speed of jump-related weapon animation

# Scene node references - camera and model hierarchy
@onready var camera_rig: Node3D = $CameraRig  # Root of camera system (handles rotation)
@onready var head: Node3D = $CameraRig/Head  # Head node for vertical camera rotation
@onready var camera: Camera3D = $CameraRig/Head/Camera3D  # Actual camera for rendering
@onready var hand_item: Node3D = $CameraRig/Head/Camera3D/HandItem  # Weapon attachment point
@onready var character_model: Node3D = get_node_or_null("rachel__black_heart_lovell_v5vrm")  # 3D model (optional)

# Player state variables
var is_local := true  # Whether this player is controlled by local input (vs remote networked player)
var player_id := -1  # Network player ID (assigned by server)
var current_movement_input := Vector2.ZERO  # Current WASD/controller input vector
var current_mouse_motion := Vector2.ZERO  # Mouse movement delta this frame

# Interpolation state for remote players (smooth movement)
var target_position := Vector3.ZERO  # Server-authoritative position to interpolate towards
var target_rotation := Vector3.ZERO  # Server-authoritative rotation to interpolate towards
var interpolation_speed := 8.0  # How quickly to interpolate (higher = more responsive)
var last_position_update := 0.0  # Timestamp of last position update for extrapolation
var has_received_position := false  # Whether we've received at least one position update

# Reconciliation state for local players (server correction)
var reconciliation_speed := 15.0  # How quickly to reconcile with server (higher = snappier correction)
var needs_reconciliation := false  # Whether local position needs server correction

# Component references - modular player systems
var inventory: Node = null  # Weapon inventory management
var damageable: Node = null  # Health and damage handling
var health_bar_3d: Node3D = null  # Floating 3D health bar above player
var current_weapon_instance: Node3D = null  # Currently equipped weapon scene instance
var hand_item_base_position: Vector3  # Original weapon position for sway animations
var was_on_floor: bool = true  # Previous frame's ground state for jump animations
var networking_manager: Node = null  # Reference to networking manager for multiplayer sync
var pending_weapon_switch_id: int = -1  # Track weapon switch we initiated to avoid applying it from state sync
# Attack cooldown removed - now handled by weapon fire rate

## _ready
## Initialize player components and set up signal connections
## Called when the player node enters the scene tree
func _ready() -> void:
	# Add to "players" group so HUD systems can find all player instances
	add_to_group("players")

	# Initialize health/damage system - core component for combat
	damageable = preload("res://shared/utils/damageable.gd").new()
	add_child(damageable)
	damageable.health_changed.connect(_on_health_changed)
	damageable.died.connect(_on_player_died)

	# Initialize floating 3D health bar - only visible on remote players
	health_bar_3d = preload("res://ui/health_bar_3d.gd").new()
	add_child(health_bar_3d)
	health_bar_3d.set_target(self)  # Tell health bar to follow this player
	health_bar_3d.set_health(damageable.current_health, damageable.max_health)
	health_bar_3d.visible = not is_local  # Hide health bar on local player

	# Initialize weapon inventory system
	inventory = preload("res://entites/player/inventory.gd").new()
	add_child(inventory)
	inventory.active_weapon_changed.connect(_on_active_weapon_changed)
	
	# Input connections are now handled in set_is_local()
	set_is_local(is_local)  # Initialize with current is_local value
	
	if not inventory.is_data_loaded:
		await inventory.data_loaded
	await get_tree().process_frame
	
	hand_item_base_position = hand_item.position
	
	# Hide character model for local player
	if is_local and character_model:
		character_model.visible = false
	
	var weapon_id = inventory.get_active_weapon_id()
	if weapon_id > 0:
		load_weapon(weapon_id)

func set_is_local(value: bool) -> void:
	var was_local = is_local
	is_local = value

	if is_local:
		# Connect inputs if not already connected
		if not InputManager.movement_input_changed.is_connected(_on_movement_input_changed):
			InputManager.movement_input_changed.connect(_on_movement_input_changed)
			InputManager.jump_just_pressed_changed.connect(_on_jump_just_pressed)
			InputManager.mouse_motion_changed.connect(_on_mouse_motion_changed)
			InputManager.weapon_switch_requested.connect(_on_weapon_switch_requested)
			InputManager.attack_pressed.connect(_on_attack_pressed)
			InputManager.reload_pressed.connect(_on_reload_pressed)

		# Enable physics for local player
		set_physics_process(true)
	elif was_local:
		# Disconnect inputs when becoming non-local
		InputManager.movement_input_changed.disconnect(_on_movement_input_changed)
		InputManager.jump_just_pressed_changed.disconnect(_on_jump_just_pressed)
		InputManager.mouse_motion_changed.disconnect(_on_mouse_motion_changed)
		InputManager.weapon_switch_requested.disconnect(_on_weapon_switch_requested)
		InputManager.attack_pressed.disconnect(_on_attack_pressed)
		InputManager.reload_pressed.disconnect(_on_reload_pressed)

		# Keep physics processing for interpolation, but disable physics simulation
		# Remote players will interpolate but not simulate physics

	if not is_local and camera:
		camera.current = false
	if is_local and character_model:
		character_model.visible = false
	elif not is_local and character_model:
		character_model.visible = true

	# Show/hide health bar based on local status
	if health_bar_3d:
		health_bar_3d.visible = not is_local

## set_player_id
## Sets the network player ID for this player instance
## @param id: The player ID assigned by the server
func set_player_id(id: int) -> void:
	player_id = id

## apply_state_sync
## Applies full state synchronization data from server
## @param state_data: Dictionary containing player state (health, ammo, weapon, etc.)
func apply_state_sync(state_data: Dictionary) -> void:
	# Note: Position and rotation are handled separately by position_update packets
	# This method only handles health, ammo, weapon, and reload state

	# Update health (server-authoritative)
	if damageable:
		var new_health = state_data.get("health", damageable.current_health)
		var max_health = state_data.get("max_health", damageable.max_health)
		damageable.current_health = new_health
		damageable.max_health = max_health

	# Update weapon and ammo (server-authoritative)
	if inventory:
		var weapon_id = state_data.get("current_weapon_id", inventory.get_active_weapon_id())
		var current_ammo = state_data.get("current_ammo", 0)
		var max_ammo = state_data.get("max_ammo", 0)

		# For local players, only apply weapon switch from state sync if:
		# 1. We didn't initiate this weapon switch ourselves, OR
		# 2. The weapon in state sync matches what we're already using (server confirmation)
		# This prevents the local player from switching weapons twice when they initiate a switch
		var should_apply_weapon_switch = true
		var current_weapon_id = inventory.get_active_weapon_id()
		if is_local:
			# If we have a pending weapon switch and state sync matches it, clear the pending flag
			if pending_weapon_switch_id == weapon_id and current_weapon_id == weapon_id:
				# Server confirmed our weapon switch - clear pending flag
				pending_weapon_switch_id = -1
				should_apply_weapon_switch = false
			# If we have a pending weapon switch but state sync doesn't match, ignore it (our switch is newer)
			elif pending_weapon_switch_id != -1 and pending_weapon_switch_id != weapon_id:
				should_apply_weapon_switch = false
			# If state sync matches current weapon, no need to switch
			elif current_weapon_id == weapon_id:
				should_apply_weapon_switch = false

		# Switch to correct weapon if different (server-authoritative)
		if should_apply_weapon_switch and current_weapon_id != weapon_id and weapon_id > 0:
			# Find slot containing this weapon
			var found_slot = false
			for slot in range(1, 4):
				if inventory.get_weapon_id_in_slot(slot) == weapon_id:
					inventory.switch_to_weapon(slot)
					found_slot = true
					# Clear pending switch if we're applying a server-initiated switch
					if is_local:
						pending_weapon_switch_id = -1
					break
			
			# If weapon not found in any slot, log warning but don't crash
			if not found_slot:
				push_warning("Weapon ID " + str(weapon_id) + " not found in inventory slots")

		# Update ammo on current weapon (wait for weapon to load if needed)
		if current_weapon_instance and current_weapon_instance.has_method("set_current_ammo"):
			current_weapon_instance.set_current_ammo(current_ammo)
			current_weapon_instance.set_max_ammo(max_ammo)

	# Update reload state
	var is_reloading = state_data.get("is_reloading", false)
	if current_weapon_instance and current_weapon_instance.has_method("set_reloading_state"):
		current_weapon_instance.set_reloading_state(is_reloading)

func _physics_process(delta: float) -> void:
	if not is_local:
		# For remote players, only interpolate towards server position/rotation
		# No physics simulation (gravity, collision, movement)
		interpolate_to_target(delta)
		return

	# Local player physics simulation
	# Add the gravity.
	if not is_on_floor():
		velocity += get_gravity() * delta

	# Apply server reconciliation if needed (smooth correction)
	if needs_reconciliation:
		var reconciliation_delta = reconciliation_speed * delta
		global_position = global_position.lerp(target_position, reconciliation_delta)

		# Check if reconciliation is complete
		if global_position.distance_to(target_position) < 0.01:
			global_position = target_position
			needs_reconciliation = false

	# Handle movement (prediction - immediate response)
	move(current_movement_input)

	# Handle camera rotation
	look()

	# Handle weapon sway
	update_weapon_sway(delta)
	move_and_slide()

	was_on_floor = is_on_floor()

func jump() -> void:
	velocity.y = JUMP_VELOCITY

func move(input_dir: Vector2) -> void:
	# Use player's basis for movement direction (player body rotation determines facing)
	var horizontal_basis = global_transform.basis

	var direction = (horizontal_basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	if direction:
		velocity.x = direction.x * SPEED
		velocity.z = direction.z * SPEED
	else:
		velocity.x = move_toward(velocity.x, 0, SPEED)
		velocity.z = move_toward(velocity.z, 0, SPEED)

func look():
	if current_mouse_motion != Vector2.ZERO:
		rotate_y(-current_mouse_motion.x * SENSITIVITY) # Horizontal rotation - rotate entire player body

		# Vertical rotation - rotate both head/camera and character model
		var new_rotation_x = head.rotation.x - current_mouse_motion.y * SENSITIVITY
		head.rotation.x = clamp(new_rotation_x, -MAX_VERTICAL_ROTATION, MAX_VERTICAL_ROTATION)

		# Apply vertical rotation to character model so other players can see where we're looking
		if character_model:
			character_model.rotation.x = head.rotation.x

		current_mouse_motion = Vector2.ZERO  # Reset after use

# Signal handlers


func _on_movement_input_changed(input: Vector2) -> void:
	current_movement_input = input

func _on_jump_just_pressed(pressed: bool) -> void:
	if pressed and is_on_floor():
		jump()

func _on_mouse_motion_changed(motion: Vector2) -> void:
	current_mouse_motion = motion

func _on_weapon_switch_requested(slot: int) -> void:
	if not inventory:
		return
	
	# Switch weapon in inventory (this will emit active_weapon_changed signal)
	inventory.switch_to_weapon(slot)
	
	# If this is the local player, sync weapon switch to server
	if is_local and networking_manager:
		var weapon_id = inventory.get_active_weapon_id()
		if weapon_id > 0:
			# Track this weapon switch so we don't apply it again from state sync
			pending_weapon_switch_id = weapon_id
			networking_manager.send_weapon_switch(weapon_id)

func _on_active_weapon_changed(weapon_id: int) -> void:
	if weapon_id > 0:
		load_weapon(weapon_id)

func _on_health_changed(new_health: int, max_health: int) -> void:
	# Update 3D health bar
	if health_bar_3d:
		health_bar_3d.set_health(new_health, max_health)

func take_damage(damage: int, attacker: Node = null) -> void:
	if damageable:
		damageable.take_damage(damage, attacker)

func _on_player_died(attacker: Node) -> void:
	# Handle player death - respawn, game over, etc.
	var attacker_name: String
	if attacker:
		attacker_name = attacker.name
	else:
		attacker_name = "unknown"
	print("Player died! Killed by: ", attacker_name)

	# Sync death across network
	if multiplayer.is_server():
		rpc("sync_death", multiplayer.get_unique_id(), attacker_name)
	else:
		sync_death.rpc_id(1, multiplayer.get_unique_id(), attacker_name)

	# In test scenes, don't auto-respawn - let the scene handle it
	# Only auto-respawn in actual game scenes
	var current_scene = get_tree().current_scene
	if current_scene and not current_scene.scene_file_path.contains("test"):
		await get_tree().create_timer(3.0).timeout
		damageable.current_health = damageable.max_health
	else:
		print("Player died in test scene - no auto-respawn")

@rpc("any_peer", "call_local")
func sync_death(victim_peer_id: int, attacker_name: String) -> void:
	print("Player %d died, killed by %s" % [victim_peer_id, attacker_name])

@rpc("any_peer", "call_local")
func take_damage_remote(damage: int, attacker_peer_id: int) -> void:
	if not multiplayer.is_server():
		return

	# Server validates and applies damage
	# Note: For now, we don't track specific attacker objects in simplified networking
	damageable.take_damage(damage, null)

	# Sync health to all clients
	sync_health.rpc(damageable.current_health, damageable.max_health)

@rpc("authority", "call_local")
func sync_health(current_health: int, max_health: int) -> void:
	damageable.current_health = current_health
	damageable.max_health = max_health

func _on_attack_pressed() -> void:
	attack()

func _on_reload_pressed() -> void:
	reload_weapon()

func load_weapon(weapon_id: int) -> void:
	if not inventory or not hand_item:
		push_warning("Cannot load weapon: inventory or hand_item is null")
		return
	
	# Clean up existing weapon
	for child in hand_item.get_children():
		child.queue_free()
	current_weapon_instance = null
	
	# Wait for cleanup to complete
	await get_tree().process_frame
	
	var weapon_data = inventory.get_weapon_data(weapon_id)
	if weapon_data.is_empty():
		push_warning("Weapon data not found for weapon_id: " + str(weapon_id))
		return
	
	var scene_path = weapon_data.get("scene_path", "")
	if scene_path.is_empty():
		push_warning("Weapon scene_path is empty for weapon_id: " + str(weapon_id))
		return
	
	var weapon_scene = load(scene_path) as PackedScene
	if not weapon_scene:
		push_error("Failed to load weapon scene: " + scene_path)
		return
	
	current_weapon_instance = weapon_scene.instantiate()
	if not current_weapon_instance:
		push_error("Failed to instantiate weapon scene: " + scene_path)
		return
	
	hand_item.add_child(current_weapon_instance)
	await get_tree().process_frame

	# Add weapon script if it doesn't have one
	if not current_weapon_instance.has_method("play_attack_animation"):
		var weapon_script = preload("res://entites/weapons/weapon.gd")
		current_weapon_instance.set_script(weapon_script)

	# Initialize weapon with data and camera
	if current_weapon_instance.has_method("initialize_weapon_data"):
		current_weapon_instance.initialize_weapon_data(weapon_data)
	if current_weapon_instance.has_method("set_camera") and camera:
		current_weapon_instance.set_camera(camera)

	# Ensure weapon position is set correctly after initialization
	if current_weapon_instance.has_method("reset_to_base_position"):
		current_weapon_instance.reset_to_base_position()

	set_weapon_visibility(current_weapon_instance, true)
	hand_item.visible = true

func update_weapon_sway(delta: float) -> void:
	if not hand_item:
		return
	
	var movement_offset = Vector3.ZERO
	if current_movement_input.length() > 0:
		# Sway opposite to movement direction
		movement_offset.x = -current_movement_input.x * SWAY_AMOUNT
		movement_offset.z = -current_movement_input.y * SWAY_AMOUNT
	
	# Jump animation - weapon moves down when jumping, up when landing
	var jump_offset = Vector3.ZERO
	if not is_on_floor():
		# In air - weapon moves down
		jump_offset.y = -JUMP_SWAY_AMOUNT
	elif not was_on_floor:
		# Just landed - weapon moves up briefly
		jump_offset.y = JUMP_SWAY_AMOUNT * 0.5
	
	# Combine movement and jump offsets
	var target_offset = movement_offset + jump_offset
	
	# Smoothly interpolate to target offset
	var current_offset = hand_item.position - hand_item_base_position
	var new_offset = current_offset.lerp(target_offset, SWAY_SPEED * delta)
	
	hand_item.position = hand_item_base_position + new_offset

func attack() -> void:
	if current_weapon_instance and current_weapon_instance.has_method("shoot"):
		# Check if we need to auto-reload
		if current_weapon_instance.has_method("get_current_ammo") and current_weapon_instance.get_current_ammo() <= 0:
			reload_weapon()
			return

		current_weapon_instance.shoot()

func reload_weapon() -> void:
	if current_weapon_instance and current_weapon_instance.has_method("reload"):
		current_weapon_instance.reload()

func set_weapon_visibility(node: Node, should_be_visible: bool) -> void:
	if node is Node3D:
		(node as Node3D).visible = should_be_visible
	for child in node.get_children():
		set_weapon_visibility(child, should_be_visible)

## set_networking_manager
## Sets reference to networking manager for multiplayer synchronization
## @param manager: The NetworkingManager instance
func set_networking_manager(manager: Node) -> void:
	networking_manager = manager
	if networking_manager and networking_manager.has_signal("weapon_switched"):
		networking_manager.weapon_switched.connect(_on_remote_weapon_switched)
	if networking_manager and networking_manager.has_signal("player_damaged"):
		networking_manager.player_damaged.connect(_on_network_damage_received)

## _on_remote_weapon_switched
## Handles weapon switch events for remote players (not local player)
## @param remote_player_id: ID of the player who switched weapons
## @param weapon_id: ID of the weapon they switched to
func _on_remote_weapon_switched(remote_player_id: int, weapon_id: int) -> void:
	# Only handle weapon switches for other players, not ourselves
	# (Local player weapon switches are handled via state sync to avoid conflicts)
	if player_id == remote_player_id:
		return
	
	if not inventory:
		return
	
	# Find the slot that contains this weapon ID
	for slot in range(1, 4):  # Assuming 3 weapon slots (1, 2, 3)
		if inventory.get_weapon_id_in_slot(slot) == weapon_id:
			# Only switch if not already on this weapon
			if inventory.get_active_weapon_id() != weapon_id:
				inventory.switch_to_weapon(slot)
			break

## _on_network_damage_received
## Handles damage events received from the network (server-authoritative)
## @param damaged_player_id: ID of the player who was damaged
## @param damage_amount: Amount of damage taken
## @param attacker_id: ID of the attacking player
func _on_network_damage_received(damaged_player_id: int, damage_amount: int, attacker_id: int) -> void:
	# Only apply damage if this player instance matches the damaged player ID
	if player_id == damaged_player_id:
		take_damage(damage_amount)
		print("Player %d took %d damage from player %d" % [player_id, damage_amount, attacker_id])

## update_target_position
## Updates the target position/rotation for interpolation (called from network updates)
## @param new_position: Server-authoritative position
## @param new_rotation: Server-authoritative rotation
func update_target_position(new_position: Vector3, new_rotation: Vector3) -> void:
	if is_local:
		# For local players, check if server position differs significantly from local position
		var position_difference = global_position.distance_to(new_position)
		if has_received_position and position_difference > 0.1:  # More than 10cm difference
			# Server correction needed - smoothly reconcile
			target_position = new_position
			target_rotation = new_rotation
			needs_reconciliation = true
		# Always update rotation for local player (look direction should be authoritative)
		# new_rotation: (body_y_rotation, head_x_rotation, roll)
		rotation.y = -new_rotation.x  # Body rotation (horizontal turning) - inverted for correct direction
		if character_model:
			character_model.rotation.x = new_rotation.y  # Head rotation (vertical looking) - correct direction
	else:
		# For remote players, interpolate towards server position
		if not has_received_position:
			# First position update - snap to position
			global_position = new_position
			rotation.y = -new_rotation.x  # Body rotation (horizontal turning) - inverted for correct direction
			if character_model:
				character_model.rotation.x = new_rotation.y  # Head rotation (vertical looking) - correct direction
			has_received_position = true
		else:
			# Subsequent updates - set interpolation target
			target_position = new_position
			target_rotation = new_rotation

	last_position_update = Time.get_ticks_msec() / 1000.0

## interpolate_to_target
## Smoothly interpolates position and rotation towards server targets
## @param delta: Time delta for frame-rate independent interpolation
func interpolate_to_target(delta: float) -> void:
	# Only interpolate if we've received at least one position update
	if not has_received_position:
		return

	# Interpolate position
	var old_pos = global_position
	global_position = global_position.lerp(target_position, interpolation_speed * delta)

	# Debug: uncomment to see interpolation
	#if not is_local and global_position.distance_to(old_pos) > 0.1:
	#	print("Interpolating player ", player_id, " from ", old_pos, " to ", target_position, " (current: ", global_position, ")")

	# Interpolate rotation (horizontal rotation - body turning)
	var current_y_rotation = rotation.y
	var target_y_rotation = -target_rotation.x  # Body rotation is in x component, inverted for correct direction
	var angle_diff = fmod(target_y_rotation - current_y_rotation + PI, TAU) - PI
	rotation.y = current_y_rotation + angle_diff * interpolation_speed * delta

	# For vertical rotation (head/camera), interpolate smoothly
	if character_model:
		var current_head_rotation = character_model.rotation.x
		var target_head_rotation = target_rotation.y  # Head rotation is in y component, correct direction
		var head_angle_diff = fmod(target_head_rotation - current_head_rotation + PI, TAU) - PI
		character_model.rotation.x = current_head_rotation + head_angle_diff * interpolation_speed * delta
