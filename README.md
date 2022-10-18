[![crates.io](https://img.shields.io/crates/v/felix)](https://crates.io/crates/felix) ![aur:felix-rs](https://img.shields.io/aur/version/felix-rs) ![MSRV](https://img.shields.io/badge/MSRV-1.59.0-orange)

# _felix_

A tui file manager with vim-like key mapping, written in Rust. Fast, simple, and easy to configure & use.

For the detailed document, please see https://kyoheiu.dev/felix.

![sample](screenshots/sample.gif)

## New Release

## v1.3.0 (2022-10-18)

### Changed

- Huge refactoring: Migrated to crossterm from termion due to the maintainability and future-support for Windows. New module `term.rs` contains (almost) all of the terminal API, so that other modules will not get effected by the future backend change.
  - Alongside, some changes are added to show the file path properly in Windows.
  - With crossterm, opening a file in e.g. Vim, it feels as if this app "freezes". This behavior is not what I want, so from v1.3.0, `open_file_in_new_window` can work only if \[exec\] is set in config file, and the extension of the item matches the key.
  - As a result of this migration, the number of dependencies increased to 66 from 51.
- `default` key in the config file become `Option`, so that users can select \$EDITOR without explicitly setting it up. The initial process of asking users to select the default command has also been fixed accordingly.

### Fixed

- After zoxide jump, turn off the filter mode.
- Many typos fixed.

### Added

- New error: `OpenNewWindow`
- `Dockerfile` and `.dockerignore`, to test the cross-compiling for Windows

## Status

| OS      | Status                  |
| ------- | ----------------------- |
| Linux   | works                   |
| NetBSD  | works                   |
| MacOS   | works                   |
| Windows | not fully supported yet |

_For Windows users: At least this app can be cross-compiled for Windows. See `Dockerfile`. Please try the native build and report any problems._

MSRV(Minimum Supported Rust Version): **1.59.0**

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

## Integrations

In addition, you can use felix more conveniently by installing these two apps:

- [zoxide](https://github.com/ajeetdsouza/zoxide): A smarter `cd` command, which enables you to jump to a directory that matches the keyword in felix.
- [chafa](https://hpjansson.org/chafa/): Terminal graphics for the 21st century, by which you can preview images in felix.

These apps do not need any configuration to use with felix!

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
o                 :Open a file in a new window.
gg                :Go to the top.
G                 :Go to the bottom.
z + Enter         :Go to the home directory.
z <keyword>       :Jump to a directory that matches the keyword. (zoxide required)
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

## Preview feature

By default, text files and directories can be previewed.  
Install `chafa` and you can preview images without any configuration.

## Configuration

### Linux

```
config file     : $XDG_CONFIG_HOME/felix/config.toml
trash directory : $XDG_CONFIG_HOME/felix/trash
log files       : \$XDG_CONFIG_HOME/felix/log
```

### macOS

```
config file     : $HOME/Library/Application Support/felix/config.toml
trash directory : $HOME/Library/Application Support/felix/trash
log files       : \$HOME/Library/Application Support/felix/log
```

### Windows

```
config file     : $PROFILE\\AppData\\Roaming\\felix\\config.toml
trash directory : $PROFILE\\AppData\\Roaming\\felix\\trash
log files       : $PROFILE\\AppData\\Roaming\\felix\\log
```

For more details, visit https://kyoheiu.dev/felix.
