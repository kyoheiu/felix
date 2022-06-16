/// Help text.
pub const HELP: &str = "# felix v0.9.6
A simple TUI file manager with vim-like keymapping.

## Usage
`fx` => Show items in the current directory.
`fx <directory path>` => Show items in the path.
Both relative and absolute path available.

## Arguments
`fx -h` | `fx --help`    => Print help.
`fx -v` | `fx --version` => Check update.
`fx -l [dir path]` => Launch the app and create a log file in `$XDG_CONFIG_HOME/felix/log`.

## Manual
j / Up            :Go up.
k / Down          :Go down.
h / Left          :Go to the parent directory if exists.
l / Right / Enter :Open a file or change directory.
o                 :Open a fila in a new window. 
gg                :Go to the top.
G                 :Go to the bottom.
z + Enter         :Go to the home directory.
z <keyword>       :*zoxide required* Jump to a directory that matches the keyword.
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

## Configuration
config file    : $XDG_CONFIG_HOME/felix/config.toml
trash directory: $XDG_CONFIG_HOME/felix/trash

For more detail, visit https://github.com/kyoheiu/felix
";
