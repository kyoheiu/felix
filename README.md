# fm

A simple TUI file manager with vim-like keybind, written in Rust.
(_Currently works on Linux_)

![sample](https://github.com/kyoheiu/fm/blob/main/screenshots/sample.gif)

My aim is to make a file manager that enables you to:

- open files in the current directory as fast as you can
- configurate easily
- do what you want in daily use

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
cargo +nightly install --path .
```

## Usage

`fm` shows items in the current working directory. For example,

```
$ echo $PWD
/home/kyohei/rust/fm

$ fm
```

...shows this list. (Left: file-name, Right: modified time)

![ss1](https://github.com/kyoheiu/fm/blob/main/screenshots/1.jpg)

`fm <whatever>` shows simple help text.

## Key manual

| Key                   | Explanation                                                                                                                                               |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| j / Key Up            | Go up. If the list exceeds max-row, list "scrolls" before the top of the list.                                                                            |
| k / Key Down          | Go down. If the list exceeds max-row, list "scrolls" before the bottom of the list.                                                                       |
| h / Key Left          | Go to parent directory if exists.                                                                                                                         |
| l / Key Right / Enter | Open file or change directory. The exec commands can be managed in config.                                                                                |
| g                     | Go to the top.                                                                                                                                            |
| G                     | Go to the bottom.                                                                                                                                         |
| D                     | Delete and yank item (item will go to the trash directory).                                                                                               |
| y                     | Yank item. If you yanked other item before, its information is replaced by this one.                                                                      |
| p                     | Put yanked item to the current directory. If item with same name exists, copied item will be renamed with the suffix "\_copied".                          |
| t                     | Change sort order (by name <-> by modified time). This change remains until the program ends (After that, the sort order will be restored as configured). |
| c                     | Rename item.                                                                                                                                              |
| m                     | Switch to `mkdir` mode (type name and Enter to make new directory in the current dir.).                                                                   |
| E                     | Empty the trash directory. **Please think twice before using this command.**                                                                              |
| /                     | Switch to filter mode (type keyword and Enter to go to filtered list).                                                                                    |
| Esc                   | In normal mode, exit program. In rename, `mkdir` or filter mode, return to normal mode.                                                                   |

Note that items moved to the trash directory are prefixed with Unix time (like `1633843993`) to avoid name conflict. This prefix will be removed when paste.

## Configuration

```
config file    : $XDG_CONFIG_HOME/fm/config.toml
trash directory: $XDG_CONFIG_HOME/fm/trash
```

Default config file, which is [here](config.toml), will be created automatically when you launch the program for the first time.

In config.toml, you can configurate:

- color of directory, file, and symlink separatively
- whether you see the warning message when delete item
- default key for sorting item list ("Name" or "Time")
- how to open file

### Command configuration

If you write

```
default = "nvim"

[exec]
feh = ["jpg", "jpeg", "png", "gif", "svg"]
zathura = ["pdf"]
```

...then, .jpg, .jpeg, .png, .gif and .svg files are are opened by `feh <file-name>`, .pdf files by `zathura <file-name>` and others by `nvim <file-name>` .

## todo (or not todo)

- [x] easier way to configurate exec commands
- [x] choose whether the warning appears or not when delete
- [x] change sort order (file name / new to old)
- [ ] implement `v`(select mode)
