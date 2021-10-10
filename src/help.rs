pub const HELP: &str = "# fm
A tiny file manager with vim-like keybind.

## Usage

When `fm` starts,
it shows items in the current working directory.

### Manual

`fm`                  :Just start the program.
j / Key Up            :Go up.
k / Key Down          :Go down.
h / Key Left          :Go to parent directory if exists.
l / Key Right / Enter :Open file or change directory.
g                     :Go to the top.
G                     :Go to the bottom.
D                     :Cut and yank item.
y                     :Yank item.
p                     :Copy yanked item to the current directory.
c                     :Rename item.
m                     :Go to `mkdir` mode.
E                     :Empty the trash directory.
/                     :Go to filter mode.
Esc                   :Exits program or return to normal mode.
`fm <whatever>`       :Show help.

## Configuration

config file    : $XDG_CONFIG_HOME/fm/config.toml
trash directory: $XDG_CONFIG_HOME/fm/trash

for more detail, visit
https://github.com/kyoheiu/fm
";
