extends Control

@onready var layout_container = $PanelContainer/LayoutContainer
@onready var game_container = $PanelContainer/LayoutContainer/GameContainer
@onready var game_viewport = $PanelContainer/LayoutContainer/GameContainer/SubViewport
@onready var game_board = $PanelContainer/LayoutContainer/GameContainer/SubViewport/Board
@onready var hud_panel = $PanelContainer/LayoutContainer/HUD
@onready var option_container = $PanelContainer/LayoutContainer/HUD/OptionContainer
const button_class = preload("res://game/hud_button.tscn")

@export var hud_scale_ratio: float = 0.15

func _ready():
	get_tree().root.size_changed.connect(_on_resize)
	_on_resize()
	
	GameSettings.load_settings()
	_add_hud_options()


func _on_resize():
	var screen_size = get_viewport_rect().size
	var is_portrait = screen_size.y > screen_size.x
	_switch_orientation(is_portrait)
	_switch_hud(is_portrait)
	
	# Resize HUD children
	var small_dim = min(screen_size.x, screen_size.y)
	var new_btn_size = small_dim * hud_scale_ratio
	var size_vector = Vector2(new_btn_size, new_btn_size)
	for opn in option_container.get_children():
		if opn is Button:
			opn.custom_minimum_size = size_vector


func _switch_orientation(is_portrait: bool):
	# Save current children
	var children = layout_container.get_children()
	for child in children:
		layout_container.remove_child(child)
	
	# Replace the container with the correct type
	var new_container
	if is_portrait:
		new_container = VBoxContainer.new()
	else:
		new_container = HBoxContainer.new()
	
	# Configure the new container
	new_container.name = "LayoutContainer"
	new_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	
	# Swap the node in the scene tree
	layout_container.replace_by(new_container)
	layout_container = new_container # Update reference
	
	for child in children:
		layout_container.add_child(child)


func _switch_hud(is_portrait: bool):
	var new_opt_container
	if is_portrait:
		# HUD is at bottom (wide), buttons should flow horizontally
		new_opt_container = HBoxContainer.new()
		# Optional: Add spacing or alignment
		new_opt_container.alignment = BoxContainer.ALIGNMENT_CENTER
	else:
		# HUD is at side (tall), buttons should flow vertically
		new_opt_container = VBoxContainer.new()
		new_opt_container.alignment = BoxContainer.ALIGNMENT_CENTER

	# Swap the container while keeping the buttons
	var options = option_container.get_children()
	for btn in options:
		option_container.remove_child(btn)
	
	option_container.replace_by(new_opt_container)
	option_container = new_opt_container
	
	# Re-add options
	for btn in options:
		option_container.add_child(btn)


func _add_hud_options():
	# invert controls
	var button_invert_contr: TextureButton = button_class.instantiate()
	button_invert_contr.texture_normal = load("res://sprites/button/InvertControlsA.png")
	button_invert_contr.texture_pressed = load("res://sprites/button/InvertControlsB.png")
	button_invert_contr.toggle_mode = true
	button_invert_contr.button_pressed = GameSettings.inverted_controls
	button_invert_contr.toggled.connect(_invert_controls)
	option_container.add_child(button_invert_contr)
	
	# test
	var button_test: TextureButton = button_class.instantiate()
	option_container.add_child(button_test)
	var button_test1: TextureButton = button_class.instantiate()
	option_container.add_child(button_test1)


func _invert_controls(toggled_on: bool):
	GameSettings.inverted_controls = toggled_on
	GameSettings.save_settings()
