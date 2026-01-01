# GunGameOnline
An online Multiplayer First Person Shooter video game built with Godot and Rust.
> this is a prototype / proof of concept for a multiplayer FPS game and is not fully functional.

![GunGameOnline](./docs/screenshot.png)

## Features
- Multiplayer gameplay with lobby system and authorative server.
- Weapon Swapping and bullet amount management.
- Health and damage system.
- Real-time synchronization of player positions and rotations.
- Pause menu and disconnect functionality.

## Architecture
The frontend is built using Godot game engine which is a "view" into a player's representation of the game world and a "cache" of the player state on the server.
The server written in Rust is authorative for the game world and handles the logic for the game.

The server uses a ENETs UDP endpoint to handle packets for fast real-time gameplay such as player state synchronization or weapon firing.

HTTP endpoints are used for less speed critical operations such as lobby joining and creation.

Server state's state is a struct with a lock for thread safety when handling concurrent requests.

![System Architecture](./docs/architecture.png)
