extends Camera2D

@export var zoom_speed: float = 0.1
@export var max_zoom: Vector2 = Vector2(100.0, 100.0)

@onready var board: Node2D = get_parent()
@onready var tilemap: TileMapLayer = board.cells_layer

var _is_panning := false
var min_zoom: Vector2 = Vector2(0.1, 0.1)

func _ready() -> void:
	get_tree().root.size_changed.connect(update_zoom_limit)
	board.board_generated.connect(_on_resize)

func _unhandled_input(event: InputEvent) -> void:
	# Zooming
	# Case A: Mouse Wheel
	if event is InputEventMouseButton:
		var zoom_event := event as InputEventMouseButton
		if zoom_event.button_index == MOUSE_BUTTON_WHEEL_UP:
			Log.verbose("Zooming IN.")
			_zoom_camera(1 + zoom_speed)
		elif zoom_event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			Log.verbose("Zooming OUT.")
			_zoom_camera(1 - zoom_speed)
	
	# Case B: Pinch
	elif event is InputEventMagnifyGesture:
		var zoom_event := event as InputEventMagnifyGesture
		_zoom_camera(zoom_event.factor) # Factor is >1 (zoom in) or <1 (zoom out)
		get_viewport().set_input_as_handled()
	
	# Panning
	# Case A: Touch Drag
	if event is InputEventScreenDrag:
		var pan_event := event as InputEventScreenDrag
		Log.verbose("Panning. (Touch)")
		_handle_pan(pan_event.relative)
		self._is_panning = true
		get_viewport().set_input_as_handled()

	# Case B: Mouse Drag
	# We check if Left Button is held down during motion
	elif event is InputEventMouseMotion:
		var pan_event := event as InputEventMouseMotion
		if (pan_event.button_mask & MOUSE_BUTTON_MASK_LEFT):
			Log.verbose("Panning. (Mouse)")
			_handle_pan(pan_event.relative)
			self._is_panning = true
			get_viewport().set_input_as_handled()

	# Pan release cancellation
	elif event is InputEventScreenTouch or event.is_action_released("primary"):
		if event.is_released():
			if self._is_panning:
				Log.verbose("Canceling select release. (panning)")
				self._is_panning = false
				# Consume the release event
				get_viewport().set_input_as_handled()

func _handle_pan(relative_motion: Vector2) -> void:
	self.position -= relative_motion * (1.0 / zoom.x)

func _zoom_camera(factor: float) -> void:
	var new_zoom := zoom * factor
	self.zoom = new_zoom.clamp(min_zoom, max_zoom)

func _on_resize() -> void:
	await get_tree().process_frame
	update_zoom_limit()
	center_camera_on_board()

func center_camera_on_board() -> void:
	var center_pos := ((tilemap.tile_set.tile_size) * Vector2i(board.width, board.height)) / 2.0
	self.position = center_pos

func update_zoom_limit() -> void:
	Log.debug("Updating minimum zoom limit...")
	if not self.tilemap or not is_inside_tree(): 
		Log.error("Attempt to update zoom limit for non-existent board.")
		return
	
	var used_rect := tilemap.get_used_rect()
	var tile_size := tilemap.tile_set.tile_size
	var board_pixel_size := Vector2(
		used_rect.size.x * tile_size.x,
		used_rect.size.y * tile_size.y
	)
	Log.info("Board Size: %v" % board_pixel_size)
	
	if board_pixel_size.x <= 0 or board_pixel_size.y <= 0:
		Log.error("Game not yet started.")
		return

	var viewport_size := get_viewport_rect().size
	Log.debug("Viewport Size: %v" % viewport_size)
	if viewport_size.x <= 1 or viewport_size.y <= 1:
		Log.error("Game not yet started.")
		return

	var min_ratio_x := viewport_size.x / board_pixel_size.x
	var min_ratio_y := viewport_size.y / board_pixel_size.y
	
	var calculated_min: float = min(min_ratio_x, min_ratio_y)
	self.min_zoom = Vector2(calculated_min, calculated_min)
	if self.min_zoom < Vector2(0.01, 0.01):
		Log.error("Minimum board size too low to display.")
	self.zoom = self.min_zoom
	Log.debug("Zoom limit updated to: %v" % min_zoom)
