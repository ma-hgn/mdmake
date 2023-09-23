
# 📜 mdmake

> Generate Static Websites from Directories of Markdown Files.

1. [Features](#features)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Usage](#usage)

## Features

- [x] Convert single files from Markdown to HTML
  - [x] [Commonmark specification](https://spec.commonmark.org/0.30/) compliance
- [x] Watch Mode that detects File changes and recompiles those Markdown files
- [x] Capability to add custom HTML-Headers and -Footers
- [x] Linking file against a CSS style sheet
- [x] Convert nested directory structure of Markdown files to interlinked HTML-Pages
  - [x] Updating relative links to other Markdown files to point to their respective HTML output

  - [x] Linking all files against a single CSS style sheet

## Installation

> To install mdmake from source, first [set up a Rust tool chain including Cargo](https://rustup.rs/).

Simply clone, then build and install the project!

```sh
git clone https://github.com/markichnich/mdmake --depth=1 &&
cargo install --path mdmake
```
or alternatively

```sh
git clone https://github.com/markichnich/mdmake --depth=1 &&
cd mdmake && cargo build --release && sudo cp target/release/mdmake /usr/bin
```


## Quick Start

First, set up a directory hierarchy of Markdown files within a folder.
Unless specified, `mdmake` tries to use `./src` as the source folder.

```text
src
├── index.md
├── food
│   ├── fried_rice.md
│   └── spaghetti_carbonara.md
└── gardening
    └── cacti.md
```
Then execute `mdmake`.
When not specifying an output folder. `./out` will be used.

```text
├── index.md
├── food
│   ├── fried_rice.md
│   └── spaghetti_carbonara.md
└── gardening
    └── cacti.md
out
├── index.html
├── food
│   ├── fried_rice.html
│   └── spaghetti_carbonara.html
└── gardening
    └── cacti.html
```

> Additionally, `mdmake` will look for `style.css`, `header.html` and `footer.html` in the input directory.
- If a style sheet is found, it will be copied to the output directory and all HTML files will be linked against it automatically.
- If a header/footer if found, it will be prepended/appended to the HTML bodies of the output files.


You can also [manually specify](#usage) the input- and output directories, as well as style sheet-, header- and footer files.


## Usage

```
Generate static websites from a directory of markdown files.

Usage: mdmake [OPTIONS] [COMMAND]

Commands:
  watch, -w  Watch for changes and automatically recompile.
  help       Print this message or the help of the given subcommand(s)

Options:
  -i, --input [<DIRECTORY>]   The project root of the markdown files.
  -o, --output [<DIRECTORY>]  The destination for the compiled webpage.
      --style [<FILE>]        The CSS-stylesheet to use for all html files.
      --header [<FILE>]       The HTML-header to prepend to all HTML-Bodies.
      --footer [<FILE>]       The HTML-footer to append to all HTML-Bodies.
  -h, --help                  Print help
  -V, --version               Print version
```

## Credits

- Inspiration from [`ssg` by Roman Zolotarev](https://romanzolotarev.com/ssg.html)
