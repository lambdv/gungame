extends PanelContainer

@onready var lobby_code_label: Label = $HBoxContainer/InfoContainer/LobbyCode
@onready var player_count_label: Label = $HBoxContainer/InfoContainer/PlayerCount
@onready var scene_label: Label = $HBoxContainer/InfoContainer/Scene
@onready var join_button: Button = $HBoxContainer/JoinButton

signal join_pressed(lobby_code: String)

var lobby_data: Dictionary
var lobby_code: String

func _ready() -> void:
	print("Lobby item _ready called for lobby: ", lobby_code)
	# Ensure the panel can pass mouse events to children
	mouse_filter = Control.MOUSE_FILTER_PASS
	print("Panel rect: ", get_rect())
	if join_button:
		print("Button rect: ", join_button.get_rect())
		print("Button global position: ", join_button.global_position)

func _on_button_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		print("Button received mouse input event: ", event)

func setup(data: Dictionary) -> void:
	print("=== LOBBY ITEM SETUP START ===")
	print("Setting up lobby item with data: ", data)

	# Check if @onready variables are initialized
	print("Checking @onready variables...")
	print("lobby_code_label: ", lobby_code_label)
	print("player_count_label: ", player_count_label)
	print("scene_label: ", scene_label)
	print("join_button: ", join_button)

	# If any @onready variables are null, try to find them manually
	if lobby_code_label == null or player_count_label == null or scene_label == null or join_button == null:
		print("WARNING: Some @onready variables are null, trying to find them manually...")

		# Try to find the nodes manually in the scene hierarchy
		if lobby_code_label == null:
			lobby_code_label = find_label_by_name("LobbyCode")
		if player_count_label == null:
			player_count_label = find_label_by_name("PlayerCount")
		if scene_label == null:
			scene_label = find_label_by_name("Scene")
		if join_button == null:
			join_button = find_button_by_name("JoinButton")

		print("After manual search - lobby_code_label: ", lobby_code_label)
		print("After manual search - player_count_label: ", player_count_label)
		print("After manual search - scene_label: ", scene_label)
		print("After manual search - join_button: ", join_button)

	lobby_data = data
	lobby_code = data.get("code", "")
	print("Extracted lobby_code: '", lobby_code, "'")

	if lobby_code_label:
		var new_text = lobby_code if lobby_code != "" else "UNKNOWN"
		print("Setting lobby_code_label text to: '", new_text, "'")
		lobby_code_label.text = new_text
		print("After setting, lobby_code_label.text is: '", lobby_code_label.text, "'")
	else:
		print("ERROR: lobby_code_label is still null after deferring!")
		# Try to find it manually
		lobby_code_label = find_label_by_name("LobbyCode")
		if lobby_code_label:
			print("Found LobbyCode label manually")
			lobby_code_label.text = lobby_code if lobby_code != "" else "UNKNOWN"

	var player_count = data.get("player_count", 0)
	var max_players = data.get("max_players", 4)
	print("Player count: ", player_count, "/", max_players)

	if player_count_label:
		var new_player_text = "%d / %d players" % [player_count, max_players]
		print("Setting player_count_label text to: '", new_player_text, "'")
		player_count_label.text = new_player_text
	else:
		print("ERROR: player_count_label is null!")
		player_count_label = find_label_by_name("PlayerCount")
		if player_count_label:
			print("Found PlayerCount label manually")
			player_count_label.text = "%d / %d players" % [player_count, max_players]

	var scene = data.get("scene", "world")
	if scene_label:
		var new_scene_text = ""
		match scene:
			"world":
				new_scene_text = "World Map"
			_:
				new_scene_text = scene.capitalize()
		print("Setting scene_label text to: '", new_scene_text, "'")
		scene_label.text = new_scene_text
	else:
		print("ERROR: scene_label is null!")
		scene_label = find_label_by_name("Scene")
		if scene_label:
			print("Found Scene label manually")
			var new_scene_text = ""
			match scene:
				"world":
					new_scene_text = "World Map"
				_:
					new_scene_text = scene.capitalize()
			scene_label.text = new_scene_text

	# Disable join button if lobby is full
	if join_button:
		var is_full = player_count >= max_players
		join_button.disabled = is_full
		print("Setting up button for lobby ", data.get("code", "unknown"), " - disabled: ", join_button.disabled, " players: ", player_count, "/", max_players, " is_full: ", is_full)
		if join_button.disabled:
			join_button.text = "FULL"
		else:
			join_button.text = "JOIN"
		print("Button text set to: '", join_button.text, "'")

		# Connect button signal (only once per instance)
		if not join_button.pressed.is_connected(_on_join_pressed):
			print("Connecting join button signal for lobby: ", data.get("code", "unknown"))
			join_button.pressed.connect(_on_join_pressed)
			print("Button connected successfully")
		else:
			print("Join button signal already connected for lobby: ", data.get("code", "unknown"))
	else:
		print("ERROR: join_button is null!")
		join_button = find_button_by_name("JoinButton")
		if join_button:
			print("Found JoinButton manually")
			var is_full = player_count >= max_players
			join_button.disabled = is_full
			join_button.text = "FULL" if join_button.disabled else "JOIN"
			if not join_button.pressed.is_connected(_on_join_pressed):
				join_button.pressed.connect(_on_join_pressed)

	print("=== LOBBY ITEM SETUP END ===")

func find_label_by_name(node_name: String) -> Label:
	# Recursively search for a label with the given name
	return _find_node_by_name_and_type(self, node_name, "Label") as Label

func find_button_by_name(node_name: String) -> Button:
	# Recursively search for a button with the given name
	return _find_node_by_name_and_type(self, node_name, "Button") as Button

func _find_node_by_name_and_type(node: Node, node_name: String, node_type: String) -> Node:
	if node.name == node_name and node.get_class() == node_type:
		return node

	for child in node.get_children():
		var result = _find_node_by_name_and_type(child, node_name, node_type)
		if result:
			return result

	return null

func _on_join_pressed() -> void:
	print("=== JOIN BUTTON PRESSED ===")
	print("Join button pressed for lobby: ", lobby_code)
	print("Button disabled state: ", join_button.disabled)
	print("Button visible: ", join_button.visible)
	print("Button in tree: ", join_button.is_inside_tree())
	print("Lobby data: ", lobby_data)
	if join_button.disabled:
		print("ERROR: Button is disabled!")
		return
	print("Emitting join_pressed signal with code: ", lobby_code)
	join_pressed.emit(lobby_code)
	print("Signal emitted successfully")
