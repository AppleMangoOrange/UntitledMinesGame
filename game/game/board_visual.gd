extends Node2D

signal board_generated()
signal game_started()
signal game_ended()

# Tile atlas coordinates for different cell states
const SPRITE_COORDS := Vector2i(0, 0)
const MODULE_NAME = "BoardInterface"

@onready var cells_layer: TileMapLayer = $TileMapLayer
@onready var camera: Camera2D = $Camera2D
var game_ongoing = false;

enum CellTiles {
	ZERO = 0,
	ONE,
	TWO,
	THREE,
	FOUR,
	FIVE,
	SIX,
	SEVEN,
	EIGHT,
	HIDDEN = 9,
	FLAGGED = 10,
	FLAGGED_WRONG = 11,
	MINE_EXPLODED = 12,
	MINE_NOT_FOUND = 13,
}

# Preload logic
var game: MinesCore
var gen_seed: int
var width: int
var height: int
var num_mines: int
var _solvable: bool # TODO


func _ready() -> void:
	Log.add_module(MODULE_NAME, Log.DEBUG)
	self.camera.board = self
	self.camera.tilemap = cells_layer
	
	GameSettings.game["width"] = 5
	GameSettings.game["height"] = 4
	GameSettings.game["num_mines"] = 10
	
	init(GameSettings.game["width"], GameSettings.game["height"])
	create(123, GameSettings.game["num_mines"], true, GameSettings.game["width"] / 2, GameSettings.game["height"] / 2)

func init(width: int, height: int):
	self.width = width
	self.height = height
	cells_layer.clear()
	for y in range(height):
		for x in range(width):
			var coords := Vector2i(x, y)
			cells_layer.set_cell(coords, CellTiles.HIDDEN, SPRITE_COORDS)
	board_generated.emit()

func create(seed: int, num_mines: int, _solvable: bool, start_x: int, start_y: int) -> void:
	self.gen_seed = seed
	self.num_mines = num_mines
	self._solvable = _solvable
	Log.debug("Generating solvable game of size %dx%d" % [self.width, self.height], MODULE_NAME)
	self.game = MinesCore.from_params(
		self.gen_seed,
		self.width,
		self.height,
		self.num_mines,
		self._solvable,
		start_x, start_y
	)
	game.game_lost.connect(_on_game_lost)
	game.game_won.connect(_on_game_won)
	game.cell_updated.connect(_on_cell_updated)
	for y in range(height):
		for x in range(width):
			var coords := Vector2i(x, y)
			_update_cell_visual(coords)
	self.game_ongoing = true;
	self.game_started.emit()

func _unhandled_input(event: InputEvent) -> void:
	if not game:
		Log.error("Uninitialised game recieved input.", MODULE_NAME)
		return
	
	if event.is_action_released("primary") or event.is_action_released("secondary"):
		var local_pos = cells_layer.get_local_mouse_position()
		var map_coords = cells_layer.local_to_map(local_pos)
		Log.debug("Selecting %v." % map_coords, MODULE_NAME)
		get_viewport().set_input_as_handled()
		_handle_chord(map_coords)
		
		if GameSettings.game["invert_controls"] == true:
			if event.is_action("primary"):
				_handle_reveal(map_coords)
			elif event.is_action("secondary"):
				_handle_flag(map_coords)
		else:
			if event.is_action("primary"):
				_handle_flag(map_coords)
			elif event.is_action("secondary"):
				_handle_reveal(map_coords)

func _handle_reveal(coords: Vector2i) -> void:
	Log.verbose("Revealing cell at %v." % coords, MODULE_NAME)
	var _hint = game.open(coords.x, coords.y)

func _handle_flag(coords: Vector2i) -> void:
	Log.verbose("Flagging cell at %v." % coords, MODULE_NAME)
	game.flag(coords.x, coords.y)

func _handle_chord(coords: Vector2i) -> bool:
	return game.chord(coords.x, coords.y)

func _update_cell_visual(coords: Vector2i) -> void:
	Log.verbose("Updating cell at %v." % coords, MODULE_NAME)
	Log.debug("Current opened cells: %d" % self.game.revealed)
	var value: int = game.sprite(coords.x, coords.y)
	cells_layer.set_cell(coords, value, SPRITE_COORDS)

## Signals

func _on_cell_updated(x: int, y: int) -> void:
	_update_cell_visual(Vector2i(x, y))

func _on_game_lost(x: int, y: int) -> void:
	Log.error("Game Over. Hit mine at (%d, %d)" % [x, y], MODULE_NAME)
	self.game_ended.emit()
	self.game_ongoing = false
	#for y in range(rows):
		#for x in range(cols):
			#_update_cell_visual(x, y)

func _on_game_won(x: int, y: int) -> void:
	print("Received game won signal.")
	Log.info("Congratulations, you won.", MODULE_NAME)
	self.game_ended.emit()
	self.game_ongoing = false

### external control
#
#func reset_game() -> void:
	#if game:
		#game.Reset()
		#_generate_board()
#
#
#func get_remaining_mines() -> int:
	#if game:
		#return game.GetRemainingMines()
	#return 0
#
