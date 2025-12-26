extends Control
@onready var console = $console

func write(message: String) -> void:
	console.text += message + "\n"

func _ready() -> void:
	write("Hello, World!")
	dotest()

func dotest():
	write("test start")
	var res = NetworkingAdaptor.send_http_request("http://127.0.0.1:8080/test", [], HTTPClient.METHOD_GET, "")
	if res >= 0:
		write("test success")
	else:
		write("test failed")
