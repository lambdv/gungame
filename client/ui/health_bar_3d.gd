extends Node3D

var health_bar: ProgressBar = null
var sub_viewport: SubViewport = null
var mesh_instance: MeshInstance3D = null

var target_node: Node3D = null
var offset: Vector3 = Vector3(0, 2.2, 0)  # Position above head

func _ready() -> void:
	_create_health_bar()

func _create_health_bar() -> void:
	# Create SubViewport
	sub_viewport = SubViewport.new()
	sub_viewport.size = Vector2i(200, 40)
	sub_viewport.transparent_bg = true
	add_child(sub_viewport)

	# Create ProgressBar inside viewport
	health_bar = ProgressBar.new()
	health_bar.size = Vector2(200, 40)
	health_bar.position = Vector2(0, 0)  # Position at top-left of viewport
	health_bar.max_value = 100.0
	health_bar.value = 100.0
	health_bar.show_percentage = false

	# Style the progress bar with red colors
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.2, 0.2, 0.8)  # Dark gray background
	style.border_color = Color(0, 0, 0, 1)  # Black border
	style.border_width_left = 2
	style.border_width_right = 2
	style.border_width_top = 2
	style.border_width_bottom = 2
	style.set_corner_radius_all(4)

	var fill_style = StyleBoxFlat.new()
	fill_style.bg_color = Color(1, 0, 0, 1)  # Bright red fill
	fill_style.set_corner_radius_all(4)

	health_bar.add_theme_stylebox_override("background", style)
	health_bar.add_theme_stylebox_override("fill", fill_style)

	sub_viewport.add_child(health_bar)

	# Create QuadMesh for 3D display
	var quad_mesh = QuadMesh.new()
	quad_mesh.size = Vector2(1.5, 0.3)  # Make it larger for better visibility

	mesh_instance = MeshInstance3D.new()
	mesh_instance.mesh = quad_mesh

	# Setup material with viewport texture
	var viewport_texture = sub_viewport.get_texture()
	var material = StandardMaterial3D.new()
	material.albedo_texture = viewport_texture
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.no_depth_test = true
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	mesh_instance.material_override = material

	add_child(mesh_instance)

func _process(_delta: float) -> void:
	if target_node and mesh_instance:
		global_position = target_node.global_position + offset
		# Make health bar face camera
		var camera = get_viewport().get_camera_3d()
		if camera:
			look_at(camera.global_position, Vector3.UP)

func set_health(health: int, max_health: int) -> void:
	if health_bar:
		health_bar.max_value = max_health
		health_bar.value = health

func set_target(target: Node3D) -> void:
	target_node = target
