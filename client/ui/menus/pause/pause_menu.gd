extends Control

@onready var resume_button: Button = $CenterContainer/PanelContainer/VBoxContainer/ResumeButton
@onready var leave_button: Button = $CenterContainer/PanelContainer/VBoxContainer/LeaveButton

func _ready() -> void:
	# Connect button signals
	resume_button.connect("pressed", Callable(self, "_on_resume_pressed"))
	leave_button.connect("pressed", Callable(self, "_on_leave_pressed"))

	# Initially hide the pause menu
	visible = false

	# Connect to game state changes
	if GameStateManager:
		GameStateManager.paused.connect(_on_game_paused)
		GameStateManager.resumed.connect(_on_game_resumed)

func _on_resume_pressed() -> void:
	if GameStateManager:
		GameStateManager.resume()

func _on_leave_pressed() -> void:
	# Gracefully leave the game
	_leave_game()

func _on_game_paused() -> void:
	visible = true
	# Ensure mouse is visible for UI interaction
	if InputManager:
		InputManager.release_mouse()

func _on_game_resumed() -> void:
	visible = false
	# Recapture mouse for gameplay
	if InputManager:
		InputManager.capture_mouse()

func _leave_game() -> void:
	# Leave current lobby but keep HTTP connection alive for lobby browser
	if NetworkingManager:
		NetworkingManager.return_to_lobby_browser()

	# Change to lobby list scene instead of main menu
	get_tree().change_scene_to_file("res://ui/menus/lobbies/lobby_list.tscn")
