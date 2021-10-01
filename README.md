# fm

A tiny file manager that enables you to open file fast.

## Installation

```
git clone https://github.com/kyoheiu/fm.git
cd fm
cargo install
```

## Usage

| Key                   | Explanation                                                                       |
| --------------------- | --------------------------------------------------------------------------------- |
| j / Key Up            | Go up. If lists exceeds max-row, lists "scrolls" before the top of the list.      |
| k / Key Down          | Go down. If lists exceeds max-row, lists "scrolls" before the bottom of the list. |
| h / Key Left          | Go to parent directory if exists.                                                 |
| l / Key Right / Enter | Open file(exec in any way fo now) or change directory(change lists as if `cd`).   |
