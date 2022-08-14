[![crates.io](https://img.shields.io/crates/v/felix)](https://crates.io/crates/felix) ![aur:felix-rs](https://img.shields.io/aur/version/felix-rs)

# _felix_

A tui file manager with vim-like key mapping, written in Rust. Fast, simple, and easy to configure & use.

For the detailed document, please see https://kyoheiu.dev/felix.

![sample](screenshots/sample.gif)

## New Release

## v1.1.1 (2022-08-11)

### Fixed

- In the filter mode and shell mode, when you don't have any input, `backspace` now means return to the normal mode.
- Also, during the filter mode, `Esc` now restores the cursor position.

## v1.1.0 (2022-08-08)

### Important change about the preview feature

- From v1.1.0, felix uses [hpjansson](https://github.com/hpjansson) /
  [chafa](https://github.com/hpjansson/chafa) instead of `libsixel` & `viuer` to preview image files. This greatly improves the performance and code maintainability, and as a consequence, the number of dependencies is reduced (137 -> 53).
- Due to this change, the image preview does not work out of the box: **_Install `chafa` and it will be enabled without configuration_**. To install, please see https://hpjansson.org/chafa/.
- By `chafa`, the high-res image preview is enabled in terminals that support sixel, or kitty.
- In other terminals, images are displayed by characters.

For more details, see `CHANGELOG.md`.

## Status

| OS      | Status                           |
| ------- | -------------------------------- |
| Linux   | works                            |
| NetBSD  | works                            |
| MacOS   | works (tested only on Intel Mac) |
| Windows | not supported yet                |

## Installation

_Make sure that `gcc` is installed._

From crates.io:

```
cargo install felix
```

From AUR:

```
yay -S felix-rs
```

On NetBSD, package is available from the official repositories:

```
pkgin install felix
```

From this repository:

```
git clone https://github.com/kyoheiu/felix.git
cd felix
cargo install --path .
```

## Usage

```
`fx` => Show items in the current directory.
`fx <directory path>` => Show items in the path.
Both relative and absolute path available.
```

## Options

```
`-h` | `--help` => Print help.
`-v` | `--version` => Check update.
`-l [path]` | `--log [path]` => Launch the app and create a log file.
```

## Key Manual

```
j / Up            :Go up.
k / Down          :Go down.
h / Left          :Go to the parent directory if exists.
l / Right / Enter :Open a file or change directory.
o                 :Open a fila in a new window.
gg                :Go to the top.
G                 :Go to the bottom.
z + Enter         :Go to the home directory.
z <keyword>       :Jump to a directory that matches the keyword. (zoxide* required)
dd                :Delete and yank one item.
yy                :Yank one item.
p                 :Put yanked item(s) in the current directory.
V                 :Switch to the select mode.
  - d             :In the select mode, delete and yank selected item(s).
  - y             :In the select mode, yank selected item(s).
u                 :Undo put/delete/rename.
Ctrl + r          :Redo put/delete/rename.
v                 :Toggle whether to show the preview.
backspace         :Toggle whether to show hidden items.
t                 :Toggle the sort order (name <-> modified time).
:                 :Switch to the shell mode.
c                 :Switch to the rename mode.
/                 :Switch to the filter mode.
Esc               :Return to the normal mode.
:cd / :z          :Go to the home directory.
:z <keyword>      :Same as `z <keyword>`.
:e                :Reload the current directory.
:empty            :Empty the trash directory.
:h                :Show help.
:q / ZZ           :Exit.
```

\* zoxide https://github.com/ajeetdsouza/zoxide

## Preview feature

By default, text files and directories can be previewed.
To preview images, you need to install chafa.
Please see https://hpjansson.org/chafa/.

## Configuration

### Linux

config file : $XDG_CONFIG_HOME/felix/config.toml
trash directory: $XDG_CONFIG_HOME/felix/trash
log files : \$XDG_CONFIG_HOME/felix/log

### macOS

config file : $HOME/Library/Application Support/felix/config.toml
trash directory: $HOME/Library/Application Support/felix/trash
log files : \$HOME/Library/Application Support/felix/log

For more details, visit https://kyoheiu.dev/felix.
