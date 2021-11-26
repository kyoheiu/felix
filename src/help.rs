pub const HELP: &str = "# felix
A simple TUI file manager with vim-like keymapping.
This program works on terminals with 21 columns or more.

## Usage

`fx` => Show items in current directory.
`fx <directory path>` => Show items in the path.
Both relative and absolute available.

### Manual

j / Key Up            :Go up.
k / Key Down          :Go down.
h / Key Left          :Go to parent directory if exists.
l / Key Right / Enter :Open file or change directory.
gg                    :Go to the top.
G                     :Go to the bottom.
dd                    :Delete and yank item.
yy                    :Yank item.
p                     :Put yanked item in the current directory.
V                     :Switch to select mode.
  - d                 :In select mode, delete and yank selected items.
  - y                 :In select mode, yank selected items.
t                     :Toggle sort order (name <-> modified time).
:                     :Switch to shell mode.
c                     :Switch to rename mode.
/                     :Switch to filter mode.
Esc                   :Return to normal mode.
:e                    :Reload the current directory.
:empty                :Empty the trash directory.
:h                    :Show help.
:q / ZZ               :Exit the program.

## Configuration

config file    : $XDG_CONFIG_HOME/felix/config.toml
trash directory: $XDG_CONFIG_HOME/felix/trash

for more detail, visit:
https://github.com/kyoheiu/felix
";
