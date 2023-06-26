/// Help text.
pub const HELP: &str = r###"# felix v2.4.1
A simple TUI file manager with vim-like keymapping.

## Usage
`fx` => Show items in the current directory.
`fx <directory path>` => Show items in the path.
Both relative and absolute path available.

## Options
`--help` | `-h`   => Print help.
`--log`  | `-l`   => Launch the app, automatically generating a log file.
`--init`          => Returns a shell script that can be sourced for shell integration.

## Manual
j / Down          :Go down.
k / Up            :Go up.
h / Left          :Go to the parent directory if exists.
l / Right / Enter :Open item or change directory.
gg                :Go to the top.
G                 :Go to the bottom.
z + Enter         :Go to the home directory.
z <keyword>       :Jump to a directory that matches the keyword. (zoxide required)
o                 :Open item in a new window.
e                 :Unpack archive/compressed file.
dd                :Delete and yank item.
yy                :Yank item.
p                 :Put yanked item(s) in the current directory.
:reg              :Show registers. To hide it, press v.
"ayy              :Yank item to register a.
"add              :Delete and yank item to register a.
"Ayy              :Append item to register a.
"Add              :Delete and append item to register a.
"ap               :Put item(s) from register a.
V                 :Switch to the linewise visual mode.
  - y             :In the visual mode, yank selected item(s).
  - d             :In the visual mode, delete and yank selected item(s).
  - "ay           :In the visual mode, yank items to register a.
  - "ad           :In the visual mode, delete and yank items to register a.
  - "Ay           :In the visual mode, append items to register a.
  - "Ad           :In the visual mode, delete and append items to register a.
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
/                 :Search items by a keyword.
n                 :Go forward to the item that matches the keyword.
N                 :Go backward to the item that matches the keyword.
Esc               :Return to the normal mode.
:cd               :Go to the home directory.
:e                :Reload the current directory.
:trash            :Go to the trash directory.
:empty            :Empty the trash directory.
:h                :Show help.
:q                :Exit.
ZZ                :Exit without cd to last working directory (if `match_vim_exit_behavior` is `false`).
ZQ                :cd into the last directory and exit (if `match_vim_exit_behavior` is `false`).

## Preview feature
By default, text files and directories can be previewed.
To preview images, you need to install chafa (>= v1.10.0).
Please see https://hpjansson.org/chafa/

## Configuration
### Linux
config file    : $XDG_CONFIG_HOME/felix/config.yaml
trash directory: $XDG_DATA_HOME/felix/trash
log files      : $XDG_DATA_HOME/felix/log

### macOS
On macOS, felix looks for the config file in the following locations:

1. `$HOME/Library/Application Support/felix/config.yaml`
2. `$HOME/.config/felix/config.yaml`

trash directory: $HOME/Library/Application Support/felix/trash
log files      : $HOME/Library/Application Support/felix/log

### Windows
config file     : $PROFILE\\AppData\\Roaming\\felix\\config.yaml
trash directory : $PROFILE\\AppData\\Local\\felix\\trash
log files       : $PROFILE\\AppData\\Local\\felix\\log

For more details, visit https://github.com/kyoheiu/felix
"###;
