# fm

A tiny file manager with vim-like keybind.

## Installation

```
git clone https://github.com/kyoheiu/fm.git
cd fm
cargo install --path .
```

## Usage

| Key                   | Explanation                                                                                              |
| --------------------- | -------------------------------------------------------------------------------------------------------- |
| j / Key Up            | Go up. If lists exceeds max-row, lists "scrolls" before the top of the list.                             |
| k / Key Down          | Go down. If lists exceeds max-row, lists "scrolls" before the bottom of the list.                        |
| h / Key Left          | Go to parent directory if exists.                                                                        |
| l / Key Right / Enter | Open file or change directory(change lists as if `cd`). The execution commands can be managed in config. |
| g                     | Go to first line of the list.                                                                            |
| G                     | Go to last line of the list.                                                                             |
| D                     | Cut and yank item. (item will go to the trash directory)                                                 |
| y                     | Yank item. If you yanked other item before, its information will be replaced by this one.                |
| p                     | Copy yanked item to the current directory.                                                               |
| c                     | Rename item.                                                                                             |
| m                     | `mkdir`                                                                                                  |
| E                     | Empty the trash directory. Please think twice before using this command.                                 |
| /                     | Enter the filter mode. (type keyword and Enter to go to filtered list)                                   |
| H                     | Show help.                                                                                               |
