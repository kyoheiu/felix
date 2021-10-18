# fm

A simple TUI file manager with vim-like keybind, written in Rust.
(_Currently works on Linux_)

![sample](https://github.com/kyoheiu/fm/blob/main/screenshots/sample.gif)

My aim is to make a file manager that enables you to:

- open files in the current directory as fast as you can
- configurate easily
- do what you want to in daily use

And with `fm`, you can:

- see items in the current directory when launch
- set your own commands to open file inside this program with `config.toml`, which is so handy!
- filter the list and choose item easily
- delete
- yank & put
- rename
- make a new directory
- empty the trash directory

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

...shows this list. (Left: file-name, Right: modified time)

![ss1](https://github.com/kyoheiu/fm/blob/main/screenshots/1.jpg)

`fm <whatever>` shows simple help text.

## Key manual

| Key                   | Explanation                                                                                                                      |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| j / Key Up            | Go up. If the list exceeds max-row, list "scrolls" before the top of the list.                                                   |
| k / Key Down          | Go down. If the list exceeds max-row, list "scrolls" before the bottom of the list.                                              |
| h / Key Left          | Go to parent directory if exists.                                                                                                |
| l / Key Right / Enter | Open file or change directory(change lists as if `cd`). The exec commands can be managed in config.                              |
| g                     | Go to the top.                                                                                                                   |
| G                     | Go to the bottom.                                                                                                                |
| D                     | Delete and yank item. (item will go to the trash directory)                                                                      |
| y                     | Yank item. If you yanked other item before, its information will be replaced by this one.                                        |
| p                     | Put yanked item to the current directory. If item with same name exists, copied item will be renamed with the suffix "\_copied". |
| c                     | Rename item.                                                                                                                     |
| m                     | Switch to `mkdir` mode. (type name and Enter to make new one in the current directory.)                                          |
| E                     | Empty the trash directory. **Please think twice before using this command.**                                                     |
| /                     | Switch to filter mode. (type keyword and Enter to go to filtered list)                                                           |
| Esc                   | In normal mode, exits program. In filter or `mkdir` mode, return to normal mode.                                                 |

Note that items moved to the trash directory will be prefixed with Unix time (like `1633843993`) to avoid name conflict. This prefix will be removed when paste.

## Configuration

```
config file    : $XDG_CONFIG_HOME/fm/config.toml
trash directory: $XDG_CONFIG_HOME/fm/trash
```

Default config file, which is [here](src/config.rs), will be created automatically when you launch the program for the first time.

In config.toml, you can set:

- color of directory name
- color of file name
- how to open file

For example, if you write

```
[exec]
default = "nvim"
jpg = "feh"
```

...then `fm` opens jpg file, say `01.jpg`, by `feh 01.jpg`, and the other items by `nvim <file-name>`. The execution is inside `fm`, so you can return to the list right after closing file.

## todo (or not todo)

- [ ] easier way to configurate exec commands
- [ ] choose whether the warning appears or not when delete
- [ ] change sort order (new to old / old to new)
- [ ] implement `v`(select mode)
