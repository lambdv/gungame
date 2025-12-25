extends Node

var weapon_ids: Array[int] = [1, 2, 3]  # Available weapon IDs
var active_weapon_index: int = 0  # Index in weapon_ids array
var weapons_data: Dictionary = {}  # Loaded from JSON
var is_data_loaded: bool = false  # Track if data is loaded

signal active_weapon_changed(weapon_id: int)
signal data_loaded

func _ready() -> void:
	load_weapons_data()
	is_data_loaded = true
	data_loaded.emit()

func load_weapons_data() -> void:
	var file = FileAccess.open("res://shared/data/weapons.json", FileAccess.READ)
	if not file:
		return
	
	var json_string = file.get_as_text()
	file.close()
	
	var json = JSON.new()
	if json.parse(json_string) != OK:
		return
	
	var data = json.data
	if not data.has("data"):
		return
	
	for weapon in data["data"]:
		weapons_data[int(weapon["id"])] = weapon

func get_active_weapon_id() -> int:
	if weapon_ids.is_empty():
		return -1
	return weapon_ids[active_weapon_index]

func get_weapon_data(weapon_id: int) -> Dictionary:
	return weapons_data.get(weapon_id, {})

func get_weapon_id_in_slot(slot: int) -> int:
	var index = slot - 1
	if index >= 0 and index < weapon_ids.size():
		return weapon_ids[index]
	return -1

func switch_to_weapon(slot: int) -> void:
	var index = slot - 1
	if index >= 0 and index < weapon_ids.size():
		active_weapon_index = index
		active_weapon_changed.emit(get_active_weapon_id())
