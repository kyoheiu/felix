[![crates.io](https://img.shields.io/crates/v/felix)](https://crates.io/crates/felix) ![arch linux](https://img.shields.io/archlinux/v/community/x86_64/felix-rs) ![MSRV](https://img.shields.io/badge/MSRV-1.63.0-orange)

# _felix_

A tui file manager with Vim-like key mapping, written in Rust.  
Fast, simple, and easy to configure & use.

For an overview of this app, take a look at this README, especially [key manual](#key-manual).  
For more detailed document, visit https://kyoheiu.dev/felix.

- [New release](#new-release)
- [Status](#status)
- [Installation](#installation)
- [Integrations](#integrations)
- [Usage](#usage)
  - [Key manual](#key-manual)
- [Preview feature](#preview)
- [Configuration](#configuration)

![sample](screenshots/sample.gif)

<a id="new-release"></a>

## New release

## v2.2.5 (2023-02-12)

### Added
- Allow renaming even when item name contains non-ascii chars (i.e. wide chars).
- Key command with arguments is now supported: For example,
  ```
  exec:
  'feh -.':
    [jpg, jpeg, png, gif, svg, hdr]
  ```
  this configuration enables you to execute `feh -. <item path>` by `Enter | l | Right`, or `o`.
- Check for out-of-boundary of the cursor at the top of loop.

### Fixed
- Display when using in kitty: Correctly show the cursor and preview.

For more details, see `CHANGELOG.md`.

<a id="status"></a>

## Status

| OS      | Status               |
| ------- | -------------------- |
| Linux   | works                |
| NetBSD  | works                |
| MacOS   | works                |
| Windows | not fully tested yet |

_For Windows users: From v1.3.0, it can be at least compiled on Windows (see `.github/workflows/install_test.yml`.) If you're interested, please try and report any problems._

<a id="installation"></a>

## Installation

### Prerequisites

- Make sure that `gcc` is installed.
- MSRV(Minimum Supported rustc Version): **1.60.0**

Update Rust if rustc < 1.63:

```
rustup update
```

### From crates.io

```
cargo install felix
```

### Arch Linux

```
pacman -S felix-rs
```

*The launcher binary name is `felix` via pacman.  
Alias fx='felix' if you want, as this document (and other installations) uses `fx`.*

### NetBSD

Available from the official repositories.

```
pkgin install felix
```

### From this repository

```
git clone https://github.com/kyoheiu/felix.git
cd felix
cargo install --path .
```

<a id="integrations"></a>

## Integrations

In addition, you can use felix more conveniently by installing these two apps:

- [zoxide](https://github.com/ajeetdsouza/zoxide): A smarter `cd` command, which enables you to jump to a directory that matches the keyword in felix.
- [chafa](https://hpjansson.org/chafa/): Terminal graphics for the 21st century, by which you can preview images as high-res in felix. ***chafa must be v1.10.0 or later.***

These apps do not need any configuration to use with felix!

<a id="usage"></a>

## Usage

*If you install this app via pacman, the default binary name is `felix`.*

```
`fx` => Show items in the current directory.
`fx <directory path>` => Show items in the directory.
Both relative and absolute path available.
```

### Options

```
`-h` | `--help` => Print help.
`-v` | `--version` => Check update.
`-l` | `--log` => Launch the app, automatically generating a log file in `{data_local_dir}/felix/log`.
```

<a id="key-manual"></a>

### Key manual

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
:trash            :Go to the trash directory.
:empty            :Empty the trash directory.
:h                :Show help.
:q / ZZ           :Exit.
```

<a id="preview"></a>

## Preview feature

| type | previewable |
| --- | --- |
| directory | yes |
| text | yes (and with syntax-highlighting if configured) |
| image| see below |

### Image preview feature
By default, *every terminal* can show an image by blocks (a.k.a low-res). In addition,
- iTerm2 protocol is natively supported, so if your terminal implements it, you can see a high-res image.
- By installing `chafa`, terminals that implement sixel or kitty protocol can display a high-res image.

<a id="configuration"></a>

## Configuration

### Linux

```
config file     : $XDG_CONFIG_HOME/felix/config.yaml
trash directory : $XDG_DATA_HOME/felix/Trash
log files       : $XDG_DATA_HOME/felix/log
```

### macOS

```
config file     : $HOME/Library/Application Support/felix/config.yaml
trash directory : $HOME/Library/Application Support/felix/Trash
log files       : $HOME/Library/Application Support/felix/log
```

### Windows

```
config file     : $PROFILE\AppData\Roaming\felix\config.yaml
trash directory : $PROFILE\AppData\Local\felix\Trash
log files       : $PROFILE\AppData\Local\felix\log
```

For more details, visit https://kyoheiu.dev/felix.
