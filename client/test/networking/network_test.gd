extends Control
@onready var console = $console

func write(message: String) -> void:
	console.text += message + "\n"

func _ready() -> void:
	write("Hello, World!")
	dotest()

func dotest():
	write("test start")
	# Note: This test uses NetworkingManager directly now
	# For testing, you can call NetworkingManager methods
	write("NetworkingManager available: " + str(NetworkingManager != null))
	if NetworkingManager:
		write("test success - NetworkingManager loaded")
	else:
		write("test failed - NetworkingManager not found")
