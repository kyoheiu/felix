/// Help text.
pub const HELP: &str = "# felix v2.0.1
A simple TUI file manager with vim-like keymapping.

## Usage
`fx` => Show items in the current directory.
`fx <directory path>` => Show items in the path.
Both relative and absolute path available.

## Arguments
`-h` | `--help`    => Print help.
`-v` | `--version` => Check update.
`-l [path]` | `--log [path]` => Launch the app and create a log file.

## Manual
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

## Preview feature
By default, text files and directories can be previewed.
To preview images, you need to install chafa.
Please see https://hpjansson.org/chafa/

## Configuration
### Linux
config file    : $XDG_CONFIG_HOME/felix/config.yaml
trash directory: $XDG_CONFIG_HOME/felix/trash
log files      : $XDG_CONFIG_HOME/felix/log

### macOS
config file    : $HOME/Library/Application Support/felix/config.yaml
trash directory: $HOME/Library/Application Support/felix/trash
log files      : $HOME/Library/Application Support/felix/log

### Windows
config file     : $PROFILE\\AppData\\Roaming\\felix\\config.yaml
trash directory : $PROFILE\\AppData\\Roaming\\felix\\trash
log files       : $PROFILE\\AppData\\Roaming\\felix\\log

For more details, visit https://github.com/kyoheiu/felix
";
