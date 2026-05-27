# Untitled Minesweeper Game

**A cross-platform Minesweeper game aiming to have _all_ the features.**

Made using the Godot game engine.

## Feature List:
- [x] Guaranteed-solvable games
    - [x] Simon Tatham's solver ([solver](mines_core/src/core/solver/st.rs), [generator](mines_core/src/core/generator/st.rs))
    - [ ] Mine-dropping generator
- [x] Games with mine density > 0.4
- [ ] Puzzle levels isolating aspects to teach advanced Minesweeper tactics
    - [ ] Trivial constraints
    - [ ] Wing/subset elimination for 2 constraints
    - [ ] More than 2 constraints
    - [ ] Board-level analysis
- [ ] Anti-Guess: The game automatically detects when the player is guessing instead of solving, losing the game.
    - [ ] Detect wrongly flagging open cells
    - [ ] Generate possible variations of the frontier to detect guessing
    - Reference: [Chocolate Sweeper](https://nyahoon.com/products/chocolate-sweeper)
    - Reference: [Kaboom](https://github.com/pwmarcz/kaboom)
- [ ] Expert mode: `?`. Games required to be solvable using the given information.
- [ ] UI: Dark mode / Themes
- [ ] Real-time board generation (<= 1s)
    - [ ] Testing
    - [ ] idea: Priority queue of constraints
- [ ] Infinite minesweeper: Just set the density and solve forever
- Settings:
    - [x] Chord mode
        - uncover cells which have satisfied the mines count
        - place flags on cells which have satisfied the cleared cells count
    - [ ] Auto mode: Same as Chord but the player need not click on the cell

### Guaranteed-solvable games

The game includes a Rust port of **Simon Tatham's Portable Puzzle Collection**'s [`mines.c`](https://git.tartarus.org/?p=simon/puzzles.git;a=blob;f=mines.c;h=37bd52b3cbbec97eea423439accc7733143fd272;hb=HEAD). The port isn't perfect or completely loyal to the original source in terms of working, but it is functionally equivalent.

### Game density

More advanced tactics of Minesweeper are only applicable in boards with relatively higher densities. Increasing the mine density beyond 0.4 will decrease the chances of getting "boring" games.

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

### Similar games

- [Simon Tatham's mines.c](https://www.chiark.greenend.org.uk/~sgtatham/puzzles/js/mines.html)
- [Chocolate Sweeper](https://nyahoon.com/products/chocolate-sweeper)
- [Kaboom](https://github.com/pwmarcz/kaboom)
- [14 Minesweeper Variants](https://store.steampowered.com/app/1865060/14_Minesweeper_Variants/)
- [DemonCrawl](https://store.steampowered.com/app/1141220/DemonCrawl/)
- [DragonSweeper](https://danielben.itch.io/dragonsweeper)
