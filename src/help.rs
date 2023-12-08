/// Help text.
pub const HELP: &str = concat!(
    "# felix v",
    env!("CARGO_PKG_VERSION"),
    r###"
A simple TUI file manager with vim-like keymapping.

## Usage
`fx` => Show items in the current directory.
`fx <directory path>` => Show items in the path.
Both relative and absolute path available.

## Options
`--help` | `-h`   => Print help.
`--log`  | `-l`   => Launch the app, automatically generating a log file.
`--init`          => Returns a shell script that can be sourcedfor
                     for shell integration.

## Manual
j / <Down>         :Go down.
k / <Up>           :Go up.
h / <Left>         :Go to the parent directory if exists.
l / <Right> / <CR> :Open item or change directory.
gg                 :Go to the top.
G                  :Go to the bottom.
z<CR>              :Go to the home directory.
z {keyword}<CR>    :Jump to a directory that matches the keyword.
                    (zoxide required)
<C-o>              :Jump backward.
<C-i>              :Jump forward.
i{file name}<CR>   :Create a new empty file.
I{dir name}<CR>    :Create a new empty directory.
o                  :Open item in a new window.
e                  :Unpack archive/compressed file.
dd                 :Delete and yank item.
yy                 :Yank item.
p                  :Put yanked item(s) from register zero
                    in the current directory.
:reg               :Show registers. To hide it, press v.
"ayy               :Yank item to register a.
"add               :Delete and yank item to register a.
"Ayy               :Append item to register a.
"Add               :Delete and append item to register a.
"ap                :Put item(s) from register a.
V                  :Switch to the linewise visual mode.
  - y              :In the visual mode, yank selected item(s).
  - d              :In the visual mode, delete and yank selected item(s).
  - "ay            :In the visual mode, yank items to register a.
  - "ad            :In the visual mode, delete and yank items to register a.
  - "Ay            :In the visual mode, append items to register a.
  - "Ad            :In the visual mode, delete and append items to register a.
u                  :Undo put/delete/rename.
<C-r>              :Redo put/delete/rename.
v                  :Toggle whether to show the preview.
s                  :Toggle between vertical / horizontal split in the preview mode.
<Alt-j>
 / <Alt-<Down>>    :Scroll down the preview text.
<Alt-k> 
 / <Alt-<Up>>      :Scroll up the preview text.
<BS>               :Toggle whether to show hidden items.
t                  :Toggle the sort order (name <-> modified time).
c                  :Switch to the rename mode.
/{keyword}         :Search items by a keyword.
n                  :Go forward to the item that matches the keyword.
N                  :Go backward to the item that matches the keyword.
:                  :Switch to the command line.
  - <C-r>a         :In the command line, paste item name in register a.
:cd<CR>            :Go to the home directory.
:cd {path}<CR>     :Go to the path.
:e<CR>             :Reload the current directory.
:trash<CR>         :Go to the trash directory.
:empty<CR>         :Empty the trash directory.
:h<CR>             :Show help.
:q<CR>             :Exit.
:{command}         :Execute a command e.g. :zip test *.md
<Esc>              :Return to the normal mode.
<C-h>              :Works as Backspace after `i`, `I`, `c`, `/`, `:` and `z`.
ZZ                 :Exit without cd to last working directory
                    (if `match_vim_exit_behavior` is `false`).
ZQ                 :cd into the last working directory and exit
                    (if shell setting is ready and `match_vim_exit_behavior is `false`).

## Preview feature
By default, text files and directories can be previewed.
To preview images, you need to install chafa (>= v1.10.0).
Please see https://hpjansson.org/chafa/

## Configuration

*Both `config.yaml` and `config.yml` work.*

### Linux
config file    : $XDG_CONFIG_HOME/felix/config.yaml(config.yml)
trash directory: $XDG_DATA_HOME/felix/trash
log files      : $XDG_DATA_HOME/felix/log

### macOS
On macOS, felix looks for the config file in the following locations:

1. `$HOME/Library/Application Support/felix/config.yaml(config.yml)`
2. `$HOME/.config/felix/config.yaml`

trash directory: $HOME/Library/Application Support/felix/trash
log files      : $HOME/Library/Application Support/felix/log

### Windows
config file     : $PROFILE\\AppData\\Roaming\\felix\\config.yaml(config.yml)
trash directory : $PROFILE\\AppData\\Local\\felix\\trash
log files       : $PROFILE\\AppData\\Local\\felix\\log

For more details, visit https://github.com/kyoheiu/felix
"###
);
