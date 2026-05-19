class_name Board
extends Node2D

## Board configuration
@export var rows: int = 9
@export var cols: int = 18
@export var mine_count: int = 40

# Signals
signal board_generated()

# Tile atlas coordinates for different cell states
const HIDDEN_ATLAS := Vector2i(0, 0)

# Source IDs in your tileset (adjust based on your tileset configuration)
const SOURCE_HIDDEN := 9     # Hidden cell sprite
const SOURCE_FLAG := 10      # Flag sprite
const SOURCE_MINE := 12      # Mine sprite (revealed on loss)
const SOURCE_WRONG := 11     # Wrong flag sprite
# Numbers 0-8 would use source IDs 0-8

@onready var cells_layer: TileMapLayer = find_child("TileMapLayer")

# Preload logic
var game: MinesCore


func _ready() -> void:
	game = MinesCore.new()
	add_child(game)
	game.test_rust_connection()
	#_initialise_game()
	#_generate_board()
	board_generated.emit()

#func _initialise_game() -> void:
	#
	## Connect signals
	#game.connect("CellRevealed", _on_cell_revealed)
	#game.connect("CellFlagged", _on_cell_flagged)
	#game.connect("GameLost", _on_game_lost)
	#game.connect("GameWon", _on_game_won)
	#game.connect("GameStarted", _on_game_started)
	#
	## Start a new game
	#Log.debug("Generating solvable game")
	#game.NewGame(cols, rows, mine_count)
#
#
#func _unhandled_input(event: InputEvent) -> void:
	#if not game:
		#return
	#
	#if event.is_action_released("primary") or event.is_action_released("secondary"):
		#Log.debug("Selecting.")
		#var local_pos = cells_layer.get_local_mouse_position()
		#var map_coords = cells_layer.local_to_map(local_pos)
		#get_viewport().set_input_as_handled()
		## Always check for chord click
		#_handle_chord(map_coords)
		#
		##if map_coords.x >= 0 and map_coords.x < cols and map_coords.y >= 0 and map_coords.y < rows:
		#if not GameSettings.inverted_controls:
			#if event.is_action("primary"):
				#_handle_reveal(map_coords)
			#elif event.is_action("secondary"):
				#_handle_flag(map_coords)
		#else: # Inverted controls enabled
			#if event.is_action("primary"):
				#_handle_flag(map_coords)
			#elif event.is_action("secondary"):
				#_handle_reveal(map_coords)
#
#
#func _handle_reveal(coords: Vector2i) -> void:
	#if game.IsPlaying or not game.IsStarted:
		#game.Reveal(coords.x, coords.y)
#
#
#func _handle_flag(coords: Vector2i) -> void:
	#if game.IsPlaying or not game.IsStarted:
		#game.ToggleFlag(coords.x, coords.y)
#
#
#func _handle_chord(coords: Vector2i) -> void:
	#if game.IsPlaying:
		#game.Chord(coords.x, coords.y)
#
#
#func _generate_board() -> void:
	#cells_layer.clear()
	#for y in range(rows):
		#for x in range(cols):
			#var coords := Vector2i(x, y)
			#cells_layer.set_cell(coords, SOURCE_HIDDEN, HIDDEN_ATLAS)
#
#
#func _update_cell_visual(x: int, y: int) -> void:
	#var coords := Vector2i(x, y)
	#var value: int = game.GetCell(x, y)
	#
	#match value:
		#-2:  # CELL_UNKNOWN
			#cells_layer.set_cell(coords, SOURCE_HIDDEN, HIDDEN_ATLAS)
		#-1:  # CELL_FLAGGED
			#cells_layer.set_cell(coords, SOURCE_FLAG, HIDDEN_ATLAS)
		#64:  # CELL_MINE_REVEALED
			#cells_layer.set_cell(coords, SOURCE_MINE, HIDDEN_ATLAS)
		#65:  # CELL_MINE_HIT
			#cells_layer.set_cell(coords, SOURCE_MINE, HIDDEN_ATLAS)  # Could use different visual
		#66:  # CELL_WRONG_FLAG
			#cells_layer.set_cell(coords, SOURCE_WRONG, HIDDEN_ATLAS)
		#_:
			## Numbers 0-8 - use the value as source ID
			#if value >= 0 and value <= 8:
				#cells_layer.set_cell(coords, value, HIDDEN_ATLAS)
#
#
## Signal handlers
#
#func _on_cell_revealed(x: int, y: int, _value: int) -> void:
	#_update_cell_visual(x, y)
#
#
#func _on_cell_flagged(x: int, y: int, _is_flagged: bool) -> void:
	#_update_cell_visual(x, y)
#
#
#func _on_game_lost(mine_x: int, mine_y: int) -> void:
	#Log.info("Game Over! Hit mine at (", mine_x, ", ", mine_y, ")")
	## Update all cells to show final state
	#for y in range(rows):
		#for x in range(cols):
			#_update_cell_visual(x, y)
#
#
#func _on_game_won() -> void:
	#Log.info("Congratulations! You won!")
#
#
#func _on_game_started() -> void:
	#Log.debug("New game started:", cols, "C x", rows, "R with", mine_count, "mines")
	#_generate_board()
#
#
### Public API for external control
#
#func new_game(width: int = -1, height: int = -1, mines: int = -1) -> void:
	#if width > 0:
		#cols = width
	#if height > 0:
		#rows = height
	#if mines > 0:
		#mine_count = mines
	#
	#if game:
		#game.NewGame(cols, rows, mine_count)
		#_generate_board()
#
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
#
#func is_game_over() -> bool:
	#if game:
		#return game.IsDead or game.HasWon
	#return false
