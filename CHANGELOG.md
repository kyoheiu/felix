# Changelog

## Notes

## Unreleased

## v2.11.1 (2023-12-10)

### Fixed

- Allow SHIFT key to enter characters after `i`, `I`, `c`, `/`, `:` and `z`.

## v2.11.0 (2023-12-09)

### Added

- `<C-h>` for Backspace functionality after `i`, `I`, `c`, `/`, `:` and `z`.

## v2.10.2 (2023-11-26)

### Fixed
- Added a filter to every user input to reject `Keyup` events. This is required on the windows platform.

## v2.10.1 (2023-11-02)

### Fixed
- Convert tab to 4 spaces when using bat to preview text files.

## v2.10.0 (2023-11-01)

### Added
- `bat` integration: If `bat` installed, felix automatically adds syntax highlighting to the text preview.
  - Add `has_bat` field to `State`.
  - Add `FxError::InvalidPath` to handle invalid unicode in file path.

## v2.9.0 (2023-10-22)

### Added
- Change color of untracked/changed files or directories containing such files. Default color is Red(1). You can change it in the config file.
  - Add `git2`.
  
### Fixed
- Explicitly ignore the key release events for Windows.

## v2.8.1 (2023-08-25)

### Fixed
- Fix help text.

## v2.8.0 (2023-08-25)

### Added
- `i{file name}<CR>` to create new file, and `I{dir name}<CR>` to create new directory.
- If zoxide is installed, whenever changing directory inside felix, `zoxide add` will be executed to add the directory or increment its rank in the zoxide database.
  - For this, State now has a new field `has_zoxide`, which is checked at startup.

### Changed
- config's `color` is now optional: By this, all config fields are optional.
  - Remove warning message when you launch felix without the config file.
- When opening file by default editor is failed, felix displays more accurate warning: `$EDITOR may not be set, or config file may be invalid.`.

### Removed
- Remove `syntect` and syntax highlighting in the preview area. This will improve build and start-up times, and resolve the handling of wide chars such as CJK.

## v2.7.0 (2023-08-05)

### Changed

- Minimal supported rust version is now 1.67.1
- Upgrade dependencies.
- Update syntect version to v5.1.0. This fixes the handling of multibyte chars in the preview area.
- Allow file name `config.yml` in addition to `config.yaml` for the configuration.

## v2.6.0 (2023-07-22)

### Added

- Allow `<C-r>` in command line: Paste item name(s) in register. e.g. `<C-r>"` pastes item name in unnamed register.
- Allow wild card in command line: e.g. `:zip test *.md` works now.
- Ability to `cd` {absolute/relative path}.
- Ability to jump backward / forward (`<C-o>`, `<C-i>` respectively)

## v2.5.0 (2023-07-13)

### Added

- Ability to exit to LWD (last working directory): See Integrations for details.

## v2.4.1 (2023-06-21)

### Changed

- Show status bar and registers even if current directory does not contain any item.

## v2.4.0 (2023-06-14)

### Added

- Add registers (unnamed, zero, numbered, named): Now you can view registers by `:reg`, and add items to registers by usual vim command (prefixed by `"`). See the key manual for more details.
- Refactor unpacking command: `e` unpacks / decompresses gz(Gzip), tar.gz, xz(lzma), tar.xz, zst(Zstandard), tar.zst, tar, and zip file format and formats based on it.

### Removed

- `:z` - Use `z` instead.

## v2.3.0 (2023-05-26)

### Changed

- Add extra config file path for macOS: `/Users/$USER/.config/felix/config.yaml` will be read after `$HOME/Library/Application Support/felix/config.yaml`.
- If config file is not found, or found one is broken, felix launches with the default configuration, without creating new one.
- If the current directory is read-only, `dd`, `Vd` and `p` is disabled in the first place.
- Bump up MSRV to 1.65.

### Added

- Add `is_ro` field to `State`.

### Removed

- NetBSD install test. It often failed while setting up the VM, which had nothing with felix.

## v2.2.8 (2023-05-19)

### Fixed

- Kitty-specific: Enable scrolling of the preview text by redrawing the screen only when needed (this also improves the perfomance entirely).

## v2.2.7 (2023-05-05)

### Added

- Print `[RO]` on the headline if user does not have the write permission on the directory. This is available only on UNIX for now.

## v2.2.6 (2023-04-24)

### Removed

- Remove duplicated `-v | --version` option. This is because i) Since some users
  do not have `cargo` installed, fetching latest version via `cargo` doesn't
  work for many, and ii) `-h | --help` option can already show the current
  version.

## v2.2.5 (2023-02-12)

### Added

- Allow renaming even when item name contains non-ascii chars (i.e. wide chars).
- Key command with arguments is now supported: For example,
  ```
  exec:
  'feh -.':
    [jpg, jpeg, png, gif, svg, hdr]
  ```
  this configuration enables you to execute `feh -. <item path>` by
  `Enter | l | Right`, or `o`.
- Check for out-of-boundary of the cursor at the top of loop.

### Fixed

- Display when using in kitty: Correctly show the cursor and preview.

## v2.2.4 (2023-02-01)

### Fixed

- Disable remove_and_yank in the trash dir.
- Clear selection in the select mode if something fails.
- Cursor move after deleting multiple items in select mode.

## v2.2.3 (2023-01-20)

### Fixed

- Wide chars handling: Using unicode_width, now felix can properly split file
  name or previewed texts.
- Preview space height: When horizontally split, image preview could break the
  layout. Fixed this by adjusting the height.

### Added

- `chafa`'s minimal supported version: >= v1.10.0
- Add pacman installation.

## v2.2.2 (2022-12-19)

### Fixed

- Disable commands with Ctrl or other modifiers unless explicitly implemented.
  (For now, `Ctrl + r` to redo, `Alt + j` and `Alt + k` to scroll the preview
  text are implemented) This avoids for example the situation where `Ctrl + d`
  unintentionally deletes an item.
- Add `create_dir_all` to `config_dir` and `data_local_dir` to avoid error.
- Check if the argument is directory.

## v2.2.1 (2022-12-15)

### Fixed

- Fix the compilation on NetBSD.

## v2.2.0 (2022-12-12)

### Changed

- **IMPORTANT**: Trash, log directory, and session file path changed.
  - from v2.2.0, felix will use `dirs::data_local_dir()` to store the deleted
    items and log files, instead of `dirs::config_dir()`.
  - Due to this change, the path for linux will be
    `$XDG_DATA_HOME/felix/{Trash, log, .session}`, in most case
    `/home/user/.local/share/felix/{Trash, log, .session}`. For Windows
    `{FOLDERID_LocalAppData}\felix\{Trash, log, .session}`, typically
    `C:\Users\user\AppData\Local\felix\{Trash, log, .session}`. No change for
    macOS users.
  - Note that config file path does not change on any OS!
  - Please don't forget deleting old trash directory and log files if you don't
    want them anymore.
- Refactoring overall.

### Added

- `:trash` to go to the trash directory.

### Fixed

- Support NetBSD to open file in a new window.
- Properly remove broken symlink in Windows as well. Also, when
  deleting/puttiing a directory, broken symlink(s) in it won't cause any error
  and will be removed from the file system after deleting/putting.

## v2.1.1 (2022-12-02)

### Fixed

- You can now open a file in a new window on Wayland environment too.
- Proper handling of wide characters: Even if an item or a info message includes
  some wide charatcters such as CJK, the layout won't break anymore.
- Fix cursor color after printing the text preview.

### Changed

- Some refactoring around text-printing in the preview space.
- When you change the sort key, felix refresh the list more efficiently than
  ever by avoiding to read all the items.
- Item order(Name): Case-insensitive instead of sensitive.

## v2.1.0 (2022-11-19)

### Added

- Feature to unpack archive/compressed file to the current directory. Supported
  types: `tar.gz`(Gzip), `tar.xz`(lzma), `tar.zst`(Zstandard & tar),
  `zst`(Zstandard), `tar`, zip file format and formats based on it(`zip`,
  `docx`, ...). To unpack, press `e` on the item.
  - The number of dependencies bumps up to around 150 due to this.

### Fixed

- Bug: In the select mode, the selected item was unintentionally canceled when
  going up/down.
- Delete pointer properly when removing items.
- Instead of panic, return error when `config_dir()` fails.

### Changed

- Image file detection: Use magic bytes instead of checking the extension. This
  will enable to detect image more precisely.

## v2.0.1 (2022-11-12)

### Fixed

- Fixed the bug in making config at the launch.
- Fixed the config file path on macOS.

## v2.0.0 (2022-11-11)

### Changed

- Migrated to yaml from toml: New config file will be created at the first
  launch (In this process you should enter the default command name or choose to
  use \$EDITOR). No more need to keep `config.toml`.
- Add the fallback when config file cannot be read: In such a case, you can use
  the default Config.
- HUGE refactoring overall.

### Added

- Horizontal split, in addition to the vertical split. To toggle, press `s`.
- Syntax highlighting (if possible) in previewed texts. To turn on, state
  `syntax_hightlight = true` in `config.toml`. you can also choose your theme,
  either from the default theme set or your favorite .tmtheme.
- Enable scrolling in the preview space. `Alt + j / Up` goes down, `Alt + k`
  goes up. Experimental and may have some bugs, and with a big text file the
  perf issue may arise.
- Search by keyword. Similar to the filter mode, but this feature do not
  manipulate the item list, just let users jump to the item that matches the
  keyword, just like Vim's `/`. `n` and `N` after `/` also works.
- Show permissions on the footer (in unix only).

### Fixed

- Use `exists()` instead of `File::open()` to check whether the item path is
  valid when moving between directories. This allows Windows users to use this
  app at least with the basic commands.
- Avoid `unwrap()` / `panic!` as possible and return the proper error.

### Removed

- Removed the filter mode, which is replaced by the keyword search.
- Removed debug print in `make_config_if_not_exists`
- Removed `use_full_width` and `item_name_length` in `config.toml`. Will always
  use full width of the terminal.

## v1.3.2 (2022-10-23)

### Added

- Add `std::panic::catch_unwind` to manually restore after a panic rewind. This
  allows the cursor to be restored and the screen cleared when this app panics.

### Fixed

- Fixed: Similar to v1.3.1, attempting to preview a symbolic link to a
  nonexistent file caused a panic. Now the preview shows `(file not readable)`
  for such a link.

## v1.3.1 (2022-10-21)

### Fixed

- Attempting to preview a symbolic link to a directory caused a panic. It has
  been fixed and the preview will now show the contents of the linked directory.

## v1.3.0 (2022-10-18)

### Changed

- Huge refactoring: Migrated to crossterm from termion due to the
  maintainability and future-support for Windows. New module `term.rs` contains
  (almost) all of the terminal API, so that other modules will not get effected
  by the future backend change.
  - Alongside, some changes are added to show the file path properly in Windows.
  - With crossterm, opening a file in e.g. Vim, it feels as if this app
    "freezes". This behavior is not what I want, so from v1.3.0,
    `open_file_in_new_window` can work only if \[exec\] is set in config file,
    and the extension of the item matches the key.
- `default` key in the config file become `Option`, so that users can select
  `$EDITOR` without explicitly setting it up. The initial process of asking
  users to select the default command has also been fixed accordingly.

### Fixed

- After zoxide jump, turn off the filter mode.
- Many typos fixed.

### Added

- New error: `OpenNewWindow`
- New GitHub actions: Add windows-install

## v1.2.0 (2022-10-01)

### Changed

- Huge refactoring: Instead of `thiserror`, use custom error type to make it
  easier to handle.
- Bump up chrono version to 0.4.22, clarifing the feature to use.
- Avoid extra heap allocation by using write! instead of push_str/format!.
- Copied item will be renamed with the suffix "\_{count}" such as "test_1.txt",
  instead of "test_copied.txt".

### Fixed

- Choose `None` for directory extension.

## v1.1.2 (2022-08-29)

### Fixed

- Use full width of the terminal when `use_full_width` in config.toml is not
  set.
- Use `cursor::Goto` instead of `cursor::Left` to fix the layout in macOS
  Terminal.app.
- Refactor functions around the layout.

## v1.1.1 (2022-08-11)

### Fixed

- In the filter mode and shell mode, when you don't have any input, `backspace`
  now means to return to the normal mode.
- Also, when you press `Esc` during the filter mode, the cursor position is now
  restored.

## v1.1.0 (2022-08-08)

### Changed

- Use `chafa` instead of `libsixel` & `viuer` to preview image files. This
  greatly improves the performance and code maintainability, and as a
  consequence, the number of dependencies is reduced (137 -> 53).
- With `chafa`, the hi-res image preview is supported in kitty or terminals that
  support sixel.
- Files larger than 1GB are no longer previewed in order to improve the
  performance.
- Remove profile.release to support older version of Rust.
- Huge refactoring (layout.rs created).

### Added

- `content-inspector` to exclude binary files to be previewed.

## v1.0.1 (2022-07-28)

### Fixed

- Add thread sleep time after state.open_file(). This is necessary because, with
  tiling window managers, the window resizing is sometimes slow and felix
  reloads the layout so quickly that the display may become broken. By the sleep
  (50ms for now and I think it's not easy to recognize this sleep), this will be
  avoided.

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

- `z <keyword>` works without prefix `:` (jump to a directory that matches the
  keyword).
- Refactor: Use redraw() and reload() instead of multiple methods.
- Better config: If config file not found, now you can interactively set the
  default command.
- In the filter mode, press `h` or `Left` to return to the normal mode and
  reload the current directory's contents.

## v0.9.4 (2022-06-08)

### Added

- Hi-res image preview is enabled if i) your terminal supports sixel, and ii)
  you've preinstalled `libsixel`. If not, images are printed by blocks as
  before.

### Changed

- Some refactoring.

## v0.9.3 (2022-05-25)

### Added

- `-l` option creates a log file in `$XDG_CONFIG_HOME/felix/log`. Information
  such as put, delete, rename, emptying the trash directory, etc. will be
  recorded.
- Add message when there are no operations left to undo/redo.

### Changed

- Simplify the info line(below the current directory information).
- Make rename information more understandable("New name: " instead of
  "&#8658;").
- Use struct `Operation` to express the manipulation within the app
  (put/delete/rename) and implement some methods.
- Refactor overall.

### Fixed

- Fix put/delete process information.

## v0.9.2 (2022-05-18)

### Added

- [Experimental] Image preview on the right half of the terminal (press `v`).
  This feature uses `viuer`, and high resolution preview, which can be used in
  kitty or terminals that support sixel, is disabled due to the clearance
  issues.
- create `viuer` and `image` to preview the image.

### Fixed

- Fix text preview bug around new line that occurs when it has tab character.
- file_ext in ItemInfo is now always lowercased to speed up matching with the
  extension map.
- Disable renaming non-ascii items: Wide characters such as CJK or characters
  that do not match our intuition caused panic, so before editing, item name is
  now checked if it contains only ascii characters.

### Changed

- Version check option now uses -v | --version, instead of -c | --check.
- Refactor: Remove magic number and use variable with proper name in the filter
  and shell mode.
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
- New command: `v` to toggle whether to show i) part of the content for text
  file (no wrapping and static), or ii) contents tree for directory. Note that
  this preview feature may not work effectively with small terminal.
- trying to make felix user guide (just to show how to use each commands) by
  mdbook

### Changed

- felix now works with smaller terminal size (4 rows and 4 columns is the
  minimum). If column is fewer than 28, modified time is not displayed.
- Huge refactoring overall.
  - use `struct colors` for `state.layout.colors`
  - `is_hidden` moved to `ItemInfo`'s field, make it easier to toggle
    show/hidden items
  - in `open_files` method, use `ItemInfo`'s existing field to get extension
  - `Iteminfo.ext` to `Option<String>`
  - split `move_cursor` method to multiple methods
- Inverted color on the last row (to show distinctively)

### Fixed

- Show help text correctly in small window size (scrollable with `j` | `k` |
  `Up` | `Down`)
- 'P' to print manipulation lists (put/delete/rename) is changed to work only
  when RUST_LOG has a value.

## v0.8.1 (2022-05-04)

### Fixed

- undo/redo order when new operations occurs. Now manipulation list will be
  "branched", which means undone operations will be discarded when new
  manipulation is pushed, so that redo cannot lead to an error.

## v0.8.0 (2022-05-03)

### Added

- New commands: 'u' to undo and 'Ctrl + r' to redo. Targets of these new
  commands are put/delete/rename.

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

- New configuration: You can now use the full width of terminal by setting
  `use_full_width` to true (false by default). I hope this will lead to a better
  user experience. _For those who use <=0.6.0, felix can work without replacing
  config.toml because `use_full_width` is an option._

## v0.6.0 (2022-04-13)

### Added

- ':z <keyword>' lets you jump to a directory that matches the keyword. (zoxide
  required)
- :cd | :z => Go to home directory.

### Fixed

- Fix bug when reading .git/HEAD to show branch name

## v0.5.2 (2022-04-10)

### Added

- New option for config: Now you can set the max length of the item name to be
  displayed (if the terminal size is not enough, it will be automatically
  adjusted). It's optional, so you can use your config file in < v.0.5.1 as is.
  See `config.toml` for details.

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
- felix keeps the state of show_hidden(whether to show hidden items) and
  sort_by(by name or by modified time): The change remains after exit.

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

- Remove Ctrl + c for copying item name to the clipboard (in order to reduce
  build dependency)

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

- select mode, where you can delete and yank multiple items

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

- Command argument enabled: fm <directory path> shows items in the path (both
  relative and absolute can be used).
- `H` to show help.

## v0.1.3 (2021-10-22)

### Added

- `t` to change the sort order (file name <-> modified time)

## v0.1.2 (2021-10-20)

### Added

- Add `FileType::Symlink` to change color of symlink item (color configurable in
  `config.toml`).
