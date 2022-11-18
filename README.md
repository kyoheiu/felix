[![crates.io](https://img.shields.io/crates/v/felix)](https://crates.io/crates/felix) ![aur:felix-rs](https://img.shields.io/aur/version/felix-rs) ![MSRV](https://img.shields.io/badge/MSRV-1.60.0-orange)

# _felix_

A tui file manager with vim-like key mapping, written in Rust.  
Fast, simple, and easy to configure & use.

For the detailed document, please see https://kyoheiu.dev/felix.

![sample](screenshots/sample.gif)

## New Release

## v2.1.0 (2022-11-19)

### Added

![sample_extraction](screenshots/sample_extraction.gif)

- Feature to unpack archive/compressed file to the current directory. Supported types: `tar.gz`(Gzip), `tar.xz`(lzma), `tar.zst`(Zstandard & tar), `zst`(Zstandard), `tar`, zip file format and formats based on it(`zip`, `docx`, ...). To unpack, press `e` on the item.
  - The number of dependencies bumps up to around 150 due to this.

### Fixed

- Bug: In the select mode, the selected item was unintentionally canceled when going up/down.
- Delete pointer properly when removing items.
- Instead of panic, return error when `config_dir()` fails.

### Changed

- Image file detection: Use magic bytes instead of checking the extension. This will enable to detect image more precisely.

For more details, see `CHANGELOG.md`.

## Status

| OS      | Status               |
| ------- | -------------------- |
| Linux   | works                |
| NetBSD  | works                |
| MacOS   | works                |
| Windows | not fully tested yet |

_For Windows users: From v1.3.0, it can be at least compiled on Windows (see `.github/workflows/install_test.yml`.) If you're interested, please try and report any problems._

MSRV(Minimum Supported rustc Version): **1.60.0**

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
j / Down          :Go down.
k / Up            :Go up.
h / Left          :Go to the parent directory if exists.
l / Right / Enter :Open a file or change directory.
o                 :Open a file in a new window.
e                 :Unpack archive/compressed file.
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
s                 :Toggle between vertical / horizontal split in the preview mode.
Alt + j / Down    :Scroll down the preview text.
Alt + k / Up      :Scroll up the preview text.
backspace         :Toggle whether to show hidden items.
t                 :Toggle the sort order (name <-> modified time).
:                 :Switch to the shell mode.
c                 :Switch to the rename mode.
/                 :Search items by the keyword.
n                 :Go forward to the item that matches the keyword.
N                 :Go backward to the item that matches the keyword.
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
config file     : $XDG_CONFIG_HOME/felix/config.yaml
trash directory : $XDG_CONFIG_HOME/felix/trash
log files       : \$XDG_CONFIG_HOME/felix/log
```

### macOS

```
config file     : $HOME/Library/Application Support/felix/config.yaml
trash directory : $HOME/Library/Application Support/felix/trash
log files       : \$HOME/Library/Application Support/felix/log
```

### Windows

```
config file     : $PROFILE\AppData\Roaming\felix\config.yaml
trash directory : $PROFILE\AppData\Roaming\felix\trash
log files       : $PROFILE\AppData\Roaming\felix\log
```

For more details, visit https://kyoheiu.dev/felix.
