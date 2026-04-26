# `mines.c` — Logic Functions Reference

Documentation for the logic (non-rendering) functions in Simon Tatham's `mines.c`.

---

## Table of Contents

1. [Data Structures](#1-data-structures)
2. [The Mask — What It Is and How It Is Used](#2-the-mask--what-it-is-and-how-it-is-used)
3. [The `set` Struct](#3-the-set-struct)
4. [The `setstore` — Tree + Todo List](#4-the-setstore--tree--todo-list)
5. [Set Operations](#5-set-operations)
6. [The `squaretodo` List](#6-the-squaretodo-list)
7. [Parameters & Configuration](#7-parameters--configuration)
8. [How `minesolve` Works — Detailed Walkthrough](#8-how-minesolve-works--detailed-walkthrough)
9. [Grid Generation](#9-grid-generation)
10. [Game Lifecycle](#10-game-lifecycle)

---

## 1. Data Structures

### `game_params`
```c
struct game_params { int w, h, n; bool unique; int first_click_x, first_click_y; };
```
Board width `w`, height `h`, mine count `n`, whether the grid must be uniquely solvable, and an optional preset first-click position.

---

### `mine_layout`
```c
struct mine_layout { int refcount; bool *mines; int n; bool unique;
                     random_state *rs; midend *me; int startx, starty; };
```
**Reference-counted** layout shared across all `game_state` copies of the same puzzle. Before the first click, `mines` is `NULL`; the deferred random state `rs` is used to generate the layout on demand when the player first clicks. After generation, `startx`/`starty` record that click so the UI can draw a hint cross if the player undoes it.

---

### `game_state`
```c
struct game_state { int w, h, n; bool dead, won, used_solve;
                    struct mine_layout *layout; signed char *grid; };
```
`grid[]` is a flat `w*h` array. Cell at `(x,y)` is `grid[y*w + x]`. Values:

| Value | Meaning |
|-------|---------|
| `0`–`8` | Open; digit = adjacent mine count |
| `-1` | Flagged as a mine by the player |
| `-2` | Unknown / covered |
| `-3` | Marked with a question mark |
| `64` | Mine revealed on game-over (not the one hit) |
| `65` | Mine the player stepped on |
| `66` | Incorrectly flagged square shown on game-over |

---

## 2. The Mask — What It Is and How It Is Used

The **mask** is a 9-bit integer that selects a subset of cells from a 3×3 grid window. A set's `(x, y)` stores the coordinates of the **top-left corner** of that window. Each of the 9 bits corresponds to one cell in the window:

```
Bit layout (bit 0 = LSB):

  bit 0  bit 1  bit 2     →  cells at columns: x, x+1, x+2
  bit 3  bit 4  bit 5     →  cells at columns: x, x+1, x+2
  bit 6  bit 7  bit 8     →  cells at columns: x, x+1, x+2
    ↓      ↓      ↓
  row y  row y  row y
 row y+1 row y+1 row y+1
 row y+2 row y+2 row y+2
```

So bit `(dy * 3 + dx)` corresponds to the cell at `(x + dx, y + dy)`. A bit is **set (1)** if that cell is still **unknown** and is part of the constraint; **clear (0)** if the cell is outside the set (either already revealed or outside the grid). The `mines` field says how many mines are hidden among the cells whose bits are 1.

**Example:** a cell at `(5, 3)` has been revealed as `2`. Its unknown neighbours are at `(4,2)`, `(5,2)`, `(6,2)`, and `(4,4)`. The set would have `x=4, y=2`, and the mask would have bits set for `(0,0)`, `(1,0)`, `(2,0)`, `(0,2)` — i.e. `mask = 0b000000111 | 0b001000000 = 0x47`. `mines = 2`.

The mask is used in `setmunge` for set intersection/difference, in `bitcount16` to count how many cells a constraint covers, and in `known_squares` to iterate over exactly those cells.

---

## 3. The `set` Struct

```c
struct set {
  short x, y;   /* top-left corner of the 3x3 window */
  short mask;   /* 9-bit bitmask: which cells are unknown and in this set */
  short mines;  /* how many mines are among the masked cells */
  bool todo;    /* is this set currently on the todo list? */
  struct set *prev, *next; /* doubly-linked list pointers for the todo list */
};
```

A `set` is a single **constraint**: "among these specific unknown cells (identified by `x`, `y`, and `mask`), exactly `mines` of them contain a mine."

The solver creates one `set` per revealed numbered cell, representing that cell's unknown neighbours. As more squares are revealed, sets are updated (cells removed from the mask, mine count adjusted) or deleted when they become empty.

The `prev` and `next` pointers are **not** for the tree — they are exclusively for threading the set onto the **todo linked list** (described in §4). The tree uses its own internal node structure inside `tree234`.

`todo` is a boolean flag that answers "is this `set` currently on the todo list?" It prevents the same set being enqueued twice when `ss_add_todo` is called on it repeatedly. When a set is dequeued by `ss_todo`, `todo` is reset to `false`.

---

## 4. The `setstore` — Tree + Todo List

```c
struct setstore {
  tree234 *sets;                  /* all known constraints, ordered */
  struct set *todo_head, *todo_tail; /* queue of sets needing processing */
};
```

`setstore` manages two completely separate data structures that happen to share the same `set` objects:

### The Tree (`sets`)

`tree234` is a balanced 2-3-4 tree (from `tree234.c`). Every `set` ever added via `ss_add` lives as a node in this tree. The tree is sorted by the comparator `setcmp`, which orders sets by `(y, x, mask)` — that is, primarily by row, then by column, then by mask value. This ordering is what makes `ss_overlap` efficient: it can jump directly to all sets with a specific `(x, y)` origin using `findrelpos234`, rather than scanning the whole collection.

The tree provides:
- Fast insertion / deletion (`add234`, `del234`)  
- Fast lookup by key (`findrelpos234`)  
- Indexed access by position (`index234`, `count234`)

### The Todo List

The todo list is a **FIFO queue** of `set` pointers, implemented as a doubly-linked list using the `prev`/`next` fields embedded inside each `set`. `todo_head` points to the front (next to be dequeued); `todo_tail` points to the back (where new items are appended).

**Every set lives in both structures simultaneously** — it is a node in the tree AND it may be linked into the todo list at the same time. Removing a set via `ss_remove` unlinks it from both.

**Why two structures?** They serve different purposes:
- The **tree** is for *finding* sets — looking up which sets cover a given square, or iterating all sets.
- The **todo list** is a *work queue* — it records which sets the solver has not yet used to attempt deductions.

**When is a set added to the todo list?** Every time a new set is created (in `ss_add`), it is immediately appended to the todo list. Additionally, when perturbation modifies a mine, all sets overlapping the changed square have their mine count updated and are re-added to the todo list, so the solver re-examines them.

**When is a set removed from the todo list?** Only when `ss_todo` dequeues it for processing. At that point the solver inspects it and attempts deductions. Note that a set is *not* automatically re-added after being dequeued — it only comes back if something changes that makes it worth re-examining (i.e., perturbation).

---

## 5. Set Operations

### `setcmp` — Tree Ordering
```c
static int setcmp(void *av, void *bv);
```
Defines the tree sort order: sort by `y`, then `x`, then `mask`. Used internally by `tree234` for all insertions and lookups.

---

### `setmunge(x1,y1,mask1, x2,y2,mask2, diff)` → `int`

This is the core set-arithmetic function. It answers the question: **"What is the intersection (or difference) of two constraints, expressed in the coordinate system of the first?"**

Both constraints use their own `(x,y)` as origin. Before you can AND the two masks together, you have to shift `mask2` so that its bits align with `mask1`'s coordinate system. That's what the loop at the start does:

- If `x2 > x1`: `mask2`'s window starts to the right. Shift it left by one column (shift bits left by 1), but first zero out the rightmost column of `mask2` (bits 2, 5, 8 = `4|32|256`) so those cells don't wrap around to the left column.
- If `x2 < x1`: opposite — shift right by 1, zeroing the leftmost column (bits 0, 3, 6 = `1|8|64`) first.
- If `y2 > y1`: shift down one row = shift bits left by 3, zeroing the bottom row of `mask2` (bits 6,7,8 = `64|128|256`) first.
- If `y2 < y1`: shift up one row = shift bits right by 3, zeroing the top row (bits 0,1,2 = `1|2|4`).

Repeat until both share the same origin. If the two origins are 3 or more apart in either axis, their 3×3 windows can't possibly overlap, so `mask2` is forced to 0.

Once aligned, the return value is:
- **Intersection** (`diff=false`): `mask1 & mask2` — the bits (cells) that are in **both** constraints. A nonzero result means the two constraints share at least one unknown cell.
- **Difference** (`diff=true`): `mask1 & ~mask2` — the bits in constraint 1 that are **not** in constraint 2. Computed by XOR-ing `mask2` with `511` (all 9 bits set) to flip it, then ANDing with `mask1`.

**The return value is a new mask** — a set of bit positions, not a `set` struct. It represents the cells that result from the operation, still relative to `(x1, y1)`.

---

### `ss_add(ss, x, y, mask, mines)`

**Normalisation:** The caller may pass an `(x,y)` that isn't actually the top-left corner of the occupied cells. For example, if all bits in the leftmost column (bits 0, 3, 6) are 0, the origin `x` is one too far left. The code shifts `mask` right by 1 and increments `x` until bit 0, 3, or 6 is set — meaning the first column of the window actually has a cell in it. The same is done vertically. After this, `(x,y)` genuinely points to the top-left-most cell that is part of the constraint. This ensures two logically identical constraints always produce the same `(x, y, mask)` key, so the tree's deduplication check works correctly.

---

### `ss_overlap(ss, x, y, mask)` → `struct set **`

Returns a heap-allocated, `NULL`-terminated array of all sets in the tree that geometrically overlap the input `(x,y,mask)`. It searches all possible origins within ±3 of the input — since every set covers at most a 3×3 window, any overlapping set must have its origin within that range. For each candidate origin `(xx,yy)`, it uses `findrelpos234` to jump directly to that position in the sorted tree, then scans forward while `x==xx && y==yy`. A `setmunge` intersection check confirms actual overlap before adding to the result.

---

## 6. The `squaretodo` List

```c
struct squaretodo { int *next; int head, tail; };
```

This is a **completely separate, simpler todo list** from the set todo list. While the set todo list tracks which *constraints* need re-examining, the `squaretodo` tracks which **individual grid squares** have just had their status resolved (become known) and need to be processed.

**Storage:** `next` is a pre-allocated `int[w*h]` array. Each entry `next[i]` holds the index of the *next* square in the list, or `-1` if there is none. `head` and `tail` store the indices of the front and back of the queue. This avoids heap allocation per square — the entire linked list structure lives in a single array.

**When is a square added?** Via `std_add(std, i)` where `i = y*w + x`. This happens:
1. At the start of `minesolve`: every already-known square in the input grid is enqueued.
2. Inside `known_squares`: whenever the solver concludes a square is safe or a mine, it is added to the list.
3. After perturbation: if a square was previously revealed (not `-2`) and its mine status changed, it is re-added.

**How is it different from the set todo list?**

| | `squaretodo` | Set todo list |
|---|---|---|
| **Contains** | Grid square indices | `set` constraint structs |
| **Purpose** | "This square is now known — generate/update constraints from it" | "This constraint is new/updated — try to deduce from it" |
| **Populated by** | `std_add` | `ss_add_todo` |
| **Consumed by** | Phase 1 of `minesolve`'s main loop | Phase 2 of `minesolve`'s main loop |
| **Storage** | Plain `int[]` array used as a linked list | `prev`/`next` pointers embedded in `set` structs |

The two lists feed into each other: processing a square (draining `squaretodo`) creates or updates sets (which go onto the set todo list). Processing a set (draining the set todo list) may resolve squares (which go back onto `squaretodo`).

---

## 7. Parameters & Configuration

### `decode_params(params, string)`
Parses `"16x16n99"` into `params`. Format: `<W>x<H>`, then optionally `n<N>` for mine count, `a` for non-unique, `X<x>Y<y>` for first-click override.

### `encode_params(params, full)` → `char *`
Inverse of `decode_params`. Mine count and flags are only written when `full=true`.

### `validate_params(params, full)` → `const char *`
Returns an error string or `NULL`. Key rules:
- `w,h ≥ 1`; for unique grids `w,h > 2` (2-wide grids often have no uniquely solvable layout)
- `n ≥ 1` and `n ≤ w*h − 9` (a 3×3 safe starting zone must always exist)

---

## 8. How `minesolve` Works — Detailed Walkthrough

```c
static int minesolve(int w, int h, int n, signed char *grid,
                     open_cb open, perturb_cb perturb, void *ctx,
                     random_state *rs);
```

`minesolve` is given the player's current knowledge in `grid[]` (with `-2` for unknowns, `0`–`8` for open squares, `-1` for known mines). It tries to deduce as much as possible. The `open` callback lets it reveal a square and learn its digit; `perturb` lets it ask for the mine layout to be rearranged (used only during generation).

**Returns:** `-1` if stuck (failed to solve), `0` if fully solved without perturbation, `>0` = number of perturbation calls needed.

---

### Setup

Two work queues are initialised:
- `squaretodo std`: all already-known squares in the input `grid` are enqueued immediately — they are the starting seeds.
- `setstore ss`: empty; sets will be built from the known squares.

---

### Main Loop

The solver loops indefinitely. Each iteration sets `done_something = false`. If the iteration ends without having done anything, and there's no perturbation function, it breaks out (stuck). Otherwise it keeps going.

---

### Phase 1 — Square Processing (draining `squaretodo`)

```
while std is not empty:
    dequeue square i at (x, y)
    if grid[i] is an open number (≥ 0):
        build a new constraint set from i's unknown neighbours
    find all existing sets that contain cell (x, y) and update them
```

**Building a new constraint:** The code iterates over the 8 neighbours of `(x,y)`. For each neighbour:
- Already flagged as mine (`-1`): subtract 1 from the digit — that mine is already accounted for.
- Unknown (`-2`): set the corresponding bit in `val`.
- Already open (any other value): ignore.

After scanning all neighbours, `val` is the mask of unknown neighbours and `mines` is the digit minus already-flagged neighbours. This is pushed to the set store as `ss_add(ss, x-1, y-1, val, mines)`. The origin `x-1, y-1` is used because a 3×3 window centred on `(x,y)` has its top-left at `(x-1, y-1)`.

**Updating existing sets:** The newly known square must be removed from any existing constraint that included it. `ss_overlap(ss, x, y, 1)` finds all sets containing cell `(x,y)` (the mask `1` = a single cell). For each such set, `setmunge` with `diff=true` computes the old mask minus cell `(x,y)`. If the new square was a mine (`grid[i] == -1`), the set's mine count decreases by 1. The old set is deleted and the trimmed set is re-inserted (unless it became empty, meaning all its cells are now known).

---

### Phase 2 — Set Reasoning (draining the set todo list)

```
if there is a set s on the todo list:
    if s.mines == 0:  all cells in s are safe → reveal them all
    if s.mines == |s|: all cells in s are mines → flag them all
    otherwise: compare s against every overlapping set s2
```

**Trivial resolution:** If mines equals 0 or equals the number of masked cells (`bitcount16(mask)`), every cell in the set is unambiguously safe or mine. `known_squares` is called to record them, which puts them all onto `squaretodo` for Phase 1 to process next iteration.

**Overlapping set analysis:** For two constraints `s` and `s2` that share some cells:
- `swing = s - s2` (cells in s but not s2, via `setmunge diff=true`)
- `s2wing = s2 - s`

**Wing-elimination:** If `s.mines − s2.mines == |swing|`, then the wing of `s` must be entirely mines (all the "extra" mines in s compared to s2 must live in its exclusive cells), and the wing of `s2` must be entirely safe. `known_squares` is called on both wings.

**Subset rule:** If `swing` is empty (s ⊆ s2), then `s2`'s exclusive cells (`s2wing`) contain exactly `s2.mines − s.mines` mines. A new constraint for `(s2wing, s2.mines − s.mines)` is inserted. This doesn't immediately resolve anything but creates a smaller, more useful constraint.

---

### Phase 3 — Global Mine Count Deduction

Reached only when the set todo list is completely empty (no more local deductions possible). This uses the total mine count `n`.

**Counting:** Scan the entire grid. Subtract already-flagged mines (`-1`) from `n` to get `minesleft`. Count unknown cells (`-2`) as `squaresleft`.

**Trivial cases:**
- `squaresleft == 0`: board is solved, break.
- `minesleft == 0`: all remaining unknowns are safe.
- `minesleft == squaresleft`: all remaining unknowns are mines.

**Exhaustive union search (capped at 10 sets):** The idea is to find a disjoint union of current constraint sets such that the unknown squares *outside* that union are either all mines or all safe. Once found, those outside squares can be resolved.

The code uses an iterative "virtual recursion" with a `cursor` and a `setused[10]` boolean array to enumerate all disjoint unions without actual recursive calls:

```
cursor = 0
minesleft, squaresleft = (counts of unaccounted squares)

loop:
    if cursor < nsets:
        check if sets[cursor] is disjoint from all currently used sets
        if yes: add it to the union (setused[cursor]=true), subtract its
                mines and cell count from minesleft/squaresleft
        if no:  setused[cursor]=false (skip it)
        cursor++

    else (cursor == nsets, we have a complete union):
        if squaresleft > 0 and (minesleft==0 or minesleft==squaresleft):
            SUCCESS: the squares outside the union are all safe or all mines
            → call known_squares on each one, break to main loop
        else:
            BACKTRACK: walk cursor backwards to the last setused[i]==true
            remove that set from the union (add back its mines/cells)
            set setused[cursor]=false, advance cursor past it
            if no true entry found anywhere: give up (all unions exhausted)
```

This is equivalent to a depth-first search over all subsets of the constraint set collection, checking the "outside" squares at each leaf. The cap of 10 sets means at most 2¹⁰ = 1024 combinations are tried.

---

### Phase 4 — Perturbation (generation only)

If all three phases fail and a `perturb` callback was provided, the solver picks a random set from the store and calls `perturb(ctx, grid, s->x, s->y, s->mask)`. The perturbation function rearranges the actual mine layout (see §9.2) and returns a list of which squares changed. The solver updates affected constraint mine counts, re-queues those constraints, and goes back to Phase 1.

If perturbation also fails (returns `NULL`) or there is no perturb function, the solver breaks out and returns `-1`.

---

## 9. Grid Generation

### 9.1 `minectx` and `mineopen`

```c
struct minectx { bool *grid, *opened; int w, h, sx, sy;
                 bool allow_big_perturbs;
                 int nperturbs_since_last_new_open; random_state *rs; };
```

`minectx` is the context passed as `void *ctx` to `minesolve` during generation. `grid` is the **real** mine layout. `mineopen` is the `open_cb`: given `(x,y)`, it looks up `ctx->grid` and returns the true neighbour count (or `-1` if it's a mine — which during correct generation should never happen).

### 9.2 `mineperturb`

Called by the solver when stuck. Modifies the real mine layout to make progress possible while keeping the total mine count constant (mines are swapped, not added/removed).

**How it picks swap targets:** Builds a list of all candidate squares outside the stuck constraint set and the 3×3 safe zone around the starting square, sorted by priority: (1) unknown squares bordering revealed territory, (2) fully unknown squares, (3) already-revealed squares. Within each priority group, a random key shuffles the order.

**The swap:** Counts mines (`nfull`) and non-mines (`nempty`) inside the stuck set. Scans the sorted candidates for `nfull` empty squares (`tofill`) and `nempty` full squares (`toempty`). Then either fills the set (puts mines into its empty cells, takes them from `toempty`) or empties it (clears its mines, puts them into `tofill`). All changed squares are returned to the solver in a `perturbations` struct.

### 9.3 `minegen`

```c
static bool *minegen(int w, int h, int n, int x, int y, bool unique, random_state *rs);
```

Generates a complete mine layout. Randomly places `n` mines, none within the 3×3 zone around `(x,y)`. If `unique=true`, runs `minesolve` on it; if the solver returns `0` (fully solvable), done. If it returns `-1` or the perturbation count increases, the whole layout is discarded and regenerated from scratch.

### 9.4 `describe_layout` / `new_mine_layout`

`describe_layout` serialises the `bool[]` mine array to a hex nibble string, optionally obfuscated with `obfuscate_bitmap`. Format: `"<x>,<y>,<m|u><hexdigits>"`. `new_mine_layout` is a thin wrapper that calls `minegen` then `describe_layout`.

---

## 10. Game Lifecycle

### `new_game_desc`
Two modes:
- **Non-interactive**: calls `new_mine_layout` immediately, returns the full encoded layout.
- **Interactive**: returns `"r<n>,<u|a>,<rs>"` with the serialised random state. Layout generation is deferred until the first click.

### `open_square`
The primary player action. If the layout isn't generated yet, generates it now. If the clicked square is a mine, marks `dead=true`. Otherwise marks the square `-10` and flood-fills: iterates the grid looking for `-10` cells, computes their digit, sets it, and if the digit is 0, marks all unknown neighbours `-10` too. Finally checks if `covered == mines` (win condition).

### `execute_move`
Parses a semicolon-separated move string:
- `"S"` — Solve: reveal all mines and numbers (or show corrections if already dead)
- `"Fx,y"` — Flag/unflag a covered square
- `"Ox,y"` — Open a square (`open_square`)
- `"Cx,y"` — Chord: open all unmarked neighbours of an already-open square

### `interpret_move`
Translates raw mouse/keyboard events into move strings. Left-release on a covered square → `"Ox,y"`. Right-click → `"Fx,y"`. Left-release on an open numbered square where marker count matches the digit → `"Cx,y"` (or individual `"Ox,y"` for each wrongly-marked mine).

### Other Helpers

| Function | Purpose |
|----------|---------|
| `dup_game` / `free_game` | Reference-counted copy/free of game state and layout |
| `encode_ui` / `decode_ui` | Persist death counter and completion flag across save/restore |
| `game_status` | Returns `+1` (won), `-1` (lost via Solve), `0` (playing) |
| `game_timing_state` | Stops the timer once dead, won, or layout not yet generated |
| `game_flash_length` | `3×FLASH_FRAME` for death, `2×FLASH_FRAME` for win |
| `set_public_desc` | Extracts and stores the starting `(x,y)` hint from the public game description |

---

## Appendix — Solver Phase Flow

```
        ┌─────────────────────────────────────────────┐
        │              minesolve main loop             │
        │                                              │
        │  ┌─── Phase 1: squaretodo not empty? ───┐   │
        │  │  dequeue square                       │   │
        │  │  build constraint from its unknowns   │   │
        │  │  remove square from existing sets     │   │
        │  │  → may add new sets to set-todo       │   │
        │  └────────────────────────────────────── ┘   │
        │         ↓ (squaretodo drained)                │
        │  ┌─── Phase 2: set-todo not empty? ─────┐   │
        │  │  dequeue set s                        │   │
        │  │  if mines==0 or mines==|s|:           │   │
        │  │    resolve all cells → squaretodo     │   │
        │  │  else compare with overlapping sets:  │   │
        │  │    wing-elim → squaretodo             │   │
        │  │    subset rule → new set → set-todo   │   │
        │  └────────────────────────────────────── ┘   │
        │         ↓ (both queues empty)                 │
        │  ┌─── Phase 3: global mine count ────────┐   │
        │  │  try all disjoint unions (cap 10 sets)│   │
        │  │  if outside squares are all mines/safe│   │
        │  │    resolve them → squaretodo          │   │
        │  └────────────────────────────────────── ┘   │
        │         ↓ (phase 3 found nothing)             │
        │  ┌─── Phase 4: perturbation ─────────────┐   │
        │  │  rearrange mine layout                │   │
        │  │  update affected constraints          │   │
        │  │  → re-queue constraints → set-todo    │   │
        │  └────────────────────────────────────── ┘   │
        │         ↓ (perturbation also failed)          │
        │              return -1 (stuck)                │
        └─────────────────────────────────────────────┘
```
