extends CharacterBody3D

const SPEED = 8.0 # movement player positional speed
const JUMP_VELOCITY = 4.5 # jump velocity
const SENSITIVITY = 0.003 # sensitivity of the mouse
const MAX_VERTICAL_ROTATION = deg_to_rad(89)  # Limit to 89 degrees to prevent gimbal lock

const SWAY_AMOUNT = 0.02  # How far the weapon sways
const SWAY_SPEED = 8.0  # How fast the sway animation is
const JUMP_SWAY_AMOUNT = 0.05  # How far the weapon moves when jumping
const JUMP_SWAY_SPEED = 10.0  # How fast the jump animation is

@onready var camera_rig: Node3D = $CameraRig
@onready var head: Node3D = $CameraRig/Head
@onready var camera: Camera3D = $CameraRig/Head/Camera3D
@onready var hand_item: Node3D = $CameraRig/Head/Camera3D/HandItem
@onready var character_model: Node3D = get_node_or_null("rachel__black_heart_lovell_v5vrm")

var is_local := true
var current_movement_input := Vector2.ZERO
var current_mouse_motion := Vector2.ZERO

var inventory: Node = null
var damageable: Node = null
var health_bar_3d: Node3D = null
var current_weapon_instance: Node3D = null
var hand_item_base_position: Vector3
var was_on_floor: bool = true
# Attack cooldown removed - now handled by weapon fire rate

func _ready() -> void:
	# Add to players group for HUD lookup
	add_to_group("players")

	# Initialize damageable component
	damageable = preload("res://shared/utils/damageable.gd").new()
	add_child(damageable)
	damageable.health_changed.connect(_on_health_changed)
	damageable.died.connect(_on_player_died)

	# Initialize 3D health bar
	health_bar_3d = preload("res://ui/health_bar_3d.gd").new()
	add_child(health_bar_3d)
	health_bar_3d.set_target(self)
	health_bar_3d.set_health(damageable.current_health, damageable.max_health)
	health_bar_3d.visible = not is_local  # Set initial visibility based on local status

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
	elif was_local:
		# Disconnect inputs when becoming non-local
		InputManager.movement_input_changed.disconnect(_on_movement_input_changed)
		InputManager.jump_just_pressed_changed.disconnect(_on_jump_just_pressed)
		InputManager.mouse_motion_changed.disconnect(_on_mouse_motion_changed)
		InputManager.weapon_switch_requested.disconnect(_on_weapon_switch_requested)
		InputManager.attack_pressed.disconnect(_on_attack_pressed)
		InputManager.reload_pressed.disconnect(_on_reload_pressed)

	if not is_local and camera:
		camera.current = false
	if is_local and character_model:
		character_model.visible = false
	elif not is_local and character_model:
		character_model.visible = true

	# Show/hide health bar based on local status
	if health_bar_3d:
		health_bar_3d.visible = not is_local

func _physics_process(delta: float) -> void:
	if not is_local:
		return

	# Add the gravity.
	if not is_on_floor():
		velocity += get_gravity() * delta
	# Handle movement
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
	# Use camera_rig's basis (horizontal rotation only) for movement direction
	var horizontal_basis = camera_rig.global_transform.basis
	
	var direction = (horizontal_basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	if direction:
		velocity.x = direction.x * SPEED
		velocity.z = direction.z * SPEED
	else:
		velocity.x = move_toward(velocity.x, 0, SPEED)
		velocity.z = move_toward(velocity.z, 0, SPEED)

func look():
	if current_mouse_motion != Vector2.ZERO:
		camera_rig.rotate_y(-current_mouse_motion.x * SENSITIVITY) # Horizontal rotation
		# Vertical rotation
		var new_rotation_x = head.rotation.x - current_mouse_motion.y * SENSITIVITY 
		head.rotation.x = clamp(new_rotation_x, -MAX_VERTICAL_ROTATION, MAX_VERTICAL_ROTATION)

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
	if inventory:
		inventory.switch_to_weapon(slot)

func _on_active_weapon_changed(weapon_id: int) -> void:
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

	# For now, just reset health after a delay
	await get_tree().create_timer(3.0).timeout
	damageable.current_health = damageable.max_health

@rpc("any_peer", "call_local")
func sync_death(victim_peer_id: int, attacker_name: String) -> void:
	print("Player %d died, killed by %s" % [victim_peer_id, attacker_name])

@rpc("any_peer", "call_local")
func take_damage_remote(damage: int, attacker_peer_id: int) -> void:
	if not multiplayer.is_server():
		return

	# Server validates and applies damage
	var attacker = MultiplayerManager.get_remote_player(attacker_peer_id)
	damageable.take_damage(damage, attacker)

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
		return
	
	for child in hand_item.get_children():
		child.queue_free()
	current_weapon_instance = null
	
	var weapon_data = inventory.get_weapon_data(weapon_id)
	if weapon_data.is_empty():
		return
	
	var scene_path = weapon_data.get("scene_path", "")
	if scene_path.is_empty():
		return
	
	var weapon_scene = load(scene_path) as PackedScene
	if not weapon_scene:
		return
	
	current_weapon_instance = weapon_scene.instantiate()
	if current_weapon_instance:
		hand_item.add_child(current_weapon_instance)
		await get_tree().process_frame

		# Add weapon script if it doesn't have one
		if not current_weapon_instance.has_method("play_attack_animation"):
			var weapon_script = preload("res://entites/weapons/weapon.gd")
			current_weapon_instance.set_script(weapon_script)

		# Initialize weapon with data and camera
		if current_weapon_instance.has_method("initialize_weapon_data"):
			current_weapon_instance.initialize_weapon_data(weapon_data)
		if current_weapon_instance.has_method("set_camera"):
			current_weapon_instance.set_camera(camera)

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
