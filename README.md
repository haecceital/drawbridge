# drawbridge

A language-agnostic, lightweight TUI (Terminal User Interface) rendering engine written in Rust.

`drawbridge` allows you to build rich, interactive terminal animations and applications using **any programming language**. As long as your script or executable can print standard text strings to `stdout` and read from `stdin`, you can bridge it to the terminal with full mouse/keyboard interaction and automatic resizing support.

---

## How It Works

`drawbridge` acts as a middleman between the terminal and your executable script. 

1. It spawns your script as a child process.
2. It listens to your script's `stdout` and renders custom text/colors via standard commands (`Draw`, `Clear`, `Flush`).
3. It captures terminal user inputs (keyboard presses, window resizing) and forwards them directly to your script's `stdin`.

```
┌─────────────────┐   stdout (Commands)    ┌────────────┐
│ Your Executable │ ─────────────────────> │ drawbridge │ ──> [Terminal UI]
│ (Python, Lua,   │ <───────────────────── │   (Rust)   │
│  Bash, etc.)    │    stdin (Events)      └────────────┘
└─────────────────┘
```

---

## Installation

### Prerequisites
Make sure you have Rust and `cargo` installed.

### Build from Source
```bash
git clone https://github.com/haecceital/drawbridge.git
cd drawbridge
cargo build --release
```
The compiled binary will be located at `target/release/drawbridge`.

---

## Usage

Run `drawbridge` by passing your executable or script via the `--exec` flag:

```bash
drawbridge --exec "python3 animation.py"
drawbridge --exec "./animation.lua"
drawbridge --exec "/bin/bash animation.sh"
```

---

## Command Protocol (Stdout)

Your script must output text commands to `stdout`. **Each command must occupy exactly one line**. `drawbridge` buffers these operations until it receives a `Flush()` command.

#### General Parameter Rules
All commands share a highly flexible argument parsing system:
* **Hybrid Style:** All arguments support **both positional and named (keyword)** styles. You can mix them, but **once a named argument is introduced, all subsequent arguments must also be named** (similar to Python's behavior).
* **Optional Arguments:** Parameters enclosed in brackets `[...]` are optional and will fallback to their default values if omitted.

---

### 1. `Draw(...)`
Draws a character or string literal at a specified coordinate.

* **Syntax:** `Draw(x, y, glyph, [fg_color], [bg_color])`

#### Required Parameters
* `x` (Integer): The X coordinate (column).
* `y` (Integer): The Y coordinate (row).
* `glyph` (Character or String): Single quote for character `'a'`, double quote for string `"hello"`.

#### Optional Parameters
* `fg_color` (RGB Tuple): Text color. Defaults to the terminal's default text color (`Color::Reset`).
* `bg_color` (RGB Tuple): Background color. Defaults to the terminal's default background color (`Color::Reset`).

**Examples:**
```text
Draw(0, 0, "Hello World")
Draw(y = 5, glyph = 'A', x = 10, bg_color = (0, 0, 0))
Draw(10, 5, 'X', fg_color=(255,255,0), bg_color=(0,0,255))
```

### 2. `Clear()`
Clears the temporary drawing canvas buffer.
* **Syntax:** `Clear()`

### 3. `Flush()`
Renders all buffered drawing operations onto the actual terminal screen.
* **Syntax:** `Flush()`

---

## Event Protocol (Stdin)

`drawbridge` captures terminal events and writes them into your script's `stdin` so your application can be interactive. Each event ends with a newline character `\n`.

### 1. Key Events
Triggered whenever a user presses a key.
* **Format:** `Key|KeyCode|Modifiers`
* **Example:** `Key|Char('c')|KeyModifiers(CONTROL)` (when hitting `Ctrl+C`)
* **Example:** `Key|Char('a')|KeyModifiers(0x0)` (when hitting plain `a`)

### 2. Resize Events
Triggered whenever the terminal window dimensions change.
* **Format:** `Resize|Width|Height`
* **Example:** `Resize|120|40`

---

## Quick Start Example (Python)

Create a file named `animation.py`:

```python
def draw(x: int, y: int, glyph: str, fg_color: tuple = None, bg_color: tuple = None):
    g_str = f'"{glyph}"' if len(glyph) > 1 else f"'{glyph}'"
    params = {"x": x, "y": y, "glyph": g_str, "fg_color": fg_color, "bg_color": bg_color}
    
    args = [f"{k}={v}" for k, v in params.items() if v is not None]
    
    print(f"Draw({','.join(args)})")

flush = lambda: print("Flush()")
clear = lambda b: (b and print("Clear()")) or True

def main():
    x, y = 1, 1

    cnt = 0
    while clear(True):
        inputs = input().split('|')
        
        if inputs[0] != "Key":
            continue

        key = inputs[1]
        spec = inputs[2]
        if key == "Char('c')" and spec == "KeyModifiers(CONTROL)":
            break
        elif key == "Up":
            y -= 1
        elif key == "Down":
            y += 1
        elif key == "Right":
            x += 1
        elif key == "Left":
            x -= 1


        draw(0, 0, ' '.join(inputs))
        draw(x, y, ' ', bg_color = (255, 255, 255))

        flush()

if __name__ == '__main__':
    main()
```

Run it using `drawbridge`:
```bash
drawbridge --exec "python3 animation.py"
```

---

## License

This project is licensed under the [MIT License](LICENSE).