# fm

A tiny file manager with vim-like keybind.

![sample](https://github.com/kyoheiu/fm/blob/main/screenshots/sample.gif)

## Installation

```
git clone https://github.com/kyoheiu/fm.git
cd fm
cargo install --path .
```

## Usage

`fm` shows items in _the current working directory_. For example,

```
$ echo $PWD
/home/kyohei/rust/fm

$ fm
```

...shows this list.

![ss1](https://github.com/kyoheiu/fm/blob/main/screenshots/1.jpg)

`fm <whatever>` shows simple help text.

| Key                   | Explanation                                                                                                                       |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| j / Key Up            | Go up. If lists exceeds max-row, lists "scrolls" before the top of the list.                                                      |
| k / Key Down          | Go down. If lists exceeds max-row, lists "scrolls" before the bottom of the list.                                                 |
| h / Key Left          | Go to parent directory if exists.                                                                                                 |
| l / Key Right / Enter | Open file or change directory(change lists as if `cd`). The execution commands can be managed in config.                          |
| g                     | Go to the top.                                                                                                                    |
| G                     | Go to the bottom.                                                                                                                 |
| D                     | Cut and yank item. (item will go to the trash directory)                                                                          |
| y                     | Yank item. If you yanked other item before, its information will be replaced by this one.                                         |
| p                     | Copy yanked item to the current directory. If item with same name exists, copied item will be renamed with the suffix "\_copied". |
| c                     | Rename item.                                                                                                                      |
| m                     | Go to `mkdir` mode. (type name and Enter to make new one in the current directory.)                                               |
| E                     | Empty the trash directory. Please think twice before using this command.                                                          |
| /                     | Go to filter mode. (type keyword and Enter to go to filtered list)                                                                |
| Esc                   | In normal mode, exits program. In filter or mkdir mode, return to normal mode.                                                    |

## Configuration

```
config file    : $XDG_CONFIG_HOME/fm/config.toml
trash directory: $XDG_CONFIG_HOME/fm/trash
```

Default config file will be created automatically when you launch the program for the first time.
