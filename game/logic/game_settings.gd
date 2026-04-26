extends Node

# Define your global options with default values
var sound_volume: float
var is_fullscreen: bool
var inverted_controls: bool

const SAVE_PATH = "user://settings.cfg"

func _ready():
	load_settings()

func save_settings():
	var config = ConfigFile.new()
	# "Section" can be anything, e.g., "Audio", "Game", "Video"
	config.set_value("Audio", "volume", sound_volume)
	config.set_value("Video", "fullscreen", is_fullscreen)
	config.set_value("Gameplay", "invert_controls", inverted_controls)
	# Save to user:// (guaranteed writable folder)
	config.save(SAVE_PATH)

func load_settings():
	var config = ConfigFile.new()
	var error = config.load(SAVE_PATH)
	
	if error == OK:
		# Load values, providing a default if the key is missing
		sound_volume = config.get_value("Audio", "volume", 1.0)
		is_fullscreen = config.get_value("Video", "fullscreen", false)
		inverted_controls = config.get_value("Gameplay", "invert_controls", false)
		
		# Apply the settings immediately (optional but recommended)
		_apply_settings()

func _apply_settings():
	pass
