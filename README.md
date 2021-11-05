# fm

A simple TUI file manager with vim-like keybind, written in Rust.

- Linux : works well
- MacOS : _should_ work, though some unusual errors may occur
- Windows: almost unavailable due to file-name encoding error

![sample](https://github.com/kyoheiu/fm/blob/main/screenshots/sample.gif)

My aim is to make a file manager that enables you to:

- configurate easily
- do what you want in daily use

And with `fm`, now you can:

- set your own commands to open file inside this program with `config.toml`
- delete, yank & put multiple items
- filter the list and choose item easily
- toggle sort order
- rename item
- empty the trash directory
- execute shell command in shell mode

## Installation

```
git clone https://github.com/kyoheiu/fm.git
cd fm
cargo +nightly install --path .
```

## Usage

| command               |                                                               |
| --------------------- | ------------------------------------------------------------- |
| `fm`                  | Show items in the current directory.                          |
| `fm <directory path>` | Show items in the path. Both relative and absolute available. |

## Key manual

| Key                   | Explanation                                                                                                                                                                                                                                             |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| j / Key Up            | Go up. If the list exceeds max-row, list "scrolls" before the top of the list.                                                                                                                                                                          |
| k / Key Down          | Go down. If the list exceeds max-row, list "scrolls" before the bottom of the list.                                                                                                                                                                     |
| h / Key Left          | Go to parent directory if exists.                                                                                                                                                                                                                       |
| l / Key Right / Enter | Open file or change directory. Commands for execution can be managed in config file.                                                                                                                                                                    |
| gg                    | Go to the top.                                                                                                                                                                                                                                          |
| G                     | Go to the bottom.                                                                                                                                                                                                                                       |
| dd                    | Delete and yank item (item will go to the trash directory).                                                                                                                                                                                             |
| y                     | Yank item. If you yanked other item before, its information is replaced by this one.                                                                                                                                                                    |
| p                     | Put yanked item(s) in the current directory. If item with same name exists, copied item will be renamed with the suffix "\_copied".                                                                                                                     |
| V                     | Switch to select mode, where you can move cursor to select items.                                                                                                                                                                                       |
| d (select mode)       | Delete and yank selected items, and return to normal mode.                                                                                                                                                                                              |
| y (select mode)       | Yank selected items, and return to normal mode.                                                                                                                                                                                                         |
| Ctrl+c                | Copy item name to clipboard.                                                                                                                                                                                                                            |
| t                     | Toggle sort order (by name <-> by modified time). This change remains until the program ends (sort order will be restored as configured).                                                                                                               |
| :                     | Switch to shell mode. Type command and press Enter to execute it (e.g., `:cd ~` means to change directory to home dir and to refresh the list). You can use any command in the displayed directory, though the list may be broken during the execution. |
| c                     | Switch to rename mode (enter new name and press Enter to rename the item).                                                                                                                                                                              |
| /                     | Switch to filter mode (enter keyword and press Enter to go to filtered list).                                                                                                                                                                           |
| E                     | Empty the trash directory. **Please think twice before using this command.**                                                                                                                                                                            |
| Esc                   | Return to normal mode.                                                                                                                                                                                                                                  |
| :q / ZZ               | Exit the program.                                                                                                                                                                                                                                       |
| H                     | Show help.                                                                                                                                                                                                                                              |

Note that items moved to the trash directory are prefixed with Unix time (like `1633843993`) to avoid name conflict. This prefix will be removed when paste.

## Configuration

|                 |                                   |
| --------------- | --------------------------------- |
| config file     | `$XDG_CONFIG_HOME/fm/config.toml` |
| trash directory | `$XDG_CONFIG_HOME/fm/trash`       |

Default config file, which is [here](config.toml), will be created automatically when you launch the program for the first time.

In config.toml, you can configurate:

- color of directory, file, and symlink separatively
- default key for sorting item list ("Name" or "Time")
- how to open files

### Command configuration

If you write

```
default = "nvim"

[exec]
feh = ["jpg", "jpeg", "png", "gif", "svg"]
zathura = ["pdf"]
```

then, .jpg, .jpeg, .png, .gif and .svg files are opened by `feh <file-name>`, .pdf files by `zathura <file-name>` and others by `nvim <file-name>` .

## todo

- [x] easier way to configurate exec commands
- [x] choose whether the warning appears or not when delete
- [x] change sort order (file name / new to old)
- [x] implement shell mode
- [x] implement `V`(select mode)
- [ ] show item size
