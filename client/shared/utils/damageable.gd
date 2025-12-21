extends Node
class_name Damageable

# Signals
signal health_changed(new_health: int, max_health: int)
signal damage_taken(damage: int, attacker: Node)
signal died(attacker: Node)

# Health properties
@export var max_health: int = 100
@export var current_health: int = 100:
	set(value):
		current_health = clamp(value, 0, max_health)
		health_changed.emit(current_health, max_health)
		if current_health <= 0:
			died.emit(get_last_attacker())

var last_attacker: Node = null

func _ready() -> void:
	# Initialize health
	current_health = max_health

func take_damage(damage: int, attacker: Node = null) -> void:
	if current_health <= 0:
		return

	last_attacker = attacker
	var old_health = current_health
	current_health -= damage

	damage_taken.emit(damage, attacker)

	if current_health <= 0:
		died.emit(attacker)

func heal(amount: int) -> void:
	current_health = min(current_health + amount, max_health)

func is_alive() -> bool:
	return current_health > 0

func get_health_percentage() -> float:
	return float(current_health) / float(max_health)

func get_last_attacker() -> Node:
	return last_attacker
