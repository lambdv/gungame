extends Node

var client : ENetMultiplayerPeer

func _ready():
    client = ENetMultiplayerPeer.new()
    var err = client.create_client("127.0.0.1", 9000)
    if err != OK:
        print("Failed to connect")
        return
    multiplayer.multiplayer_peer = client
    print("Connected to server")

func _process(_delta):
    if client.get_connection_status() == MultiplayerPeer.CONNECTION_CONNECTED:
        # Unreliable movement update
        var pos_data = Vector3(10, 0, 0) # example player position
        rpc("update_position", pos_data)

func _on_shoot():
    # Reliable shooting event
    var target_id = 1
    rpc("player_shoot", target_id)

@rpc func update_position(_pos):
    # Called on server and other clients
    pass

@rpc func player_shoot(_target_id):
    # Called reliably
    pass