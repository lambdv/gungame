extends Node3D

signal reload_started
signal reload_finished

var animation_player: AnimationPlayer = null
var current_tween: Tween = null

# Weapon properties loaded from JSON
var weapon_data: Dictionary = {}
var damage: int = 0
var fire_rate: float = 1.0  # shots per second
var range: float = 100.0
var ammo: int = 0
var current_ammo: int = 0
var reload_time: float = 1.0

# Shooting state
var can_shoot: bool = true
var last_shot_time: float = 0.0

# Reloading state
var is_reloading: bool = false
var reload_timer: SceneTreeTimer = null

# Reference to player/camera for raycasting
var player_camera: Camera3D = null

func _ready() -> void:
	# Find AnimationPlayer in weapon scene
	animation_player = find_animation_player(self)

func find_animation_player(node: Node) -> AnimationPlayer:
	if node is AnimationPlayer:
		return node
	for child in node.get_children():
		var result = find_animation_player(child)
		if result:
			return result
	return null

func play_attack_animation() -> void:
	if animation_player:
		if animation_player.has_animation("attack"):
			# Force restart from beginning every time
			if animation_player.is_playing():
				animation_player.stop()
			animation_player.play("attack")
			animation_player.seek(0.0)
	else:
		# Simple transform animation if no AnimationPlayer
		play_simple_attack_animation()

func play_simple_attack_animation() -> void:
	# Simple recoil animation for weapons without AnimationPlayer
	# Stop any existing tween to allow rapid repeating
	if current_tween:
		current_tween.kill()

	current_tween = create_tween()
	var base_position = position
	var recoil_distance = 0.15  # Increased from 0.05 for more movement

	# Recoil backward (away from camera) - positive Z moves toward camera in Godot
	current_tween.tween_property(self, "position", base_position + Vector3(0, 0, recoil_distance), 0.1)
	current_tween.tween_property(self, "position", base_position, 0.1)
	current_tween.tween_callback(func(): current_tween = null)

func initialize_weapon_data(data: Dictionary) -> void:
	weapon_data = data
	damage = data.get("damage", 0)
	fire_rate = data.get("fire_rate", 1.0)
	range = data.get("range", 100.0)
	ammo = data.get("ammo", 0)
	current_ammo = ammo
	reload_time = data.get("reload_time", 1.0)

func set_camera(camera: Camera3D) -> void:
	player_camera = camera

func can_fire() -> bool:
	if not can_shoot:
		return false

	if is_reloading:
		return false

	if ammo > 0 and current_ammo <= 0:
		return false

	var current_time = Time.get_ticks_msec() / 1000.0
	var time_since_last_shot = current_time - last_shot_time
	return time_since_last_shot >= (1.0 / fire_rate)

func shoot() -> void:
	if not can_fire():
		return

	if not player_camera:
		push_error("Weapon has no camera reference for raycasting")
		return

	# Update ammo
	if ammo > 0:
		current_ammo -= 1

	# Update shot timing
	last_shot_time = Time.get_ticks_msec() / 1000.0

	# Perform raycast
	var ray_origin = player_camera.global_position
	var ray_direction = -player_camera.global_transform.basis.z.normalized()

	var space_state = get_world_3d().direct_space_state
	var ray_query = PhysicsRayQueryParameters3D.new()
	ray_query.from = ray_origin
	ray_query.to = ray_origin + ray_direction * range
	ray_query.collision_mask = 0xFFFFFFFF  # Hit everything

	var ray_result = space_state.intersect_ray(ray_query)

	if ray_result:
		# Hit something - check if it's damageable
		var hit_object = ray_result.collider
		if hit_object and hit_object.has_method("take_damage"):
			var attacker = get_parent().get_parent()  # Player is grandparent

			# Check if hit object supports remote damage (server-authoritative)
			if hit_object.has_method("take_damage_remote"):
				# Use server-authoritative damage via RPC
				hit_object.take_damage_remote(damage, multiplayer.get_unique_id())
			else:
				# Local damage for non-networked objects
				hit_object.take_damage(damage, attacker)

	# Play attack animation
	play_attack_animation()

func get_current_ammo() -> int:
	return current_ammo

func get_max_ammo() -> int:
	return ammo

func can_reload() -> bool:
	# Can reload if we have limited ammo, are not at max, and not already reloading
	return ammo > 0 and current_ammo < ammo and not is_reloading

func reload() -> void:
	if not can_reload():
		return

	is_reloading = true
	reload_started.emit()

	# Play reload animation if available
	if animation_player and animation_player.has_animation("reload"):
		animation_player.play("reload")
		animation_player.seek(0.0)

		# Wait for animation to complete or use timer as fallback
		if animation_player.current_animation == "reload":
			var animation_length = animation_player.get_animation("reload").length
			reload_timer = get_tree().create_timer(max(reload_time, animation_length))
		else:
			reload_timer = get_tree().create_timer(reload_time)
	else:
		# No animation, just use timer
		reload_timer = get_tree().create_timer(reload_time)

	reload_timer.timeout.connect(_on_reload_finished)

func _on_reload_finished() -> void:
	# Refill ammo
	current_ammo = ammo
	is_reloading = false
	can_shoot = true
	reload_finished.emit()

	# Clean up timer reference (SceneTreeTimer is automatically freed)
	reload_timer = null

func is_weapon_reloading() -> bool:
	return is_reloading

## set_current_ammo
## Externally sets current ammo (used for server synchronization)
## @param ammo: New ammo count
func set_current_ammo(ammo: int) -> void:
	current_ammo = ammo

## set_max_ammo
## Externally sets max ammo (used for server synchronization)
## @param ammo: New max ammo count
func set_max_ammo(ammo: int) -> void:
	ammo = ammo

## set_reloading_state
## Externally sets reload state (used for server synchronization)
## @param reloading: Whether weapon should be reloading
func set_reloading_state(reloading: bool) -> void:
	is_reloading = reloading
	can_shoot = not reloading
