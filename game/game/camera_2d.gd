extends Camera2D

# Zoom speed and limits
@export var zoom_speed: float = 0.1
@export var max_zoom: Vector2 = Vector2(100.0, 100.0)

# Reference to other nodes
@onready var board: Node2D = get_parent()
@onready var tilemap: TileMapLayer = board.find_child("TileMapLayer")


# Panning variable to avoid opening mines when dragging is finished
var _is_panning = false
# Current lower zoom limit (window size)
var min_zoom: Vector2 = Vector2(0.1, 0.1)

# Called when the node enters the scene tree for the first time.
func _ready() -> void:
	Log.set_log_level(0)
	# Change min size when window size changed
	get_tree().root.size_changed.connect(update_zoom_limit)
	# Set initial state
	board.board_generated.connect(_on_resize)

func _unhandled_input(event: InputEvent) -> void:
	# Zooming 
	# Case A: Mouse Wheel
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			#Log.debug("Zooming IN.")
			_zoom_camera(1 + zoom_speed)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			#Log.debug("Zooming OUT.")
			_zoom_camera(1 - zoom_speed)
	
	# Case B: Pinch
	elif event is InputEventMagnifyGesture:
		_zoom_camera(event.factor) # Factor is >1 (zoom in) or <1 (zoom out)
		get_viewport().set_input_as_handled()
	
	# Panning
	# Case A: Touch Drag
	if event is InputEventScreenDrag:
		Log.debug("Panning. (Touch)")
		_handle_pan(event.relative)
		_is_panning = true
		get_viewport().set_input_as_handled()

	# Case B: Mouse Drag
	# We check if Left Button is held down during motion
	elif event is InputEventMouseMotion and (event.button_mask & MOUSE_BUTTON_MASK_LEFT):
		Log.debug("Panning. (Mouse)")
		_handle_pan(event.relative)
		_is_panning = true
		get_viewport().set_input_as_handled()

	# Pan release cancellation
	elif event is InputEventScreenTouch or event.is_action_released("primary"):
		if event.is_released():
			if _is_panning:
				Log.debug("Canceling select release. (panning)")
				_is_panning = false
				# Swallow the release event so the board doesn't see it
				get_viewport().set_input_as_handled()

func _handle_pan(relative_motion: Vector2):
	position -= relative_motion * (1.0 / zoom.x)

func _zoom_camera(factor: float):
	var new_zoom = zoom * factor
	zoom = new_zoom.clamp(min_zoom, max_zoom)

func _on_resize():
	await get_tree().process_frame
	update_zoom_limit()
	center_camera_on_board()

func center_camera_on_board():
	# Calculate center in pixels
	var center_pos = ((tilemap.tile_set.tile_size) * Vector2i(board.cols, board.rows)) / 2.0
	position = center_pos

func update_zoom_limit():
	Log.info("Updating minimum zoom limit...")
	if not tilemap or not is_inside_tree(): 
		return
	
	# Get Board Size in Pixels
	var used_rect = tilemap.get_used_rect()
	var tile_size = tilemap.tile_set.tile_size
	var board_pixel_size = Vector2(
		used_rect.size.x * tile_size.x,
		used_rect.size.y * tile_size.y
	)
	Log.debug("Board Size:", board_pixel_size)
	
	# Check for empty board
	if board_pixel_size.x <= 0 or board_pixel_size.y <= 0:
		Log.error("Game not yet started.")
		return

	var viewport_size = get_viewport_rect().size
	Log.debug("Viewport Size:", viewport_size)
	if viewport_size.x <= 1 or viewport_size.y <= 1:
		Log.error("Game not yet started.")
		return

	# Calculate Ratio
	var min_ratio_x = viewport_size.x / board_pixel_size.x
	var min_ratio_y = viewport_size.y / board_pixel_size.y
	
	# Use 'min'; entire board is visible
	var calculated_min = min(min_ratio_x, min_ratio_y)
	
	min_zoom = Vector2(calculated_min, calculated_min)
	
	if min_zoom < Vector2(0.01, 0.01):
		Log.error("Minimum board size too low to display.")

	# This function is only called when window is resized, so immediately reset the zoom level
	zoom = min_zoom
	Log.info("Zoom limit updated to:", min_zoom)
