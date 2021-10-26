pub const HELP: &str = "# fm
A simple TUI file manager with vim-like keybind.

## Usage

`fm` => Show items in current directory.
`fm <directory path>` => Show items in the path.
Both relative and absolute can be used.

### Manual

j / Key Up            :Go up.
k / Key Down          :Go down.
h / Key Left          :Go to parent directory if exists.
l / Key Right / Enter :Open file or change directory.
g                     :Go to the top.
G                     :Go to the bottom.
D                     :Delete and yank item.
y                     :Yank item.
p                     :Put yanked item to the current directory.
t                     :Toggle sort order (name <-> modified time).
c                     :Rename item.
m                     :Go to `mkdir` mode.
E                     :Empty the trash directory.
/                     :Go to filter mode.
Esc                   :Exit program or return to normal mode.
H                     :Show help.

## Configuration

config file    : $XDG_CONFIG_HOME/fm/config.toml
trash directory: $XDG_CONFIG_HOME/fm/trash

for more detail, visit:
https://github.com/kyoheiu/fm
";
