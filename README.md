[![crates.io](https://img.shields.io/crates/v/felix)](https://crates.io/crates/felix) ![aur:felix-rs](https://img.shields.io/aur/version/felix-rs)

# _felix_

## About

A tui file manager with vim-like key mapping, written in Rust. Fast, simple, and easy to configure & use.

![sample](screenshots/sample.gif)

## New Release

## v1.0.1 (2022-07-28)

### Fixed

- Add thread sleep time after state.open_file(). This is necessary because, with tiling window managers, the window resizing is sometimes slow and felix reloads the layout so quickly that the display may become broken. By the sleep (50ms for now and I think it's not easy to recognize this sleep), this will be avoided.

For more details, see `CHANGELOG.md`.

## Status

| OS      | Status                                                   |
| ------- | -------------------------------------------------------- |
| Linux   | works well                                               |
| NetBSD  | works well                                               |
| MacOS   | works, though some errors may occur. if so, let me know! |
| Windows | almost unavailable due to file-name encoding error       |

## Installation

_Make sure that `gcc` is installed._

From crates.io:

```
cargo install felix
```

From aur:

```
yay -S felix-rs
```

On NetBSD, package is available from the official repositories:

```
pkgin install felix
```

From this repository (develop branch):

```
git clone -b develop https://github.com/kyoheiu/felix.git
cd felix
cargo install --path .
```

## Usage

| command / arguments                 |                                                               |
| ----------------------------------- | ------------------------------------------------------------- |
| `fx`                                | Show items in the current directory.                          |
| `fx <directory path>`               | Show items in the path. Both relative and absolute available. |
| `fx -l [path]` or `fx --log [path]` | Launch the app and create a log file.                         |
| `fx -v` or `fx --version`           | Print the current version and check update.                   |
| `fx -h` or `fx --help`              | Print help.                                                   |

## Key manual

| Key               | Explanation                                                                                                                                                                                                                                                                    |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| j / Up            | Go up. If the list exceeds max-row, it "scrolls" before the top of the list.                                                                                                                                                                                                   |
| k / Down          | Go down. If the list exceeds max-row, it "scrolls" before the bottom of the list.                                                                                                                                                                                              |
| h / Left          | Go to the parent directory if exists.                                                                                                                                                                                                                                          |
| l / Right / Enter | Open a file or change the directory. Commands for the execution can be managed in the config file.                                                                                                                                                                             |
| o                 | Open a file in a new window. This enables you to use felix while working with the file. If you open a file in an editor that runs inside the terminal, no new window appears, and after exit some error messages may appear.                                                   |
| gg                | Go to the top.                                                                                                                                                                                                                                                                 |
| G                 | Go to the bottom.                                                                                                                                                                                                                                                              |
| z + Enter         | Go to the home directory.                                                                                                                                                                                                                                                      |
| z \<keyword\>     | **_This command requires zoxide installed._** Jump to a directory that matches the keyword. Internally, felix calls [`zoxide query <keyword>`](https://man.archlinux.org/man/zoxide-query.1.en), so if the keyword does not match the zoxide database, this command will fail. |
| dd                | Delete and yank one item (it will go to the trash directory).                                                                                                                                                                                                                  |
| yy                | Yank one item. If you yanked other item(s) before, it's replaced by this one.                                                                                                                                                                                                  |
| p                 | Put yanked item(s) in the current directory. If the item with same name exists, copied item will be renamed with the suffix "\_copied".                                                                                                                                        |
| V                 | Switch to the select mode, where you can move cursor to select items.                                                                                                                                                                                                          |
| d (select mode)   | Delete and yank selected items, and return to the normal mode.                                                                                                                                                                                                                 |
| y (select mode)   | Yank selected items, and return to the normal mode.                                                                                                                                                                                                                            |
| u                 | Undo put/delete/rename.                                                                                                                                                                                                                                                        |
| Ctrl + r          | Redo put/delete/rename.                                                                                                                                                                                                                                                        |
| v                 | Toggle whether to show the item preview (text, image, or the contents tree) on the right half of the terminal. Hi-res image preview is enabled if i) your terminal supports sixel, and ii) you've preinstalled `libsixel`. If not, images are printed by blocks.               |
| backspace         | Toggle whether to show hidden items or not. This change remains after exit (stored in `.session`).                                                                                                                                                                             |
| t                 | Toggle sort order (by name <-> by modified time). This change remains after exit (same as above).                                                                                                                                                                              |
| c                 | Switch to the rename mode (enter the new name and press Enter to rename the item).                                                                                                                                                                                             |
| /                 | Switch to the filter mode (enter the keyword and press Enter to show the filtered list). Press h or Left to exit the filter mode.                                                                                                                                              |
| :                 | **_Experimantal._** Switch to the shell mode. Type command and press Enter to execute it. You can use any command in the displayed directory, but some commands may fail, and the display may collapse during execution.                                                       |
| :cd / :z          | Go to the home directory.                                                                                                                                                                                                                                                      |
| :z \<keyword\>    | Same as `z <keyword>`.                                                                                                                                                                                                                                                         |
| :e                | Reload the current directory. Useful when something goes wrong.                                                                                                                                                                                                                |
| :empty            | Empty the trash directory. **Please think twice to use this.**                                                                                                                                                                                                                 |
| :h                | Show help. (scrolls by `j/k` or `Up/Down`)                                                                                                                                                                                                                                     |
| Esc               | Return to the normal mode.                                                                                                                                                                                                                                                     |
| :q / ZZ           | Exit.                                                                                                                                                                                                                                                                          |

Note that items moved to the trash directory are prefixed with Unix time (like `1633843993`) to avoid the name conflict. This prefix will be removed when put.

## Configuration

| OS    | path                                      |
| ----- | ----------------------------------------- |
| Linux | `$XDG_CONFIG_HOME/felix`                  |
| macOS | `$HOME/Library/Application Support/felix` |

```
felix
├── config.toml # Configuration file
├── log         # log files
└── trash       # trash directory
```

Default config file, which is [here](config.toml), will be created automatically when you launch the program for the first time.

In config.toml, you can set:

- how to open files
- max length of item to be displayed (optional)
- color of directory, file, and symlink separatively
- default key to sort the item list ("Name" or "Time")

### Command settings

For example, If you write

```
default = "nvim"

[exec]
feh = ["jpg", "jpeg", "png", "gif", "svg"]
zathura = ["pdf"]
```

then, .jpg, .jpeg, .png, .gif and .svg files are opened by `feh <file-name>`, .pdf files by `zathura <file-name>` and others by `nvim <file-name>` .
