# Bitmapped Image Font Sheet Viewer

## Introduction
The program `fontview` is a utility for viewing bitmapped font atlas files. Its purpose is
to give the font author and idea of what the font sheet would look like inside a computer graphics application.
Its particular use case is displaying fonts to be used in game programming. This repository is the source tree
for `fontview`.

## Dependencies
This program depends on `Rust 2018 Editon` to compile it.

## Installation
To install `fontview`, fork the repository and enter
```bash
cargo install
```
to install it.

## Usage
The program `fontview` is used from a shell environment. To use it, enter
```bash
fontview --input /path/to/font.bmfa
``` 
to view what the bitmap font looks like.
