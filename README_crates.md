# *felix*

## About
A tui file manager with vim-like key mapping, written in Rust.  
Works on terminals with 21 columns or more.

[sample gif](https://github.com/kyoheiu/felix/blob/main/screenshots/sample.gif)

While heavliy inspired by the great `vifm` and trying to implement its pleasant experience in Rust, at the same time this project focuses on the following points:

- simple and fast
- easy to configure how to open files

## Release
v0.8.0 (2022-05-02)
- New command: `u` to undo and `Crtl + r` to redo. put/delete/rename are the target.

v0.7.0 (2022-04-26)
- New feature: Terminal size changes are now automatically detected and the layout is fixed.

v0.6.1 (2022-04-15)
- New configuration: You can now use the full width of terminal by setting `use_full_width` to true (false by default). I hope this wil lead to a better user experience. *For those who use <=0.6.0, felix can work without replacing config.toml because `use_full_width` is an option.*

v0.6.0 (2022-04-13)
- New command: If you have [zoxide](https://github.com/ajeetdsouza/zoxide) installed, `:z <keyword>` lets you jump to a directory that matches the keyword! For more details, see Usage.

## Status

| OS | Status |
| -- | ------ |
|Linux  | works well |
|NetBSD | works well |
|MacOS  | works, though some errors may occur. if so, let me know!|
|Windows| almost unavailable due to file-name encoding error|

## Installation

*Make sure that `gcc` is installed.*

from crates.io:

```
cargo install felix
```

from aur:

```
yay -S felix-rs
```

On NetBSD, package is available from the official repositories:

```
pkgin install felix
```

GitHub repository(develop branch):

```
git clone -b develop https://github.com/kyoheiu/felix.git
cd felix
cargo install --path .
```

## Usage

| command / arguments               |                                                               |
| --------------------- | ------------------------------------------------------------- |
| `fx`                  | Show items in the current directory.                          |
| `fx <directory path>` | Show items in the path. Both relative and absolute available. |
| `fx -c` or `fx --check` |Check update. |
| `fx -h` or `fx --help` |Print help. |

## Key manual

| Key                   | Explanation                                                                                                                                                                                                                                            |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| j / Key Up            | Go up. If the list exceeds max-row, list "scrolls" before the top of the list.                                                                                                                                                                         |
| k / Key Down          | Go down. If the list exceeds max-row, list "scrolls" before the bottom of the list.                                                                                                                                                                    |
| h / Key Left          | Go to parent directory if exists.                                                                                                                                                                                                                      |
| l / Key Right / Enter | Open file or change directory. Commands for execution can be managed in config file.                                                                                                                                                                   |
| gg                    | Go to the top.                                                                                                                                                                                                                                         |
| G                     | Go to the bottom.                                                                                                                                                                                                                                      |
| dd                    | Delete and yank item (item will go to the trash directory).                                                                                                                                                                                            |
| yy                    | Yank item. If you yanked other item before, its information is replaced by this one.                                                                                                                                                                   |
| p                     | Put yanked item(s) in the current directory. If item with same name exists, copied item will be renamed with the suffix "\_copied".                                                                                                                    |
| V                     | Switch to select mode, where you can move cursor to select items.                                                                                                                                                                                      |
| d (select mode)       | Delete and yank selected items, and return to normal mode.                                                                                                                                                                                             |
| y (select mode)       | Yank selected items, and return to normal mode.                                                                                                                                                                                                        |
| u       | Undo put/delete/rename.                                                                                                                                                                                                        |
| Ctrl + r       | Redo put/delete/rename. ***Both undo and redo are experimental: They may not work as intended, because item name may be changed to avoid name conflict when put (See `p`).***                                                                                                                                                                                                       |
| backspace                     | Toggle whether to show hidden items or not. This change remains after exit (stored in .session file in config directory).                                                                                                              |
| t                     | Toggle sort order (by name <-> by modified time). This change remains after exit (same as above).                                                                                                              |
| :                     |  ***Experimantal.*** Switch to shell mode. Type command and press Enter to execute it. You can use any command in the displayed directory, but it may fail to execute the command (e.g. `cd` doesn't work for now), and also the display of items may collapse during execution. |
| c                     | Switch to rename mode (enter new name and press Enter to rename the item).                                                                                                                                                                             |
| /                     | Switch to filter mode (enter keyword and press Enter to go to filtered list).                                                                                                                                                                          |
| Esc                   | Return to normal mode.                                                                                                                                                                                                                                 |
| :cd \| :z                   |  Go to home directory.                                                                                                                                                                                                                                 |
| :z \<keyword\>                  |  ***This command requires zoxide installed.*** Jump to a directory that matches the keyword. Internally, felix calls [`zoxide query <keyword>`](https://man.archlinux.org/man/zoxide-query.1.en), so if the keyword does not match the zoxide database, this command will fail.                                                                                                                                                                                                                                 |
| :e                    | Reload the current directory. Useful when something goes wrong in filter mode (e.g. no matches) or shell mode.                                                                                                                                         |
| :empty                | Empty the trash directory. **Please think twice before using this command.**                                                                                                                                                                           |
| :h                    | Show help.                                                                                                                                                                                                                                             |
| :q / ZZ               | Exit the program.                                                                                                                                                                                                                                      |

Note that items moved to the trash directory are prefixed with Unix time (like `1633843993`) to avoid name conflict. This prefix will be removed when paste.

## Settings

|                 |                                   |
| --------------- | --------------------------------- |
| config file     | `$XDG_CONFIG_HOME/felix/config.toml` |
| trash directory | `$XDG_CONFIG_HOME/felix/trash`       |

Default config file will be created automatically when you launch the program for the first time.

In config.toml, you can set:

- how to open file
- max length of item to be displayed (optional)
- color of directory, file, and symlink separatively
- default key to sort item list ("Name" or "Time")

### Command setting

If you write

```
default = "nvim"

[exec]
feh = ["jpg", "jpeg", "png", "gif", "svg"]
zathura = ["pdf"]
```

then, .jpg, .jpeg, .png, .gif and .svg files are opened by `feh <file-name>`, .pdf files by `zathura <file-name>` and others by `nvim <file-name>` .
