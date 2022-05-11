[![crates.io](https://img.shields.io/crates/v/felix)](https://crates.io/crates/felix) ![aur:felix-rs](https://img.shields.io/aur/version/felix-rs)

# _felix_

## About

A tui file manager with vim-like key mapping, written in Rust.

![sample](https://github.com/kyoheiu/felix/blob/main/screenshots/sample.gif)

While heavliy inspired by the great `vifm` and trying to implement its pleasant experience in Rust, at the same time this project focuses on the following points:

- simple and fast
- easy to configure how to open files

## New Release

v0.9.1 (2022-05-11)
- Fix bug that after `:h`, cursor move can cause unexpected panic.
- Wrap preview text.

For more detail, see `CHANGELOG.md`.

## Status

| OS      | Status                                                   |
| ------- | -------------------------------------------------------- |
| Linux   | works well                                               |
| NetBSD  | works well                                               |
| MacOS   | works, though some errors may occur. if so, let me know! |
| Windows | almost unavailable due to file-name encoding error       |

## Installation

_Make sure that `gcc` is installed._

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

from this repository(develop branch):

```
git clone -b develop https://github.com/kyoheiu/felix.git
cd felix
cargo install --path .
```

## Usage

| command / arguments     |                                                               |
| ----------------------- | ------------------------------------------------------------- |
| `fx`                    | Show items in the current directory.                          |
| `fx <directory path>`   | Show items in the path. Both relative and absolute available. |
| `fx -c` or `fx --check` | Check update.                                                 |
| `fx -h` or `fx --help`  | Print help.                                                   |

## Key manual

| Key                   | Explanation                                                                                                                                                                                                                                                                     |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| j / Key Up            | Go up. If the list exceeds max-row, list "scrolls" before the top of the list.                                                                                                                                                                                                  |
| k / Key Down          | Go down. If the list exceeds max-row, list "scrolls" before the bottom of the list.                                                                                                                                                                                             |
| h / Key Left          | Go to parent directory if exists.                                                                                                                                                                                                                                               |
| l / Key Right / Enter | Open file or change directory. Commands for execution can be managed in config file.                                                                                                                                                                                            |
| gg                    | Go to the top.                                                                                                                                                                                                                                                                  |
| G                     | Go to the bottom.                                                                                                                                                                                                                                                               |
| dd                    | Delete and yank item (item will go to the trash directory).                                                                                                                                                                                                                     |
| yy                    | Yank item. If you yanked other item before, its information is replaced by this one.                                                                                                                                                                                            |
| p                     | Put yanked item(s) in the current directory. If item with same name exists, copied item will be renamed with the suffix "\_copied".                                                                                                                                             |
| V                     | Switch to select mode, where you can move cursor to select items.                                                                                                                                                                                                               |
| d (select mode)       | Delete and yank selected items, and return to normal mode.                                                                                                                                                                                                                      |
| y (select mode)       | Yank selected items, and return to normal mode.                                                                                                                                                                                                                                 |
| u                     | Undo put/delete/rename.                                                                                                                                                                                                                                                         |
| Ctrl + r              | Redo put/delete/rename.                                                                                                                                                                                                                                                         |
| v                     | Toggle whether to show preview (part of the content for text file, contents tree for directory) on the right half of the terminal. The previewed text is unwrapped and static for now.                                                                                                                                              |
| backspace             | Toggle whether to show hidden items or not. This change remains after exit (stored in .session file in config directory).                                                                                                                                                       |
| t                     | Toggle sort order (by name <-> by modified time). This change remains after exit (same as above).                                                                                                                                                                               |
| :                     | **_Experimantal._** Switch to shell mode. Type command and press Enter to execute it. You can use any command in the displayed directory, but it may fail to execute the command (e.g. `cd` doesn't work for now), and also the display of items may collapse during execution. |
| c                     | Switch to rename mode (enter new name and press Enter to rename the item).                                                                                                                                                                                                      |
| /                     | Switch to filter mode (enter keyword and press Enter to go to filtered list).                                                                                                                                                                                                   |
| Esc                   | Return to normal mode.                                                                                                                                                                                                                                                          |
| :cd \| :z             | Go to home directory.                                                                                                                                                                                                                                                           |
| :z \<keyword\>        | **_This command requires zoxide installed._** Jump to a directory that matches the keyword. Internally, felix calls [`zoxide query <keyword>`](https://man.archlinux.org/man/zoxide-query.1.en), so if the keyword does not match the zoxide database, this command will fail.  |
| :e                    | Reload the current directory. Useful when something goes wrong in filter mode (e.g. no matches) or shell mode.                                                                                                                                                                  |
| :empty                | Empty the trash directory. **Please think twice before using this command.**                                                                                                                                                                                                    |
| :h                    | Show help. (scrollable with `j` | `k` | `Up` | `Down`)                                                                                                                                                                                                                                                                   |
| :q / ZZ               | Exit the program.                                                                                                                                                                                                                                                               |

Note that items moved to the trash directory are prefixed with Unix time (like `1633843993`) to avoid name conflict. This prefix will be removed when paste.

## Settings

|                 |                                      |
| --------------- | ------------------------------------ |
| config file     | `$XDG_CONFIG_HOME/felix/config.toml` |
| trash directory | `$XDG_CONFIG_HOME/felix/trash`       |

Default config file, which is [here](config.toml), will be created automatically when you launch the program for the first time.

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
