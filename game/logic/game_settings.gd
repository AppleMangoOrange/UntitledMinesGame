extends Node

var audio: Dictionary[String, Variant] = {
	"master": 1.0,
	"sound": 1.0,
	"music": 1.0,
}

var video: Dictionary[String, Variant] = {
	"fullscreen": false,
	"gui_scale": 1.0,
}

var game: Dictionary[String, Variant] = {
	"invert_controls": false,
	"width": 10,
	"height": 10,
	"num_mines": 20,
	"solvable": true,
}

var sections: Dictionary[String, Dictionary] = {
	"Audio": audio,
	"Video": video,
	"Gameplay": game,
}

const SAVE_PATH = "user://settings.cfg"

func _ready() -> void:
	load_settings()

func save_settings() -> void:
	var config: ConfigFile = ConfigFile.new()
	for section in self.sections:
		for setting in self.sections[section]:
			config.set_value(section, setting, self.sections[section][setting])
	config.save(SAVE_PATH)

func load_settings() -> void:
	var config: ConfigFile = ConfigFile.new()
	var error: Error = config.load(SAVE_PATH)
	if error != OK:
		Log.error("Failed to load user settings.")
		return
	
	for section in self.sections:
		for setting in self.sections[section]:
			self.sections[section][setting] = config.get_value(
				section, setting, self.sections[section][setting]
			)
	Log.info("Loaded user settings.")
	_apply_settings()
	

func _apply_settings() -> void:
	# TODO
	pass
