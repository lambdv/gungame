extends Control

@onready var health_label: Label = $Label
@onready var ammo_label: Label = $Label2
@onready var weapon_label: Label = $Label3

var player: Node = null
var update_timer: Timer = null

func _ready() -> void:
	# Create update timer for ammo display
	update_timer = Timer.new()
	update_timer.wait_time = 0.1  # Update 10 times per second
	update_timer.timeout.connect(_update_display)
	add_child(update_timer)
	update_timer.start()

	# Find the local player
	await get_tree().process_frame
	_find_local_player()

func _find_local_player() -> void:
	var players = get_tree().get_nodes_in_group("players")
	for p in players:
		if p.is_local:
			player = p
			_connect_player_signals()
			_update_display()
			break

func _connect_player_signals() -> void:
	if player and player.damageable:
		player.damageable.health_changed.connect(_on_player_health_changed)
	if player.inventory:
		player.inventory.active_weapon_changed.connect(_on_weapon_changed)

	# Connect to weapon reload signals if available
	if player.current_weapon_instance:
		_connect_weapon_signals(player.current_weapon_instance)

func _on_player_health_changed(new_health: int, max_health: int) -> void:
	_update_health_display(new_health, max_health)

func _on_weapon_changed(weapon_id: int) -> void:
	_update_weapon_display(weapon_id)
	# Reconnect weapon signals
	if player.current_weapon_instance:
		_connect_weapon_signals(player.current_weapon_instance)

func _connect_weapon_signals(weapon: Node) -> void:
	# Disconnect previous signals to avoid duplicates
	if weapon.has_signal("reload_started"):
		if not weapon.reload_started.is_connected(_on_weapon_reload_started):
			weapon.reload_started.connect(_on_weapon_reload_started)
	if weapon.has_signal("reload_finished"):
		if not weapon.reload_finished.is_connected(_on_weapon_reload_finished):
			weapon.reload_finished.connect(_on_weapon_reload_finished)

func _on_weapon_reload_started() -> void:
	_update_display()

func _on_weapon_reload_finished() -> void:
	_update_display()

func _update_display() -> void:
	if not player:
		return

	# Update health
	if player.damageable:
		_update_health_display(player.damageable.current_health, player.damageable.max_health)

	# Update weapon info
	var weapon_id = player.inventory.get_active_weapon_id() if player.inventory else -1
	_update_weapon_display(weapon_id)

func _update_health_display(current_health: int, max_health: int) -> void:
	if health_label:
		health_label.text = "HP: %d/%d" % [current_health, max_health]

func _update_weapon_display(weapon_id: int) -> void:
	if not player or not player.inventory:
		return

	var weapon_data = player.inventory.get_weapon_data(weapon_id)
	if weapon_data.is_empty():
		if weapon_label:
			weapon_label.text = "No weapon"
		if ammo_label:
			ammo_label.text = "Ammo: --"
		return

	# Update weapon name with reload status
	var weapon_name = weapon_data.get("name", "Unknown")
	if player.current_weapon_instance and player.current_weapon_instance.has_method("is_weapon_reloading"):
		if player.current_weapon_instance.is_weapon_reloading():
			weapon_name += " (RELOADING)"

	if weapon_label:
		weapon_label.text = weapon_name

	# Update ammo
	var current_ammo = 0
	var max_ammo = weapon_data.get("ammo", 0)

	if player.current_weapon_instance and player.current_weapon_instance.has_method("get_current_ammo"):
		current_ammo = player.current_weapon_instance.get_current_ammo()

	if ammo_label:
		if max_ammo > 0:
			ammo_label.text = "Ammo: %d/%d" % [current_ammo, max_ammo]
		else:
			ammo_label.text = "Ammo: ∞"
