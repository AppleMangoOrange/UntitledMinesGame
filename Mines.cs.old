#nullable enable
using Godot;
using System;
using System.Collections.Generic;
using System.Linq;

/// <summary>
/// Minesweeper game logic ported from C to C# for Godot.
/// Designed to be instantiated and used from GDScript.
/// Based on Simon Tatham's Mines implementation.
/// </summary>
[GlobalClass]
public partial class Mines : RefCounted
{
	#region Signals

	/// <summary>Emitted when a cell is revealed. Args: x, y, value (0-8 for counts, -1 for mine)</summary>
	[Signal]
	public delegate void CellRevealedEventHandler(int x, int y, int value);

	/// <summary>Emitted when a cell is flagged or unflagged. Args: x, y, isFlagged</summary>
	[Signal]
	public delegate void CellFlaggedEventHandler(int x, int y, bool isFlagged);

	/// <summary>Emitted when the player hits a mine and loses</summary>
	[Signal]
	public delegate void GameLostEventHandler(int mineX, int mineY);

	/// <summary>Emitted when all non-mine cells are revealed</summary>
	[Signal]
	public delegate void GameWonEventHandler();

	/// <summary>Emitted when the game is reset/started</summary>
	[Signal]
	public delegate void GameStartedEventHandler();

	/// <summary>Emitted when mine layout is generated (after first click)</summary>
	[Signal]
	public delegate void MinesGeneratedEventHandler();

	#endregion

	#region Constants - Cell Values

	/// <summary>Cell is flagged as a mine</summary>
	public const int CELL_FLAGGED = -1;
	/// <summary>Cell is unknown/hidden</summary>
	public const int CELL_UNKNOWN = -2;
	/// <summary>Cell has a question mark</summary>
	public const int CELL_QUESTION = -3;
	/// <summary>Mine revealed on game loss</summary>
	public const int CELL_MINE_REVEALED = 64;
	/// <summary>Mine that killed the player</summary>
	public const int CELL_MINE_HIT = 65;
	/// <summary>Incorrectly flagged cell (shown on loss)</summary>
	public const int CELL_WRONG_FLAG = 66;

	// Enable this to see detailed solver debug output in the console
	private const bool DEBUG_SOLVER = true;

	private void LogInfo(string msg) => GD.PrintRich($"[color=cyan][INFO][/color] [color=gray][Mines][/color] {msg}");
	private void LogDebug(string msg) { if (DEBUG_SOLVER) GD.PrintRich($"[color=gray][DEBUG] [Mines] {msg}[/color]"); }
	private void LogWarn(string msg) => GD.PrintRich($"[color=yellow][WARN][/color] [color=gray][Mines][/color] {msg}");

	#endregion

	#region Inner Classes

	/// <summary>
	/// Encapsulates all mutable game state.
	/// </summary>
	private class GameState
	{
		public int Width;
		public int Height;
		public int MineCount;
		public bool IsDead;
		public bool HasWon;
		public bool IsStarted;
		public bool EnsureSolvable;
		public bool[]? MinePositions;
		public int[] Grid;
		public Random Random;

		public GameState()
		{
			Width = 9;
			Height = 9;
			MineCount = 10;
			IsDead = false;
			HasWon = false;
			IsStarted = false;
			EnsureSolvable = false;
			MinePositions = null;
			Grid = [];
			Random = new Random();
		}

		public void InitializeGrid()
		{
			int totalCells = Width * Height;
			Grid = new int[totalCells];
			for (int i = 0; i < totalCells; i++)
				Grid[i] = CELL_UNKNOWN;
		}
	}

	/// <summary>
	/// Represents a set of unknown squares that share a mine count constraint.
	/// Used by the solver to track deductions.
	/// Corresponds to 'struct set' in mines.c
	/// </summary>
	private class ConstraintSet : IComparable<ConstraintSet>
	{
		public int X;           // Top-left x coordinate of the 3x3 region
		public int Y;           // Top-left y coordinate of the 3x3 region
		public int Mask;        // Bitmask for which cells in 3x3 are part of set (bits 0-8)
		public int MineCount;   // Number of mines in this set
		public bool Todo;       // Whether on the to-do list
		public ConstraintSet? Prev;
		public ConstraintSet? Next;

		public int CompareTo(ConstraintSet? other)
		{
			if (other == null) return 1;
			if (Y != other.Y) return Y.CompareTo(other.Y);
			if (X != other.X) return X.CompareTo(other.X);
			return Mask.CompareTo(other.Mask);
		}

		public override bool Equals(object? obj)
		{
			if (obj is ConstraintSet other)
				return X == other.X && Y == other.Y && Mask == other.Mask;
			return false;
		}

		public override int GetHashCode()
		{
			return HashCode.Combine(X, Y, Mask);
		}
	}

	/// <summary>
	/// Stores constraint sets with efficient lookup and a to-do list.
	/// Optimized for O(1) random access and minimal GC pressure.
	/// Corresponds to 'struct setstore' in mines.c
	/// </summary>
	private class SetStore
	{
		private readonly HashSet<ConstraintSet> _sets;      // For O(1) uniqueness checks
		private readonly List<ConstraintSet> _list;          // For O(1) random access
		private readonly List<ConstraintSet> _overlapBuffer; // Reusable buffer to avoid allocations
		public ConstraintSet? TodoHead;
		public ConstraintSet? TodoTail;

		public SetStore()
		{
			_sets = new HashSet<ConstraintSet>();
			_list = new List<ConstraintSet>();
			_overlapBuffer = new List<ConstraintSet>();
			TodoHead = null;
			TodoTail = null;
		}

		public int Count => _sets.Count;

		public void AddTodo(ConstraintSet s)
		{
			if (s.Todo) return;

			s.Prev = TodoTail;
			if (s.Prev != null)
				s.Prev.Next = s;
			else
				TodoHead = s;
			TodoTail = s;
			s.Next = null;
			s.Todo = true;
		}

		public void Add(int x, int y, int mask, int mines)
		{
			if (mask == 0) return;

			// Normalize so that x,y are genuinely the bounding rectangle
			while ((mask & (1 | 8 | 64)) == 0) { mask >>= 1; x++; }
			while ((mask & (1 | 2 | 4)) == 0) { mask >>= 3; y++; }

			var s = new ConstraintSet { X = x, Y = y, Mask = mask, MineCount = mines, Todo = false };

			if (_sets.Add(s))
			{
				_list.Add(s);
				AddTodo(s);
			}
		}

		public void Remove(ConstraintSet s)
		{
			// Remove from todo list
			if (s.Prev != null)
				s.Prev.Next = s.Next;
			else if (s == TodoHead)
				TodoHead = s.Next;

			if (s.Next != null)
				s.Next.Prev = s.Prev;
			else if (s == TodoTail)
				TodoTail = s.Prev;

			s.Todo = false;

			if (_sets.Remove(s))
			{
				// Swap-and-pop removal for O(1) average performance
				int index = _list.IndexOf(s);
				if (index != -1)
				{
					int lastIdx = _list.Count - 1;
					if (index != lastIdx)
						_list[index] = _list[lastIdx];
					_list.RemoveAt(lastIdx);
				}
			}
		}

		/// <summary>
		/// Get all sets overlapping the given position/mask.
		/// Returns a reusable buffer - caller must use immediately before next call.
		/// </summary>
		public List<ConstraintSet> GetOverlapping(int x, int y, int mask)
		{
			_overlapBuffer.Clear();

			// Iterate over List (faster than HashSet enumerator)
			foreach (var s in _list)
			{
				// Check if sets overlap
				if (Math.Abs(s.X - x) < 3 && Math.Abs(s.Y - y) < 3)
				{
					if (SetMunge(x, y, mask, s.X, s.Y, s.Mask, false) != 0)
					{
						_overlapBuffer.Add(s);
					}
				}
			}

			return _overlapBuffer;
		}

		public ConstraintSet? PopTodo()
		{
			if (TodoHead == null) return null;

			var ret = TodoHead;
			TodoHead = ret.Next;
			if (TodoHead != null)
				TodoHead.Prev = null;
			else
				TodoTail = null;

			ret.Next = ret.Prev = null;
			ret.Todo = false;
			return ret;
		}

		/// <summary>
		/// Get a random set. O(1) performance.
		/// </summary>
		public ConstraintSet? GetRandom(Random rng)
		{
			if (_list.Count == 0) return null;
			return _list[rng.Next(_list.Count)];
		}

		public IEnumerable<ConstraintSet> All => _list;
	}

	/// <summary>
	/// Represents a change to make during mine layout perturbation.
	/// </summary>
	private struct PerturbationChange
	{
		public int X;
		public int Y;
		public int Delta;  // +1 = add mine, -1 = remove mine

		public PerturbationChange(int x, int y, int delta)
		{
			X = x;
			Y = y;
			Delta = delta;
		}
	}

	/// <summary>
	/// Context for mine generation with solver.
	/// Corresponds to 'struct minectx' in mines.c
	/// </summary>
	private class MineGenContext
	{
		public bool[] Grid;           // Actual mine positions
		public bool[] Opened;         // Which squares have been opened
		public int Width;
		public int Height;
		public int SafeX;             // First click X
		public int SafeY;             // First click Y
		public bool AllowBigPerturbs;
		public int PerturbsSinceLastOpen;
		public Random Random;

		public MineGenContext(int w, int h, int sx, int sy, Random random)
		{
			Width = w;
			Height = h;
			SafeX = sx;
			SafeY = sy;
			Grid = new bool[w * h];
			Opened = new bool[w * h];
			AllowBigPerturbs = false;
			PerturbsSinceLastOpen = 0;
			Random = random;
		}
	}

	/// <summary>
	/// Linked list for tracking squares to process.
	/// Corresponds to 'struct squaretodo' in mines.c
	/// </summary>
	private class SquareTodo
	{
		public int[] Next;
		public int Head;
		public int Tail;

		public SquareTodo(int size)
		{
			Next = new int[size];
			for (int i = 0; i < size; i++)
				Next[i] = -1;
			Head = -1;
			Tail = -1;
		}

		public void Add(int i)
		{
			if (Tail >= 0)
				Next[Tail] = i;
			else
				Head = i;
			Tail = i;
			Next[i] = -1;
		}
	}

	#endregion

	#region Game State

	private GameState _state = new();

	#endregion

	#region Properties for GDScript

	public int Width => _state.Width;
	public int Height => _state.Height;
	public int MineCount => _state.MineCount;
	public bool IsDead => _state.IsDead;
	public bool HasWon => _state.HasWon;
	public bool IsStarted => _state.IsStarted;
	public bool IsPlaying => _state.IsStarted && !_state.IsDead && !_state.HasWon;

	#endregion

	#region Initialization

	public Mines()
	{
		_state = new GameState();
		_state.InitializeGrid();
	}

	public void NewGame(int width, int height, int mineCount)
	{
		NewGame(width, height, mineCount, -1, true);
	}

	public void NewGame(int width, int height, int mineCount, int seed)
	{
		NewGame(width, height, mineCount, seed, true);
	}

	public void NewGame(int width, int height, int mineCount, int seed, bool ensureSolvable)
	{
		_state = new GameState
		{
			Width = Math.Max(3, width),
			Height = Math.Max(3, height),
			EnsureSolvable = ensureSolvable
		};

		int maxMines = (_state.Width * _state.Height) - 9;
		_state.MineCount = Math.Clamp(mineCount, 1, maxMines);
		_state.Random = seed >= 0 ? new Random(seed) : new Random();
		_state.IsDead = false;
		_state.HasWon = false;
		_state.IsStarted = false;
		_state.MinePositions = null;

		_state.InitializeGrid();
		EmitSignal(SignalName.GameStarted);
	}

	public void Reset()
	{
		NewGame(_state.Width, _state.Height, _state.MineCount, -1, _state.EnsureSolvable);
	}

	#endregion

	#region Solver Utility Functions

	/// <summary>
	/// Count the number of set bits in a 16-bit word.
	/// Corresponds to bitcount16 in mines.c
	/// </summary>
	private static int BitCount16(int word)
	{
		uint w = (uint)word;
		w = ((w & 0xAAAA) >> 1) + (w & 0x5555);
		w = ((w & 0xCCCC) >> 2) + (w & 0x3333);
		w = ((w & 0xF0F0) >> 4) + (w & 0x0F0F);
		w = ((w & 0xFF00) >> 8) + (w & 0x00FF);
		return (int)w;
	}

	/// <summary>
	/// Transform one set's mask to align with another set's coordinate system,
	/// then either intersect or subtract.
	/// Corresponds to setmunge in mines.c
	/// </summary>
	private static int SetMunge(int x1, int y1, int mask1, int x2, int y2, int mask2, bool diff)
	{
		// Adjust mask2 to align with x1,y1 coordinates
		if (Math.Abs(x2 - x1) >= 3 || Math.Abs(y2 - y1) >= 3)
		{
			mask2 = 0;
		}
		else
		{
			while (x2 > x1) { mask2 &= ~(4 | 32 | 256); mask2 <<= 1; x2--; }
			while (x2 < x1) { mask2 &= ~(1 | 8 | 64); mask2 >>= 1; x2++; }
			while (y2 > y1) { mask2 &= ~(64 | 128 | 256); mask2 <<= 3; y2--; }
			while (y2 < y1) { mask2 &= ~(1 | 2 | 4); mask2 >>= 3; y2++; }
		}

		// Invert if diff (we want A & ~B rather than A & B)
		if (diff)
			mask2 ^= 511;

		return mask1 & mask2;
	}

	/// <summary>
	/// Mark known squares as either mines or safe.
	/// Corresponds to known_squares in mines.c
	/// </summary>
	private void KnownSquares(int w, int h, SquareTodo std, int[] grid,
							  MineGenContext ctx, int x, int y, int mask, bool isMine)
	{
		int bit = 1;
		for (int yy = 0; yy < 3; yy++)
		{
			for (int xx = 0; xx < 3; xx++)
			{
				if ((mask & bit) != 0)
				{
					int i = (y + yy) * w + (x + xx);
					if (i >= 0 && i < w * h && grid[i] == CELL_UNKNOWN)
					{
						if (isMine)
						{
							grid[i] = CELL_FLAGGED;
						}
						else
						{
							grid[i] = MineOpen(ctx, x + xx, y + yy);
						}
						std.Add(i);
					}
				}
				bit <<= 1;
			}
		}
	}

	/// <summary>
	/// Open a square and return its mine count.
	/// Corresponds to mineopen in mines.c
	/// </summary>
	private int MineOpen(MineGenContext ctx, int x, int y)
	{
		if (x < 0 || x >= ctx.Width || y < 0 || y >= ctx.Height)
			return -1;

		int idx = y * ctx.Width + x;
		if (ctx.Grid[idx])
			return -1;  // Hit a mine

		if (!ctx.Opened[idx])
		{
			ctx.Opened[idx] = true;
			ctx.PerturbsSinceLastOpen = 0;
		}

		// Count neighboring mines
		int count = 0;
		for (int dy = -1; dy <= 1; dy++)
		{
			for (int dx = -1; dx <= 1; dx++)
			{
				int nx = x + dx, ny = y + dy;
				if (nx >= 0 && nx < ctx.Width && ny >= 0 && ny < ctx.Height)
				{
					if (ctx.Grid[ny * ctx.Width + nx])
						count++;
				}
			}
		}
		return count;
	}

	#endregion

	#region Solver

	/// <summary>
	/// Main solver. Returns:
	/// -1: deduction stalled (unsolvable without guessing)
	///  0: solved completely
	/// >0: number of perturbation steps required
	/// Corresponds to minesolve in mines.c
	/// </summary>
	private int MineSolve(int w, int h, int n, int[] grid, MineGenContext? ctx)
	{
		var ss = new SetStore();
		var std = new SquareTodo(w * h);
		int nperturbs = 0;

		// Initialize todo list with all known squares
		for (int y = 0; y < h; y++)
		{
			for (int x = 0; x < w; x++)
			{
				int i = y * w + x;
				if (grid[i] != CELL_UNKNOWN)
					std.Add(i);
			}
		}

		// Main deductive loop
		while (true)
		{
			bool doneSomething = false;

			// Process known squares
			while (std.Head != -1)
			{
				int i = std.Head;
				std.Head = std.Next[i];
				if (std.Head == -1)
					std.Tail = -1;

				int x = i % w;
				int y = i / w;

				if (grid[i] >= 0)
				{
					// Empty square - construct set of unknown neighbors
					int mines = grid[i];
					int bit = 1;
					int val = 0;

					for (int dy = -1; dy <= 1; dy++)
					{
						for (int dx = -1; dx <= 1; dx++)
						{
							int nx = x + dx, ny = y + dy;
							if (nx >= 0 && nx < w && ny >= 0 && ny < h)
							{
								int ni = ny * w + nx;
								if (grid[ni] == CELL_FLAGGED)
									mines--;
								else if (grid[ni] == CELL_UNKNOWN)
									val |= bit;
							}
							bit <<= 1;
						}
					}

					if (val != 0)
						ss.Add(x - 1, y - 1, val, mines);
				}

				// Find and update sets containing this square
				var overlapping = ss.GetOverlapping(x, y, 1);
				foreach (var s in overlapping)
				{
					int newmask = SetMunge(s.X, s.Y, s.Mask, x, y, 1, true);
					int newmines = s.MineCount - (grid[i] == CELL_FLAGGED ? 1 : 0);

					if (newmask != 0)
						ss.Add(s.X, s.Y, newmask, newmines);

					ss.Remove(s);
				}

				doneSomething = true;
			}

			// Process constraint sets
			var s_todo = ss.PopTodo();
			if (s_todo != null)
			{
				// Check if all squares in set are mines or all are safe
				int setSize = BitCount16(s_todo.Mask);
				if (s_todo.MineCount == 0 || s_todo.MineCount == setSize)
				{
					if (ctx != null)
					{
						KnownSquares(w, h, std, grid, ctx, s_todo.X, s_todo.Y, s_todo.Mask,
									s_todo.MineCount != 0);
					}
					continue;
				}

				// Check overlapping sets for deductions
				var overlapping = ss.GetOverlapping(s_todo.X, s_todo.Y, s_todo.Mask);
				foreach (var s2 in overlapping)
				{
					// Find non-overlapping parts (wings)
					int swing = SetMunge(s_todo.X, s_todo.Y, s_todo.Mask, s2.X, s2.Y, s2.Mask, true);
					int s2wing = SetMunge(s2.X, s2.Y, s2.Mask, s_todo.X, s_todo.Y, s_todo.Mask, true);
					int swc = BitCount16(swing);
					int s2wc = BitCount16(s2wing);

					// If one set has more mines and the difference equals wing size
					if (swc == s_todo.MineCount - s2.MineCount || s2wc == s2.MineCount - s_todo.MineCount)
					{
						if (ctx != null)
						{
							KnownSquares(w, h, std, grid, ctx, s_todo.X, s_todo.Y, swing,
										swc == s_todo.MineCount - s2.MineCount);
							KnownSquares(w, h, std, grid, ctx, s2.X, s2.Y, s2wing,
										s2wc == s2.MineCount - s_todo.MineCount);
						}
						continue;
					}

					// Check if one is a subset of the other
					if (swc == 0 && s2wc != 0)
					{
						ss.Add(s2.X, s2.Y, s2wing, s2.MineCount - s_todo.MineCount);
					}
					else if (s2wc == 0 && swc != 0)
					{
						ss.Add(s_todo.X, s_todo.Y, swing, s_todo.MineCount - s2.MineCount);
					}
				}

				doneSomething = true;
			}
			else if (n >= 0)
			{
				// Global deduction based on total mine count
				int minesleft = n;
				int squaresleft = 0;

				for (int i = 0; i < w * h; i++)
				{
					if (grid[i] == CELL_FLAGGED)
						minesleft--;
					else if (grid[i] == CELL_UNKNOWN)
						squaresleft++;
				}

				if (squaresleft == 0)
					break;  // Solved!

				// If all remaining squares are mines or all are safe
				if (minesleft == 0 || minesleft == squaresleft)
				{
					if (ctx != null)
					{
						for (int i = 0; i < w * h; i++)
						{
							if (grid[i] == CELL_UNKNOWN)
							{
								KnownSquares(w, h, std, grid, ctx, i % w, i / w, 1, minesleft != 0);
							}
						}
					}
					continue;
				}

				// ============================================================
				// RECURSIVE SET COMBINATION LOGIC (the "magic sauce")
				// Try every combination of disjoint sets to find a union that
				// accounts for all remaining mines, allowing global deductions.
				// This is essential for solving closed loops and isolated islands.
				// ============================================================
				int nsets = ss.Count;
				if (nsets > 0 && nsets <= 10 && ctx != null) // Limit to 10 sets to prevent performance issues
				{
					var sets = ss.All.ToList(); // Snapshot of current sets
					bool[] setUsed = new bool[nsets];
					int cursor = 0;
					int unionMines = 0;
					int unionSquares = 0;
					bool foundDeduction = false;

					while (!foundDeduction)
					{
						if (cursor < nsets)
						{
							// Try adding sets[cursor] to our union
							bool canAdd = true;

							// Check if this set overlaps with anything already in the union
							for (int i = 0; i < cursor && canAdd; i++)
							{
								if (setUsed[i])
								{
									// Check intersection - if non-zero, they overlap
									if (SetMunge(sets[cursor].X, sets[cursor].Y, sets[cursor].Mask,
												 sets[i].X, sets[i].Y, sets[i].Mask, false) != 0)
									{
										canAdd = false;
									}
								}
							}

							if (canAdd)
							{
								// Add to union
								unionMines += sets[cursor].MineCount;
								unionSquares += BitCount16(sets[cursor].Mask);
								setUsed[cursor] = true;
							}
							else
							{
								setUsed[cursor] = false;
							}
							cursor++;
						}
						else
						{
							// Reached a leaf - check if the remaining squares outside the union
							// can be determined based on the remaining mine count
							int outsideMines = minesleft - unionMines;
							int outsideSquares = squaresleft - unionSquares;

							if (outsideSquares > 0 && (outsideMines == 0 || outsideMines == outsideSquares))
							{
								// FOUND A DEDUCTION!
								// Any square NOT in our union is determined (Safe or Mine)
								// LogDebug($"Recursive deduction: outsideSquares={outsideSquares}, outsideMines={outsideMines}");

								for (int i = 0; i < w * h; i++)
								{
									if (grid[i] == CELL_UNKNOWN)
									{
										int sx = i % w;
										int sy = i / w;

										// Check if this square is inside our union
										bool insideUnion = false;
										for (int j = 0; j < nsets && !insideUnion; j++)
										{
											if (setUsed[j])
											{
												// Check if sets[j] covers (sx, sy)
												int dx = sx - sets[j].X;
												int dy = sy - sets[j].Y;
												if (dx >= 0 && dx < 3 && dy >= 0 && dy < 3)
												{
													if ((sets[j].Mask & (1 << (dy * 3 + dx))) != 0)
													{
														insideUnion = true;
													}
												}
											}
										}

										if (!insideUnion)
										{
											// Outside the union - we know its state!
											KnownSquares(w, h, std, grid, ctx, sx, sy, 1, outsideMines != 0);
										}
									}
								}

								foundDeduction = true;
								doneSomething = true;
							}

							if (!foundDeduction)
							{
								// Backtrack: find the last set we added and try without it
								do
								{
									cursor--;
								} while (cursor >= 0 && !setUsed[cursor]);

								if (cursor >= 0)
								{
									// Remove set[cursor] from the union and skip it
									unionMines -= sets[cursor].MineCount;
									unionSquares -= BitCount16(sets[cursor].Mask);
									setUsed[cursor] = false;
									cursor++; // Advance past this option
								}
								else
								{
									break; // Fully traversed all combinations
								}
							}
						}
					}

					if (foundDeduction)
						continue;
				}

				// Try perturbation if available (last resort)
				if (ctx != null)
				{
					nperturbs++;
					var changes = MinePerturb(ctx, grid, ss);
					if (changes != null && changes.Count > 0)
					{
						// Apply perturbation changes to solver state
						foreach (var change in changes)
						{
							int idx = change.Y * w + change.X;

							if (change.Delta < 0)
							{
								// Square became empty - it's now a known number, add to todo
								// so the solver can generate a ConstraintSet for it
								std.Add(idx);
							}
							// Note: for Delta > 0 (became mine), the grid is set to CELL_FLAGGED
							// in MinePerturb, and we don't need to add it to std

							// Update affected constraint sets
							var affected = ss.GetOverlapping(change.X, change.Y, 1);
							foreach (var s in affected)
							{
								s.MineCount += change.Delta;
								ss.AddTodo(s);
							}
						}
						continue;
					}
				}
			}

			if (doneSomething)
				continue;

			break;  // No more progress possible
		}

		// Check if solved
		for (int y = 0; y < h; y++)
		{
			for (int x = 0; x < w; x++)
			{
				if (grid[y * w + x] == CELL_UNKNOWN)
					return -1;  // Not solved
			}
		}

		return nperturbs;
	}

	/// <summary>
	/// Perturb the mine layout to make progress.
	/// Corresponds to mineperturb in mines.c
	/// </summary>
	private List<PerturbationChange>? MinePerturb(MineGenContext ctx, int[] grid, SetStore ss)
	{
		if (ctx.PerturbsSinceLastOpen++ > ctx.Width || ctx.PerturbsSinceLastOpen++ > ctx.Height)
		{
			LogDebug($"MinePerturb: Too many perturbs since last open ({ctx.PerturbsSinceLastOpen}), aborting");
			return null;
		}

		int w = ctx.Width;
		int h = ctx.Height;

		// Build list of candidate squares for perturbation
		var candidates = new List<(int x, int y, int type, int random)>();

		for (int y = 0; y < h; y++)
		{
			for (int x = 0; x < w; x++)
			{
				// Skip squares near starting position
				if (Math.Abs(y - ctx.SafeY) <= 1 && Math.Abs(x - ctx.SafeX) <= 1)
					continue;

				int type;
				if (grid[y * w + x] != CELL_UNKNOWN)
				{
					type = 3;  // Known square (last resort)
				}
				else
				{
					// Check if borders on known space
					type = 2;
					for (int dy = -1; dy <= 1; dy++)
					{
						for (int dx = -1; dx <= 1; dx++)
						{
							int nx = x + dx, ny = y + dy;
							if (nx >= 0 && nx < w && ny >= 0 && ny < h &&
								grid[ny * w + nx] != CELL_UNKNOWN)
							{
								type = 1;
								break;
							}
						}
						if (type == 1) break;
					}
				}

				candidates.Add((x, y, type, ctx.Random.Next()));
			}
		}

		// Sort by type, then random for shuffling within type
		candidates.Sort((a, b) =>
		{
			if (a.type != b.type) return a.type.CompareTo(b.type);
			return a.random.CompareTo(b.random);
		});

		// Pick a random set from the solver (O(1) with optimized SetStore)
		ConstraintSet? targetSet = ss.GetRandom(ctx.Random);

		if (targetSet == null)
			return null;

		// Count full and empty squares in the target set
		int nfull = 0, nempty = 0;
		for (int dy = 0; dy < 3; dy++)
		{
			for (int dx = 0; dx < 3; dx++)
			{
				if ((targetSet.Mask & (1 << (dy * 3 + dx))) != 0)
				{
					int sx = targetSet.X + dx;
					int sy = targetSet.Y + dy;
					if (sx >= 0 && sx < w && sy >= 0 && sy < h)
					{
						if (ctx.Grid[sy * w + sx])
							nfull++;
						else
							nempty++;
					}
				}
			}
		}

		// Find squares to fill or empty
		var toFill = new List<(int x, int y)>();
		var toEmpty = new List<(int x, int y)>();

		foreach (var (x, y, type, _) in candidates)
		{
			// Skip if in target set
			if (x >= targetSet.X && x < targetSet.X + 3 &&
				y >= targetSet.Y && y < targetSet.Y + 3 &&
				(targetSet.Mask & (1 << ((y - targetSet.Y) * 3 + (x - targetSet.X)))) != 0)
				continue;

			if (ctx.Grid[y * w + x])
				toEmpty.Add((x, y));
			else
				toFill.Add((x, y));

			if (toFill.Count >= nfull || toEmpty.Count >= nempty)
				break;
		}

		// Decide whether to fill or empty the target set
		var changes = new List<PerturbationChange>();

		if (toFill.Count >= nfull && nfull > 0)
		{
			// Fill the target set with mines
			foreach (var (x, y) in toFill.Take(nfull))
			{
				changes.Add(new PerturbationChange(x, y, +1));
				ctx.Grid[y * w + x] = true;
			}

			// Empty mines from target set
			for (int dy = 0; dy < 3; dy++)
			{
				for (int dx = 0; dx < 3; dx++)
				{
					if ((targetSet.Mask & (1 << (dy * 3 + dx))) != 0)
					{
						int sx = targetSet.X + dx;
						int sy = targetSet.Y + dy;
						if (sx >= 0 && sx < w && sy >= 0 && sy < h && ctx.Grid[sy * w + sx])
						{
							changes.Add(new PerturbationChange(sx, sy, -1));
							ctx.Grid[sy * w + sx] = false;
						}
					}
				}
			}
		}
		else if (toEmpty.Count >= nempty && nempty > 0)
		{
			// Empty adjacent squares
			foreach (var (x, y) in toEmpty.Take(nempty))
			{
				changes.Add(new PerturbationChange(x, y, -1));
				ctx.Grid[y * w + x] = false;
			}

			// Fill mines into target set
			for (int dy = 0; dy < 3; dy++)
			{
				for (int dx = 0; dx < 3; dx++)
				{
					if ((targetSet.Mask & (1 << (dy * 3 + dx))) != 0)
					{
						int sx = targetSet.X + dx;
						int sy = targetSet.Y + dy;
						if (sx >= 0 && sx < w && sy >= 0 && sy < h && !ctx.Grid[sy * w + sx])
						{
							changes.Add(new PerturbationChange(sx, sy, +1));
							ctx.Grid[sy * w + sx] = true;
						}
					}
				}
			}
		}
		else if (toEmpty.Count > 0 && nempty > 0)
		{
			// PARTIAL PERTURBATION FALLBACK:
			// We can't find enough squares to completely fill or empty the set.
			// Instead, do a partial fill - swap what we can.
			// This is crucial for high-density boards where perfect swaps aren't possible.

			// Build list of empty squares in the target set
			var setEmptySquares = new List<int>();
			for (int dy = 0; dy < 3; dy++)
			{
				for (int dx = 0; dx < 3; dx++)
				{
					if ((targetSet.Mask & (1 << (dy * 3 + dx))) != 0)
					{
						int sx = targetSet.X + dx;
						int sy = targetSet.Y + dy;
						if (sx >= 0 && sx < w && sy >= 0 && sy < h && !ctx.Grid[sy * w + sx])
							setEmptySquares.Add(sy * w + sx);
					}
				}
			}

			// Pick min(toEmpty.Count, setEmptySquares.Count) random squares to swap
			int numToSwap = Math.Min(toEmpty.Count, setEmptySquares.Count);
			if (numToSwap > 0)
			{
				// Shuffle the setEmptySquares list and take first numToSwap
				for (int k = 0; k < numToSwap; k++)
				{
					int swapIdx = k + ctx.Random.Next(setEmptySquares.Count - k);
					(setEmptySquares[k], setEmptySquares[swapIdx]) = (setEmptySquares[swapIdx], setEmptySquares[k]);
				}

				// Empty mines from outside
				for (int k = 0; k < numToSwap; k++)
				{
					var (ex, ey) = toEmpty[k];
					changes.Add(new PerturbationChange(ex, ey, -1));
					ctx.Grid[ey * w + ex] = false;
				}

				// Fill mines into set
				for (int k = 0; k < numToSwap; k++)
				{
					int idx = setEmptySquares[k];
					int fx = idx % w, fy = idx / w;
					changes.Add(new PerturbationChange(fx, fy, +1));
					ctx.Grid[fy * w + fx] = true;
				}
			}
		}

		// Update grid values for affected squares
		foreach (var change in changes)
		{
			int x = change.X, y = change.Y;
			for (int dy = -1; dy <= 1; dy++)
			{
				for (int dx = -1; dx <= 1; dx++)
				{
					int nx = x + dx, ny = y + dy;
					if (nx >= 0 && nx < w && ny >= 0 && ny < h)
					{
						if (dx == 0 && dy == 0)
						{
							if (change.Delta > 0)
							{
								grid[y * w + x] = CELL_FLAGGED;
							}
							else
							{
								// Recalculate number
								int count = 0;
								for (int dy2 = -1; dy2 <= 1; dy2++)
								{
									for (int dx2 = -1; dx2 <= 1; dx2++)
									{
										int nx2 = x + dx2, ny2 = y + dy2;
										if (nx2 >= 0 && nx2 < w && ny2 >= 0 && ny2 < h &&
											ctx.Grid[ny2 * w + nx2])
											count++;
									}
								}
								grid[y * w + x] = count;
							}
						}
						else if (grid[ny * w + nx] >= 0)
						{
							grid[ny * w + nx] += change.Delta;
						}
					}
				}
			}
		}

		return changes.Count > 0 ? changes : null;
	}

	#endregion

	#region Mine Generation

	/// <summary>
	/// Generate a solvable mine layout.
	/// Corresponds to minegen in mines.c
	/// </summary>
	private bool[] GenerateMinesWithSolver(int w, int h, int n, int safeX, int safeY, bool unique)
	{
		LogInfo($"Generating {w}x{h} solvable board ({n} mines)");

		bool[] result = new bool[w * h];
		bool success = false;
		int ntries = 0;

		while (!success)
		{
			success = false;
			ntries++;

			if (ntries % 100 == 1) LogDebug($"Attempt #{ntries}...");

			Array.Clear(result);

			// Place n mines randomly, avoiding the 3x3 safe zone
			var validPositions = new List<int>();
			for (int y = 0; y < h; y++)
			{
				for (int x = 0; x < w; x++)
				{
					if (Math.Abs(y - safeY) > 1 || Math.Abs(x - safeX) > 1)
						validPositions.Add(y * w + x);
				}
			}

			int minesToPlace = Math.Min(n, validPositions.Count);
			for (int i = 0; i < minesToPlace; i++)
			{
				int idx = _state.Random.Next(validPositions.Count);
				result[validPositions[idx]] = true;
				validPositions.RemoveAt(idx);
			}

			if (!unique)
			{
				success = true;
				continue;
			}

			// Try to solve with this layout
			var solveGrid = new int[w * h];
			for (int i = 0; i < w * h; i++)
				solveGrid[i] = CELL_UNKNOWN;

			var ctx = new MineGenContext(w, h, safeX, safeY, _state.Random)
			{
				AllowBigPerturbs = ntries > 100
			};
			Array.Copy(result, ctx.Grid, w * h);

			// Open the first square
			solveGrid[safeY * w + safeX] = MineOpen(ctx, safeX, safeY);

			int prevRet = -2;
			while (true)
			{
				// Reset solve grid
				for (int i = 0; i < w * h; i++)
					solveGrid[i] = CELL_UNKNOWN;
				solveGrid[safeY * w + safeX] = MineOpen(ctx, safeX, safeY);

				int solveRet = MineSolve(w, h, n, solveGrid, ctx);

				if (solveRet < 0 || (prevRet >= 0 && solveRet >= prevRet))
				{
					success = false;
					break;
				}
				else if (solveRet == 0)
				{
					// Copy result from context
					LogInfo($"Found solvable layout in {ntries} attempts");
					Array.Copy(ctx.Grid, result, w * h);
					success = true;
					break;
				}

				prevRet = solveRet;
			}

			// Limit retries - if we've tried too many times, fall back to random generation
			if (ntries > 1000)
			{
				LogWarn($"Gave up after {ntries} attempts! Using random (unsolvable) layout.");
				// Generate fresh random mines (not the partially-solved state)
				Array.Clear(result);
				var fallbackPositions = new List<int>();
				for (int y = 0; y < h; y++)
				{
					for (int x = 0; x < w; x++)
					{
						if (Math.Abs(y - safeY) > 1 || Math.Abs(x - safeX) > 1)
							fallbackPositions.Add(y * w + x);
					}
				}
				int fallbackMines = Math.Min(n, fallbackPositions.Count);
				for (int i = 0; i < fallbackMines; i++)
				{
					int idx = _state.Random.Next(fallbackPositions.Count);
					result[fallbackPositions[idx]] = true;
					fallbackPositions.RemoveAt(idx);
				}
				success = true;  // Give up and use random layout
			}
		}

		return result;
	}

	/// <summary>
	/// Generate mine layout (simple version without solver).
	/// </summary>
	private void GenerateMinesSimple(int safeX, int safeY)
	{
		int totalCells = _state.Width * _state.Height;
		_state.MinePositions = new bool[totalCells];

		var validPositions = new List<int>();
		for (int y = 0; y < _state.Height; y++)
		{
			for (int x = 0; x < _state.Width; x++)
			{
				if (Math.Abs(x - safeX) > 1 || Math.Abs(y - safeY) > 1)
					validPositions.Add(y * _state.Width + x);
			}
		}

		int minesToPlace = Math.Min(_state.MineCount, validPositions.Count);
		for (int i = 0; i < minesToPlace; i++)
		{
			int index = _state.Random.Next(validPositions.Count);
			_state.MinePositions[validPositions[index]] = true;
			validPositions.RemoveAt(index);
		}

		_state.IsStarted = true;
		EmitSignal(SignalName.MinesGenerated);
	}

	/// <summary>
	/// Generate mines, using solver if EnsureSolvable is true.
	/// </summary>
	private void GenerateMines(int safeX, int safeY)
	{
		if (_state.EnsureSolvable)
		{
			_state.MinePositions = GenerateMinesWithSolver(
				_state.Width, _state.Height, _state.MineCount, safeX, safeY, true);
		}
		else
		{
			GenerateMinesSimple(safeX, safeY);
			return;
		}

		_state.IsStarted = true;
		EmitSignal(SignalName.MinesGenerated);
	}

	#endregion

	#region Game Actions

	public bool Reveal(int x, int y)
	{
		if (_state.IsDead || _state.HasWon)
			return false;

		if (!IsValidCell(x, y))
			return false;

		int index = y * _state.Width + x;

		if (_state.Grid[index] == CELL_FLAGGED || _state.Grid[index] >= 0)
			return false;

		if (_state.MinePositions == null)
			GenerateMines(x, y);

		if (_state.MinePositions![index])
		{
			_state.IsDead = true;
			_state.Grid[index] = CELL_MINE_HIT;
			RevealAllMines();
			EmitSignal(SignalName.CellRevealed, x, y, CELL_MINE_HIT);
			EmitSignal(SignalName.GameLost, x, y);
			return true;
		}

		FloodReveal(x, y);
		CheckWinCondition();

		return true;
	}

	public bool ToggleFlag(int x, int y)
	{
		if (_state.IsDead || _state.HasWon)
			return false;

		if (!IsValidCell(x, y))
			return false;

		int index = y * _state.Width + x;
		int value = _state.Grid[index];

		if (value == CELL_UNKNOWN)
		{
			_state.Grid[index] = CELL_FLAGGED;
			EmitSignal(SignalName.CellFlagged, x, y, true);
			return true;
		}
		else if (value == CELL_FLAGGED)
		{
			_state.Grid[index] = CELL_UNKNOWN;
			EmitSignal(SignalName.CellFlagged, x, y, false);
			return true;
		}

		return false;
	}

	public bool Chord(int x, int y)
	{
		if (_state.IsDead || _state.HasWon)
			return false;

		if (!IsValidCell(x, y))
			return false;

		int index = y * _state.Width + x;
		int value = _state.Grid[index];

		if (value < 1 || value > 8)
			return false;

		int flagCount = 0;
		for (int dy = -1; dy <= 1; dy++)
		{
			for (int dx = -1; dx <= 1; dx++)
			{
				int nx = x + dx, ny = y + dy;
				if (IsValidCell(nx, ny) && _state.Grid[ny * _state.Width + nx] == CELL_FLAGGED)
					flagCount++;
			}
		}

		if (flagCount != value)
			return false;

		bool revealedAny = false;
		for (int dy = -1; dy <= 1; dy++)
		{
			for (int dx = -1; dx <= 1; dx++)
			{
				int nx = x + dx, ny = y + dy;
				if (IsValidCell(nx, ny) && _state.Grid[ny * _state.Width + nx] == CELL_UNKNOWN)
				{
					Reveal(nx, ny);
					revealedAny = true;
				}
			}
		}

		return revealedAny;
	}

	#endregion

	#region Helper Methods

	private bool IsValidCell(int x, int y) =>
		x >= 0 && x < _state.Width && y >= 0 && y < _state.Height;

	private int CountAdjacentMines(int x, int y)
	{
		if (_state.MinePositions == null) return 0;

		int count = 0;
		for (int dy = -1; dy <= 1; dy++)
		{
			for (int dx = -1; dx <= 1; dx++)
			{
				int nx = x + dx, ny = y + dy;
				if (IsValidCell(nx, ny) && _state.MinePositions[ny * _state.Width + nx])
					count++;
			}
		}
		return count;
	}

	private void FloodReveal(int startX, int startY)
	{
		var stack = new Stack<(int x, int y)>();
		stack.Push((startX, startY));

		while (stack.Count > 0)
		{
			var (x, y) = stack.Pop();
			int index = y * _state.Width + x;

			if (_state.Grid[index] >= 0 || _state.Grid[index] == CELL_FLAGGED)
				continue;

			int mineCount = CountAdjacentMines(x, y);
			_state.Grid[index] = mineCount;

			EmitSignal(SignalName.CellRevealed, x, y, mineCount);

			if (mineCount == 0)
			{
				for (int dy = -1; dy <= 1; dy++)
				{
					for (int dx = -1; dx <= 1; dx++)
					{
						int nx = x + dx, ny = y + dy;
						if (IsValidCell(nx, ny) && _state.Grid[ny * _state.Width + nx] == CELL_UNKNOWN)
							stack.Push((nx, ny));
					}
				}
			}
		}
	}

	private void RevealAllMines()
	{
		if (_state.MinePositions == null) return;

		for (int i = 0; i < _state.MinePositions.Length; i++)
		{
			if (_state.MinePositions[i] && _state.Grid[i] != CELL_MINE_HIT)
			{
				if (_state.Grid[i] != CELL_FLAGGED)
					_state.Grid[i] = CELL_MINE_REVEALED;
			}
			else if (_state.Grid[i] == CELL_FLAGGED && !_state.MinePositions[i])
			{
				_state.Grid[i] = CELL_WRONG_FLAG;
			}
		}
	}

	private void CheckWinCondition()
	{
		int hiddenCount = 0;
		for (int i = 0; i < _state.Grid.Length; i++)
		{
			if (_state.Grid[i] < 0)
				hiddenCount++;
		}

		if (hiddenCount == _state.MineCount)
		{
			_state.HasWon = true;

			for (int i = 0; i < _state.Grid.Length; i++)
			{
				if (_state.Grid[i] == CELL_UNKNOWN)
				{
					_state.Grid[i] = CELL_FLAGGED;
					EmitSignal(SignalName.CellFlagged, i % _state.Width, i / _state.Width, true);
				}
			}

			EmitSignal(SignalName.GameWon);
		}
	}

	#endregion

	#region Query Methods for GDScript

	public int GetCell(int x, int y)
	{
		if (!IsValidCell(x, y)) return CELL_UNKNOWN;
		return _state.Grid[y * _state.Width + x];
	}

	public bool IsMine(int x, int y)
	{
		if (_state.MinePositions == null || !IsValidCell(x, y)) return false;
		return _state.MinePositions[y * _state.Width + x];
	}

	public int GetFlagCount()
	{
		int count = 0;
		foreach (int cell in _state.Grid)
			if (cell == CELL_FLAGGED) count++;
		return count;
	}

	public int GetRemainingMines() => _state.MineCount - GetFlagCount();

	public Godot.Collections.Array<int> GetGridArray()
	{
		var arr = new Godot.Collections.Array<int>();
		foreach (int cell in _state.Grid)
			arr.Add(cell);
		return arr;
	}

	public Godot.Collections.Array<Vector2I> GetMinePositions()
	{
		var positions = new Godot.Collections.Array<Vector2I>();
		if (_state.MinePositions == null) return positions;

		for (int i = 0; i < _state.MinePositions.Length; i++)
		{
			if (_state.MinePositions[i])
				positions.Add(new Vector2I(i % _state.Width, i / _state.Width));
		}
		return positions;
	}

	public bool IsRevealed(int x, int y)
	{
		int value = GetCell(x, y);
		return value >= 0 && value <= 8;
	}

	public bool IsFlagged(int x, int y) => GetCell(x, y) == CELL_FLAGGED;

	public bool IsHidden(int x, int y)
	{
		int value = GetCell(x, y);
		return value == CELL_UNKNOWN || value == CELL_QUESTION;
	}

	#endregion
}
