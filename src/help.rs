pub const HELP: &str = "# fm
A simple TUI file manager with vim-like keybind.

## Usage

`fm` => Show items in current directory.
`fm <directory path>` => Show items in the path.
Both relative and absolute available.

### Manual

j / Key Up            :Go up.
k / Key Down          :Go down.
h / Key Left          :Go to parent directory if exists.
l / Key Right / Enter :Open file or change directory.
g                     :Go to the top.
G                     :Go to the bottom.
D                     :Delete and yank item.
y                     :Yank item.
p                     :Put yanked item in the current directory.
Ctrl+c                :Copy file name to clipboard.
t                     :Toggle sort order (name <-> modified time).
:                     :Switch to shell mode.
c                     :Switch to rename mode.
m                     :Switch to `mkdir` mode.
/                     :Switch to filter mode.
E                     :Empty the trash directory.
Esc                   :Exit program or return to normal mode.
H                     :Show help.

## Configuration

config file    : $XDG_CONFIG_HOME/fm/config.toml
trash directory: $XDG_CONFIG_HOME/fm/trash

for more detail, visit:
https://github.com/kyoheiu/fm
";
