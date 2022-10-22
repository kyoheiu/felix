# Changelog

## Notes

- Text preview color needs to be readable enough, so it's worth rethinking (Now LightBlack).

## Unreleased

### Fixed
- Remove debug print in `make_config_if_not_exists`

## v1.3.2 (2022-10-23)

### Added

- Add `std::panic::catch_unwind` to manually restore after a panic rewind. This allows the cursor to be restored and the screen cleared when this app panics.

### Fixed

- Fixed: Similar to v1.3.1, attempting to preview a symbolic link to a nonexistent file caused a panic. Now the preview shows `(file not readable)` for such a link.

## v1.3.1 (2022-10-21)

### Fixed

- Attempting to preview a symbolic link to a directory caused a panic. It has been fixed and the preview will now show the contents of the linked directory.

## v1.3.0 (2022-10-18)

### Changed

- Huge refactoring: Migrated to crossterm from termion due to the maintainability and future-support for Windows. New module `term.rs` contains (almost) all of the terminal API, so that other modules will not get effected by the future backend change.
  - Alongside, some changes are added to show the file path properly in Windows.
  - With crossterm, opening a file in e.g. Vim, it feels as if this app "freezes". This behavior is not what I want, so from v1.3.0, `open_file_in_new_window` can work only if \[exec\] is set in config file, and the extension of the item matches the key.
- `default` key in the config file become `Option`, so that users can select `$EDITOR` without explicitly setting it up. The initial process of asking users to select the default command has also been fixed accordingly.

### Fixed

- After zoxide jump, turn off the filter mode.
- Many typos fixed.

### Added

- New error: `OpenNewWindow`
- New GitHub actions: Add windows-install

## v1.2.0 (2022-10-01)

### Changed

- Huge refactoring: Instead of `thiserror`, use custom error type to make it easier to handle.
- Bump up chrono version to 0.4.22, clarifing the feature to use.
- Avoid extra heap allocation by using write! instead of push_str/format!.
- Copied item will be renamed with the suffix "\_{count}" such as "test_1.txt", instead of "test_copied.txt".

### Fixed

- Choose `None` for directory extension.

## v1.1.2 (2022-08-29)

### Fixed

- Use full width of the terminal when `use_full_width` in config.toml is not set.
- Use `cursor::Goto` instead of `cursor::Left` to fix the layout in macOS Terminal.app.
- Refactor functions around the layout.

## v1.1.1 (2022-08-11)

### Fixed

- In the filter mode and shell mode, when you don't have any input, `backspace` now means to return to the normal mode.
- Also, when you press `Esc` during the filter mode, the cursor position is now restored.

## v1.1.0 (2022-08-08)

### Changed

- Use `chafa` instead of `libsixel` & `viuer` to preview image files. This greatly improves the performance and code maintainability, and as a consequence, the number of dependencies is reduced (137 -> 53).
- With `chafa`, the hi-res image preview is supported in kitty or terminals that support sixel.
- Files larger than 1GB are no longer previewed in order to improve the performance.
- Remove profile.release to support older version of Rust.
- Huge refactoring (layout.rs created).

### Added

- `content-inspector` to exclude binary files to be previewed.

## v1.0.1 (2022-07-28)

### Fixed

- Add thread sleep time after state.open_file(). This is necessary because, with tiling window managers, the window resizing is sometimes slow and felix reloads the layout so quickly that the display may become broken. By the sleep (50ms for now and I think it's not easy to recognize this sleep), this will be avoided.

## v1.0.0 (2022-07-04)

### Fixed

- Cursor move when using G in select mode.
- Remove unnecessary loops in `dd`, `ZZ`.

## v0.9.8 (2022-06-30)

### Fixed

- Enable resizing window.

### Added

- Print message about the config file when created.

## v0.9.7 (2022-06-16)

### Fixed

- Move cursor and put properly in an empty directory.

## v0.9.6 (2022-06-16)

### Fixed

- Formatting of the contents tree.

### Changed

- Input right before the pattern matching.

## v0.9.5 (2022-06-15)

### Changed

- `z <keyword>` works without prefix `:` (jump to a directory that matches the keyword).
- Refactor: Use redraw() and reload() instead of multiple methods.
- Better config: If config file not found, now you can interactively set the default command.
- In the filter mode, press `h` or `Left` to return to the normal mode and reload the current directory's contents.

## v0.9.4 (2022-06-08)

### Added

- Hi-res image preview is enabled if i) your terminal supports sixel, and ii) you've preinstalled `libsixel`. If not, images are printed by blocks as before.

### Changed

- Some refactoring.

## v0.9.3 (2022-05-25)

### Added

- `-l` option creates a log file in `$XDG_CONFIG_HOME/felix/log`. Information such as put, delete, rename, emptying the trash directory, etc. will be recorded.
- Add message when there are no operations left to undo/redo.

### Changed

- Simplify the info line(below the current directory information).
- Make rename information more understandable("New name: " instead of "&#8658;").
- Use struct `Operation` to express the manipulation within the app (put/delete/rename) and implement some methods.
- Refactor overall.

### Fixed

- Fix put/delete process information.

## v0.9.2 (2022-05-18)

### Added

- [Experimental] Image preview on the right half of the terminal (press `v`). This feature uses `viuer`, and high resolution preview, which can be used in kitty or terminals that support sixel, is disabled due to the clearance issues.
- crate `viuer` and `image` to preview the image.

### Fixed

- Fix text preview bug around new line that occurs when it has tab character.
- file_ext in ItemInfo is now always lowercased to speed up matching with the extension map.
- Disable renaming non-ascii items: Wide characters such as CJK or characters that do not match our intuition caused panic, so before editing, item name is now checked if it contains only ascii characters.

### Changed

- Version check option now uses -v | --version, instead of -c | --check.
- Refactor: Remove magic number and use variable with proper name in the filter and shell mode.
- Restore debug print, which works in debug mode(RUST_LOG has some value).
- Use `simplelog` instead of `env_logger` to create the log file.

## v0.9.1 (2022-05-11)

### Fixed

- Fix bug that after `:h`, cursor move can cause unexpected panic.

### Changed

- Text preview always wrapped (yet static).

## v0.9.0 (2022-05-10)

### Added

- CHANGELOG.md
- New command: `v` to toggle whether to show i) part of the content for text file (no wrapping and static), or ii) contents tree for directory. Note that this preview feature may not work effectively with small terminal.
- trying to make felix user guide (just to show how to use each commands) by mdbook

### Changed

- felix now works with smaller terminal size (4 rows and 4 columns is the minimum). If column is fewer than 28, modified time is not displayed.
- Huge refactoring overall.
  - use `struct colors` for `state.layout.colors`
  - `is_hidden` moved to `ItemInfo`'s field, make it easier to toggle show/hidden items
  - in `open_files` method, use `ItemInfo`'s existing field to get extension
  - `Iteminfo.ext` to `Option<String>`
  - split `move_cursor` method to multiple methods
- Inverted color on the last row (to show distinctively)

### Fixed

- Show help text correctly in small window size (scrollable with `j` | `k` | `Up` | `Down`)
- 'P' to print manipulation lists (put/delete/rename) is changed to work only when RUST_LOG has a value.

## v0.8.1 (2022-05-04)

### Fixed

- undo/redo order when new operations occurs. Now manipulation list will be "branched", which means undone operations will be discarded when new manipulation is pushed, so that redo cannot lead to an error.

## v0.8.0 (2022-05-03)

### Added

- New commands: 'u' to undo and 'Ctrl + r' to redo. Targets of these new commands are put/delete/rename.

### Fixed

- Clarified the type of error during initial setup (now explicitly use panic).
- Added minimum row size.
- Better cursor move when terminal size is extremely small (row size < 8).

## v0.7.0 (2022-04-26)

### Added

- Terminal size changes are now automatically detected and the layout is fixed.
- felix -c shows the current version and checks if that is up to date.

![size_change.gif](https://github.com/kyoheiu/felix/blob/main/screenshots/size_change.gif)

## v0.6.1 (2022-04-15)

### Added

- New configuration: You can now use the full width of terminal by setting `use_full_width` to true (false by default). I hope this wil lead to a better user experience. _For those who use <=0.6.0, felix can work without replacing config.toml because `use_full_width` is an option._

## v0.6.0 (2022-04-13)

### Added

- ':z <keyword>' lets you jump to a directory that matches the keyword. (zoxide required)
- :cd | :z => Go to home directory.

### Fixed

- Fix bug when reading .git/HEAD to show branch name

## v0.5.2 (2022-04-10)

### Added

- New option for config: Now you can set the max length of the item name to be displayed (if the terminal size is not enough, it will be automatically adjusted). It's optional, so you can use your config file in < v.0.5.1 as is. See `config.toml` for details.

## v0.5.1 (2022-03-30)

### Fixed

- Fix message when deleting multiple items
- Remove duplicated call for env variable

## v0.5.0 (2022-03-29)

### Added

- Follow symlink if it leads to a directory
- Implement memoization of move when going to symlink dir
- Print help by `fx -h` | `fx --help`

### Fixed

- Open files whether its extension in lowercase or uppercase

## v0.4.3 (2022-03-24)

### Fixed

- cursor movement when deleting multiple items

## v0.4.2 (2022-03-07)

### Fixed

- better indicator when copying/deleting

## v0.4.1 (2022-03-04)

### Added

- show total time to delete/copy items
- show process to delete/copy items

## v0.4.0 (2022-02-01)

### Added

- enable to show/hide hidden items
- felix keeps the state of show_hidden(whether to show hidden items) and sort_by(by name or by modified time): The change remains after exit.

## v0.3.2 (2022-01-14)

### Fixed

- Restore cursor state after exit

## v0.3.1 (2022-01-13)

### Fixed

- cursor movement when going to parent directory
- cursor memoization using PathBuf instead of String

## v0.3.0 (2022-01-07)

### Added

- Show item information on the last line (index, file extension, file size)
- Add memoization of cursor position in previous directory
- Adjust cursor movement when going to different child directory

### Fixed

- display of item when selecting
- cursor movement after filter mode

## v0.2.13 (2021-12-29)

### Changed

- edition 2018 -> 2021

### Fixed

- the cursor adjustment when moving to the parent directory
- space between file name and modified time

## v0.2.12 (2021-12-10)

### Fixed

- Enable to delete broken symlink

## v0.2.11 (2021-11-27)

### Changed

- now felix can work in small terminals (21 columns or more is sufficient)

## v0.2.10 (2021-11-24)

### Removed

- Remove Ctrl + c for copying item name to the clipboard (in order to reduce build dependency)

### Changed

- Change color of selected items to make them more visible

## v0.2.9 (2021-11-18)

### Added

- show current branch if .git exists
- add message about processing when delete and put

### Fixed

- rename of multiple items when put now works correctly
- show error message when delete faiils
- cursor move when empty the trash dir

## v0.2.8 (2021-11-14)

### Fixed

- cursor move when deleting the last item

## v0.2.7 (2021-11-14)

### Fixed

- error handling
  - in shell mode (when command does not work)
  - when cannot open file

## v0.2.6 (2021-11-12)

### Changed

- Now you can install felix without +nightly.

## v0.2.4 (2021-11-11)

### Fixed

- README.md (executable name)

## v0.2.3 (2021-11-10)

### Changed

- Rename app! (fm -> fx)
- `:h` to show help
- `:empty` to empty the trash directory

### Added

- Show info when yank / delete / put / copy item name

## v0.2.2 (2021-11-08)

### Added

- `:e` to reload the current directory

## v0.2.1 (2021-11-07)

### Added

- fast copy of massive files

## v0.2.0 (2021-11-05)

### Added

- select mode, where you can delete and yank muliple items

### Changed

- key changes (g -> gg, D -> dd)

### Removed

- remove m (because you can mkdir in shell mode)
- remove warning when delete

## v0.1.5 (2021-10-28)

### Added

- shell mode.
- Copy file name to clipboard.

### Fixed

- Fix bug when delete item in filtered list.

## v0.1.4 (2021-10-26)

### Added

- Command argument enabled: fm <directory path> shows items in the path (both relative and absolute can be used).
- `H` to show help.

## v0.1.3 (2021-10-22)

### Added

- `t` to change the sort order (file name <-> modified time)

## v0.1.2 (2021-10-20)

### Added

- Add `FileType::Symlink` to change color of symlink item (color configurable in `config.toml`).
