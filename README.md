# Untitled Minesweeper Game

**A cross-platform Minesweeper game aiming to have _all_ the features.**

Made using the Godot game engine.

## Feature List:
- [x] Guaranteed-solvable games
- [ ] Games with mine density > 0.5
- [ ] Puzzle levels isolating aspects to teach advanced Minesweeper tactics
- [ ] Anti-Guess: The game automatically detects when the player is guessing instead of solving, losing the game.
    - Reference: [Chocolate Sweeper](https://nyahoon.com/products/chocolate-sweeper)
    - Reference: [Kaboom](https://github.com/pwmarcz/kaboom)
- [ ] Expert mode: `?`. Games required to be solvable using the given information.
- [ ] UI: Dark mode / Themes
- [ ] Real-time board generation (<= 1s)
    - [ ] idea: Priority queue of constraints
- [ ] Infinite minesweeper: Just set the density and solve forever
- Settings:
    - [x] Chord mode to uncover cells which have satisfied the mines count
    - [ ] Automatically uncover cells which have satisfied the mines count (without chord mode)
    - [ ] Chord mode to place flags on cells which have satisfied the cleared cells count
    - [ ] Automatically place flags on cells which have satisfied the cleared cells count (without chord mode)
  
### Guaranteed-solvable games

The game includes a C# port of **Simon Tatham's Portable Puzzle Collection**'s [`mines.c`](https://git.tartarus.org/?p=simon/puzzles.git;a=blob;f=mines.c;h=37bd52b3cbbec97eea423439accc7733143fd272;hb=HEAD) in the file [Mines.cs](logic/Mines.cs). The port isn't perfect or completely loyal to the original source, and will be worked on further.

### Game density

More advanced tactics of Minesweeper are only applicable in boards with high densities. Increasing the mine density beyond 0.5 will decrease the chances of getting "boring" games.

### Puzzles

The aim is to introduce new players to the basic rules and advanced strategies of Minesweeper. Online guides such as that of [minesweeper.online](https://minesweeper.online/help/patterns) could be used for this.

### Anti-Guess

Guessing in Minesweeper just ruins the fun of the game, learning advanced tactics and the maths behind it while improving speed makes the game more fun.

### `?` tiles

In this mode, not all tiles may reveal information about how many mines are around it. Some uncovered tiles may have a `?`, and provide no information about surrounding mines.


## Inspiration

- [Simon Tatham's Puzzle Collection](https://www.chiark.greenend.org.uk/~sgtatham/puzzles/)
- [AntiMine](https://github.com/lucasnlm/antimine-android)

### Why I am making this game

The android port of Simon Tatham's Puzzle Collection does not have easy panning, like that of AntiMine. AntiMine does not support densities as high has that of Simon Tatham's Puzzle Collection, but it has amazing UI. I wanted to combine the best qualities of both, and Godot seemed like the easiest option for the most cross-platform solution.

### Potentially similar games

- [DragonSweeper](https://danielben.itch.io/dragonsweeper)
- [14 Minesweeper Variants](https://store.steampowered.com/app/1865060/14_Minesweeper_Variants/)
- [DemonCrawl](https://store.steampowered.com/app/1141220/DemonCrawl/)
